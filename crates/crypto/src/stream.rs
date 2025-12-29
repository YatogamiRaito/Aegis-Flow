use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use bytes::{Buf, BufMut, BytesMut};
use std::cmp;
use std::io;
use std::pin::Pin;
use std::task::ready;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const U32_SIZE: usize = 4;
const NONCE_SIZE: usize = 12; // 96-bit nonce for AES-GCM
const FRAME_OVERHEAD: usize = U32_SIZE + NONCE_SIZE + 16;
const MAX_FRAME_SIZE: usize = 64 * 1024; // 64KB max payload

pub struct EncryptedStream<S> {
    stream: S,
    encryptor: Aes256Gcm,
    decryptor: Aes256Gcm,

    // Read state
    read_buffer: BytesMut,
    decrypted_buffer: BytesMut,

    // Write state
    write_buffer: BytesMut,
}

impl<S> EncryptedStream<S> {
    pub fn new(stream: S, key: &[u8]) -> Self {
        // Use the same key for both directions (symmetric)
        // In TLS, client/server keys differ. Here, we assume a single shared secret for simplicity of MVP.
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);

        Self {
            stream,
            encryptor: cipher.clone(),
            decryptor: cipher,
            read_buffer: BytesMut::with_capacity(MAX_FRAME_SIZE * 2),
            decrypted_buffer: BytesMut::with_capacity(MAX_FRAME_SIZE * 2),
            write_buffer: BytesMut::with_capacity(MAX_FRAME_SIZE * 2),
        }
    }
}

/// Helper to read from AsyncRead into BytesMut
fn poll_read_into(
    io: Pin<&mut impl AsyncRead>,
    cx: &mut Context<'_>,
    buf: &mut BytesMut,
) -> Poll<io::Result<usize>> {
    let dst = buf.chunk_mut();
    // SAFETY: We are creating a ReadBuf from uninitialized memory which is safe
    // because ReadBuf ensures we don't read uninitialized memory, passing it to
    // poll_read which fills it. advance_mut then marks it as initialized.
    let dst = unsafe { &mut *(dst as *mut _ as *mut [std::mem::MaybeUninit<u8>]) };
    let mut read_buf = ReadBuf::uninit(dst);

    ready!(io.poll_read(cx, &mut read_buf))?;

    let n = read_buf.filled().len();
    if n > 0 {
        unsafe { buf.advance_mut(n) };
    }
    Poll::Ready(Ok(n))
}

impl<S: AsyncRead + Unpin> AsyncRead for EncryptedStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let me = self.get_mut();

        loop {
            // 1. If we have decrypted data, serve it
            if !me.decrypted_buffer.is_empty() {
                let len = cmp::min(buf.remaining(), me.decrypted_buffer.len());
                buf.put_slice(&me.decrypted_buffer[..len]);
                me.decrypted_buffer.advance(len);
                return Poll::Ready(Ok(()));
            }

            // 2. Try to read frame length (4 bytes)
            if me.read_buffer.len() < U32_SIZE {
                if me.read_buffer.capacity() < U32_SIZE {
                    me.read_buffer.reserve(U32_SIZE);
                }

                let n = ready!(poll_read_into(
                    Pin::new(&mut me.stream),
                    cx,
                    &mut me.read_buffer
                ))?;
                if n == 0 {
                    return if me.read_buffer.is_empty() {
                        Poll::Ready(Ok(())) // EOF
                    } else {
                        Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Partial frame length",
                        )))
                    };
                }
                // println!("EncryptedStream: Read {} bytes, total buffer: {}", n, me.read_buffer.len());
                continue;
            }

            // 3. Parse length
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&me.read_buffer[..4]);
            let frame_len = u32::from_be_bytes(len_bytes) as usize;

            // println!("EncryptedStream: Parsed frame_len: {}", frame_len);

            if frame_len < NONCE_SIZE + 16 {
                // println!("EncryptedStream: Frame too short: {}", frame_len);
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Frame too short",
                )));
            }
            if frame_len > MAX_FRAME_SIZE + FRAME_OVERHEAD {
                // println!("EncryptedStream: Frame too large: {}", frame_len);
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Frame too large",
                )));
            }

            // 4. Try to read full frame
            let total_required = U32_SIZE + frame_len;
            if me.read_buffer.len() < total_required {
                if me.read_buffer.capacity() < total_required {
                    me.read_buffer
                        .reserve(total_required - me.read_buffer.len());
                }

                let n = ready!(poll_read_into(
                    Pin::new(&mut me.stream),
                    cx,
                    &mut me.read_buffer
                ))?;
                if n == 0 {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Incomplete frame",
                    )));
                }
                // println!("EncryptedStream: Read more bytes for frame. Buffer: {}/{}", me.read_buffer.len(), total_required);
                continue;
            }

            // 5. Decrypt frame
            // Consume length header
            me.read_buffer.advance(U32_SIZE);
            // Extract nonce and ciphertext
            let nonce = Nonce::from_slice(&me.read_buffer[..NONCE_SIZE]).to_owned(); // copy nonce
            // Extract ciphertext (remainder of frame_len) including tag
            let payload = &me.read_buffer[NONCE_SIZE..frame_len];

            match me.decryptor.decrypt(&nonce, payload) {
                Ok(plaintext) => {
                    // println!("EncryptedStream: Decrypted {} bytes", plaintext.len());
                    // print hex of first 8 bytes if available
                    // if plaintext.len() >= 8 {
                    //      println!("EncryptedStream: First 8 bytes: {:02X?}", &plaintext[..8]);
                    // }
                    me.decrypted_buffer.extend_from_slice(&plaintext);
                    me.read_buffer.advance(frame_len);
                    // Loop continues to serve from decrypted_buffer
                }
                Err(_) => {
                    // println!("EncryptedStream: Decryption failed!");
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Decryption failed",
                    )));
                }
            }
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for EncryptedStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let me = self.get_mut();

        // 1. Flush existing write buffer first
        while !me.write_buffer.is_empty() {
            let n = ready!(Pin::new(&mut me.stream).poll_write(cx, &me.write_buffer))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "Failed to write encrypted data",
                )));
            }
            me.write_buffer.advance(n);
        }

        // 2. Encrypt input buffer
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        // println!("EncryptedStream: Encrypting {} bytes", buf.len());

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        match me.encryptor.encrypt(&nonce, buf) {
            Ok(ciphertext_tag) => {
                let frame_len = NONCE_SIZE + ciphertext_tag.len();
                // println!("EncryptedStream: Writing frame len: {} (overhead: {})", frame_len, FRAME_OVERHEAD);

                // Write Header: Length(4) + Nonce(12) + CiphertextTag(...)
                me.write_buffer.put_u32(frame_len as u32);
                me.write_buffer.put_slice(&nonce);
                me.write_buffer.put_slice(&ciphertext_tag);

                // 3. Try to write immediately (opt)
                // We fake success here to batch, relying on next call or flush to send data.
                // This is compliant with AsyncWrite, provided we do eventually write it.
                Poll::Ready(Ok(buf.len()))
            }
            Err(_) => Poll::Ready(Err(io::Error::other("Encryption failed"))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();

        // Flush write buffer
        while !me.write_buffer.is_empty() {
            let n = ready!(Pin::new(&mut me.stream).poll_write(cx, &me.write_buffer))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "Failed to write encrypted data",
                )));
            }
            me.write_buffer.advance(n);
        }

        // Flush inner stream
        Pin::new(&mut me.stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();

        // Ensure everything is written
        match Pin::new(&mut *me).poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        }

        Pin::new(&mut me.stream).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_stream_roundtrip() {
        let key = [0x42u8; 32];
        let payload = b"Hello, Aegis-Flow Secure Stream!";

        // Use an in-memory buffer as the "network"
        let mut network_buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut network_buffer);

        // 1. Write encrypted data to buffer
        {
            let mut writer = EncryptedStream::new(&mut cursor, &key);
            writer.write_all(payload).await.unwrap();
            writer.flush().await.unwrap();
        }

        // 2. Read back from buffer
        let mut read_cursor = std::io::Cursor::new(&network_buffer);
        let mut reader = EncryptedStream::new(&mut read_cursor, &key);

        let mut decrypted = vec![0u8; payload.len()];
        reader.read_exact(&mut decrypted).await.unwrap();

        assert_eq!(&decrypted, payload);
    }

    #[tokio::test]
    async fn test_large_payload_chunking() {
        let key = [0x11u8; 32];
        // Create payload covering multiple internal buffer states
        let payload = vec![0xAAu8; 5000];

        let mut network_buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut network_buffer);

        {
            let mut writer = EncryptedStream::new(&mut cursor, &key);
            writer.write_all(&payload).await.unwrap();
            writer.flush().await.unwrap();
        }

        let mut read_cursor = std::io::Cursor::new(&network_buffer);
        let mut reader = EncryptedStream::new(&mut read_cursor, &key);

        let mut decrypted = Vec::new();
        reader.read_to_end(&mut decrypted).await.unwrap();

        assert_eq!(decrypted, payload);
    }

    #[tokio::test]
    async fn test_multiple_writes() {
        let key = [0x22u8; 32];
        let p1 = b"Part 1";
        let p2 = b"Part 2";

        let mut network_buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut network_buffer);

        {
            let mut writer = EncryptedStream::new(&mut cursor, &key);
            writer.write_all(p1).await.unwrap();
            writer.write_all(p2).await.unwrap();
            writer.flush().await.unwrap();
        }

        let mut read_cursor = std::io::Cursor::new(&network_buffer);
        let mut reader = EncryptedStream::new(&mut read_cursor, &key);

        let mut buf = vec![0u8; p1.len() + p2.len()];
        reader.read_exact(&mut buf).await.unwrap();

        assert_eq!(&buf[..p1.len()], p1);
        assert_eq!(&buf[p1.len()..], p2);
    }

    #[tokio::test]
    async fn test_detect_tampering() {
        let key = [0x33u8; 32];
        let payload = b"Secret Data";

        let mut network_buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut network_buffer);

        {
            let mut writer = EncryptedStream::new(&mut cursor, &key);
            writer.write_all(payload).await.unwrap();
            writer.flush().await.unwrap();
        }

        // Tamper with the ciphertext (last byte)
        let len = network_buffer.len();
        network_buffer[len - 1] ^= 0xFF;

        let mut read_cursor = std::io::Cursor::new(&network_buffer);
        let mut reader = EncryptedStream::new(&mut read_cursor, &key);

        let mut buf = vec![0u8; payload.len()];
        let result = reader.read_exact(&mut buf).await;

        assert!(result.is_err());
    }

    struct SlowReader<R> {
        inner: R,
    }

    impl<R: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for SlowReader<R> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            // Force reading 1 byte at a time (or less if buf size is 0)
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }

            // Create a small buffer of size 1
            let mut small_buf = [0u8; 1];
            let mut read_buf = tokio::io::ReadBuf::new(&mut small_buf);

            ready!(Pin::new(&mut self.inner).poll_read(cx, &mut read_buf))?;

            let n = read_buf.filled().len();
            if n > 0 {
                buf.put_slice(read_buf.filled());
            }
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_partial_reads_handling() {
        let key = [0x55u8; 32];
        let payload = b"Stress Test Payload";

        let mut network_buffer = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut network_buffer);
            let mut writer = EncryptedStream::new(&mut cursor, &key);
            writer.write_all(payload).await.unwrap();
            writer.flush().await.unwrap();
        }

        let cursor = std::io::Cursor::new(network_buffer);
        let slow_reader = SlowReader { inner: cursor };
        let mut reader = EncryptedStream::new(slow_reader, &key);

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();

        assert_eq!(buf, payload);
    }

    #[tokio::test]
    async fn test_invalid_frame_header() {
        let key = [0x66u8; 32];
        let mut network_buffer = Vec::new();

        // Write invalid length (too small)
        network_buffer.extend_from_slice(&(10u32.to_be_bytes())); // < OVERHEAD
        network_buffer.extend_from_slice(&[0u8; 10]); // junk data

        let cursor = std::io::Cursor::new(network_buffer);
        let mut reader = EncryptedStream::new(cursor, &key);

        let mut buf = [0u8; 32];
        let err = reader.read(&mut buf).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Frame too short"));
    }

    #[tokio::test]
    async fn test_truncated_frame() {
        let key = [0x77u8; 32];
        let payload = b"Data";
        let mut network_buffer = Vec::new();

        {
            let mut cursor = std::io::Cursor::new(&mut network_buffer);
            let mut writer = EncryptedStream::new(&mut cursor, &key);
            writer.write_all(payload).await.unwrap();
            writer.flush().await.unwrap();
        }

        // Truncate the buffer by 1 byte
        let len = network_buffer.len();
        network_buffer.truncate(len - 1);

        let cursor = std::io::Cursor::new(network_buffer);
        let mut reader = EncryptedStream::new(cursor, &key);

        let mut buf = Vec::new();
        let err = reader.read_to_end(&mut buf).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn test_frame_too_large() {
        let key = [0x88u8; 32];
        let mut network_buffer = Vec::new();

        // Write invalid length (too large)
        // MAX_FRAME_SIZE (1MB) + 100
        let too_large_len = 1_000_000 + 100 + 100; // Large enough
        network_buffer.extend_from_slice(&(too_large_len as u32).to_be_bytes());
        network_buffer.extend_from_slice(&[0u8; 10]); // some junk

        let cursor = std::io::Cursor::new(network_buffer);
        let mut reader = EncryptedStream::new(cursor, &key);

        let mut buf = [0u8; 32];
        let err = reader.read(&mut buf).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Frame too large"));
    }

    #[tokio::test]
    async fn test_partial_frame_header() {
        let key = [0x99u8; 32];
        // Only write 2 bytes of the length prefix
        let network_buffer = vec![0x00, 0x00];

        let cursor = std::io::Cursor::new(network_buffer);
        let mut reader = EncryptedStream::new(cursor, &key);

        let mut buf = [0u8; 32];
        let err = reader.read(&mut buf).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
        assert!(err.to_string().contains("Partial frame length"));
    }
    struct FailingWriter {
        fail_after_bytes: usize,
        written: usize,
        fail_mode_write_zero: bool,
    }

    impl Unpin for FailingWriter {}

    impl tokio::io::AsyncWrite for FailingWriter {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            if self.written >= self.fail_after_bytes {
                if self.fail_mode_write_zero {
                    return Poll::Ready(Ok(0));
                } else {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "Synthetic Error",
                    )));
                }
            }
            let n = std::cmp::min(buf.len(), self.fail_after_bytes - self.written);
            // Simulate accepting valid bytes up to limit
            // But if fail_after_bytes is small, we might write 0?
            // If fail_after_bytes > written, n > 0 (assuming buf > 0)

            if n == 0 && !buf.is_empty() {
                if self.fail_mode_write_zero {
                    return Poll::Ready(Ok(0));
                } else {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "Synthetic Error",
                    )));
                }
            }

            self.written += n;
            Poll::Ready(Ok(n))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    // Stub AsyncRead for FailingWriter to satisfy bounds if needed (EncryptedStream wraps S: AsyncRead+AsyncWrite)
    impl tokio::io::AsyncRead for FailingWriter {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_write_zero_error() {
        let key = [0xAAu8; 32];
        let payload = b"Some data to write";

        // Write buffer logic:
        // EncryptedStream writes header (4+12=16 bytes) + ciphertext (len+16)
        // We want it to fail partway through.

        let writer = FailingWriter {
            fail_after_bytes: 10, // Fail after 10 bytes (during header write)
            written: 0,
            fail_mode_write_zero: true,
        };

        let mut stream = EncryptedStream::new(writer, &key);

        // This should fail when flushing or writing
        let result = stream.write_all(payload).await;

        // If write_all succeeds (buffering), flush must fail
        if result.is_ok() {
            let flush_res = stream.flush().await;
            assert!(flush_res.is_err());
            assert_eq!(flush_res.unwrap_err().kind(), io::ErrorKind::WriteZero);
        } else {
            assert!(result.is_err());
            // Could be WriteZero or other if propagated
        }
    }

    #[tokio::test]
    async fn test_io_error_propagation() {
        let key = [0xBBu8; 32];
        let payload = b"More data";

        let writer = FailingWriter {
            fail_after_bytes: 5,
            written: 0,
            fail_mode_write_zero: false, // BrokenPipe
        };

        let mut stream = EncryptedStream::new(writer, &key);
        let result = stream.write_all(payload).await;

        if let Ok(()) = result {
            let flush_res = stream.flush().await;
            assert!(flush_res.is_err());
            assert_eq!(flush_res.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
        } else if let Err(e) = result {
            assert_eq!(e.kind(), io::ErrorKind::BrokenPipe);
        }
    }
    struct FailingReader {
        data: Vec<u8>,
        read_idx: usize,
        fail_at_idx: usize,
    }

    impl Unpin for FailingReader {}

    impl tokio::io::AsyncRead for FailingReader {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            // Serve data until fail_at_idx
            let remaining = &self.data[self.read_idx..];
            if remaining.is_empty() {
                // Determine if we should fail or just return EOF
                // If we stopped exactly at fail_at_idx and fail_at_idx < potential_len, maybe fail?
                // But simplified: fail if we haven't read everything and hit index
                if self.read_idx == self.fail_at_idx && self.read_idx < self.data.len() + 100 {
                    // Logic here is tricky. Let's keep it simple:
                    // If we run out of data, it's EOF.
                    return Poll::Ready(Ok(()));
                }
                return Poll::Ready(Ok(()));
            }

            let limit = std::cmp::min(remaining.len(), self.fail_at_idx - self.read_idx);
            if limit == 0 && self.read_idx == self.fail_at_idx {
                // Reached fail point
                return Poll::Ready(Ok(())); // Unexpected EOF simulation
            }

            let chunk_size = std::cmp::min(limit, buf.remaining());
            buf.put_slice(&remaining[..chunk_size]);
            self.read_idx += chunk_size;
            Poll::Ready(Ok(()))
        }
    }

    impl tokio::io::AsyncWrite for FailingReader {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Ready(Ok(0))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_stream_unexpected_eof_after_header() {
        let key = [0xCCu8; 32];
        // Create valid encrypted frame
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, b"payload".as_ref()).unwrap();

        let mut frame = Vec::new();
        let frame_len = NONCE_SIZE + ciphertext.len();
        frame.extend_from_slice(&(frame_len as u32).to_be_bytes());
        frame.extend_from_slice(&nonce);
        frame.extend_from_slice(&ciphertext);

        // We want to simulate reading header (4 bytes) successfully,
        // passing the "Parse length" check,
        // but then failing to read the FULL frame body.

        // FailingReader logic needs to serve > 4 bytes but < full frame
        let partial_len = 4 + 5; // Header + 5 bytes

        let reader = FailingReader {
            data: frame,
            read_idx: 0,
            fail_at_idx: partial_len,
        };

        let mut stream = EncryptedStream::new(reader, &key);
        let mut buf = [0u8; 128];
        let err = stream.read(&mut buf).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
        assert!(err.to_string().contains("Incomplete frame"));
    }

    #[tokio::test]
    async fn test_stream_flush_propagation() {
        let key = [0xDDu8; 32];
        let stream = Vec::new();
        let mut enc_stream = EncryptedStream::new(stream, &key);

        // Flush should not error on Vec
        enc_stream.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_stream_decrypt_failure() {
        let key = [0xEEu8; 32];
        // Create frame with wrong key
        let wrong_key = [0xFFu8; 32];
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&wrong_key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, b"secret".as_ref()).unwrap();

        let mut frame = Vec::new();
        let frame_len = NONCE_SIZE + ciphertext.len();
        frame.extend_from_slice(&(frame_len as u32).to_be_bytes());
        frame.extend_from_slice(&nonce);
        frame.extend_from_slice(&ciphertext);

        let reader = io::Cursor::new(frame);
        let mut stream = EncryptedStream::new(reader, &key);
        let mut buf = [0u8; 128];

        let result = stream.read(&mut buf).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_invalid_frame_length() {
        let key = [0xAAu8; 32];
        // Frame length of 0
        let mut frame = Vec::new();
        frame.extend_from_slice(&0u32.to_be_bytes());

        let reader = io::Cursor::new(frame);
        let mut stream = EncryptedStream::new(reader, &key);
        let mut buf = [0u8; 128];

        let err = stream.read(&mut buf).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn test_stream_shutdown_propagation() {
        let key = [0xBBu8; 32];
        let stream = Vec::new();
        let mut enc_stream = EncryptedStream::new(stream, &key);

        enc_stream.shutdown().await.unwrap();
    }

    #[test]
    fn test_stream_constants() {
        assert_eq!(U32_SIZE, 4);
        assert_eq!(NONCE_SIZE, 12);
        assert_eq!(MAX_FRAME_SIZE, 64 * 1024);
        assert_eq!(FRAME_OVERHEAD, U32_SIZE + NONCE_SIZE + 16);
    }

    #[test]
    fn test_max_frame_size_reasonable() {
        // Should be 64KB
        assert_eq!(MAX_FRAME_SIZE, 64 * 1024);
    }

    #[tokio::test]
    async fn test_stream_empty_write() {
        let key = [0xFFu8; 32];
        let stream = Vec::new();
        let mut enc_stream = EncryptedStream::new(stream, &key);

        // Write empty buffer - should return 0
        let n = enc_stream.write(&[]).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn test_stream_read_on_empty_buffer() {
        let key = [0xAAu8; 32];
        // Empty network buffer (no encrypted data)
        let cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut reader = EncryptedStream::new(cursor, &key);

        let mut buf = [0u8; 64];
        // Should return Ok(0) for EOF
        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);
    }

    struct FragmentedReader {
        data: Vec<u8>,
        cursor: usize,
    }

    impl tokio::io::AsyncRead for FragmentedReader {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            if self.cursor >= self.data.len() {
                return std::task::Poll::Ready(Ok(()));
            }
            if buf.remaining() > 0 {
                // Read only 1 byte
                buf.put_slice(&self.data[self.cursor..self.cursor + 1]);
                self.cursor += 1;
                std::task::Poll::Ready(Ok(()))
            } else {
                std::task::Poll::Ready(Ok(()))
            }
        }
    }

    #[tokio::test]
    async fn test_stream_fragmented_read() {
        let key = vec![0u8; 32];
        let mut plaintext = vec![0u8; 120];
        // 120 bytes -> small enough for one frame, but we force fragmentation on read
        for (i, byte) in plaintext.iter_mut().enumerate() {
            *byte = i as u8;
        }

        // 1. Create a dummy encrypted stream to write data safely
        // We use duplex to write into "wire" format
        let (client, mut server) = tokio::io::duplex(4096);
        let mut encryptor = EncryptedStream::new(client, &key);

        let plaintext_clone = plaintext.clone();
        tokio::spawn(async move {
            encryptor.write_all(&plaintext_clone).await.unwrap();
            // Close to signal EOF
            encryptor.shutdown().await.unwrap();
        });

        // 2. Read all "encrypted" bytes from the wire
        let mut encrypted_data = Vec::new();
        server.read_to_end(&mut encrypted_data).await.unwrap();

        // 3. Create FragmentedReader to feed these bytes 1-by-1
        let reader = FragmentedReader {
            data: encrypted_data,
            cursor: 0,
        };

        // 4. Decrypt using FragmentedReader as source
        let mut stream = EncryptedStream::new(reader, &key);
        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await.unwrap();

        assert_eq!(buffer, plaintext);
    }

    #[tokio::test]
    async fn test_encrypted_stream_write_zero_error() {
        use std::io::{Error, ErrorKind};
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use tokio::io::AsyncWriteExt; // needed for write_all/flush

        struct ZeroWriter;
        impl tokio::io::AsyncWrite for ZeroWriter {
            fn poll_write(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _buf: &[u8],
            ) -> Poll<Result<usize, Error>> {
                // Simulate underlying stream accepting 0 bytes (WriteZero)
                Poll::Ready(Ok(0))
            }
            fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
                Poll::Ready(Ok(()))
            }
            fn poll_shutdown(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Result<(), Error>> {
                Poll::Ready(Ok(()))
            }
        }
        impl tokio::io::AsyncRead for ZeroWriter {
            fn poll_read(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _buf: &mut tokio::io::ReadBuf<'_>,
            ) -> Poll<Result<(), Error>> {
                Poll::Ready(Ok(()))
            }
        }

        let key = vec![0u8; 32];
        let mut stream = EncryptedStream::new(ZeroWriter, &key);

        let result = stream.write_all(b"test").await;
        if result.is_ok() {
            let flush_res = stream.flush().await;
            assert!(flush_res.is_err());
            assert_eq!(flush_res.unwrap_err().kind(), ErrorKind::WriteZero);
        } else {
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_stream_decryption_error() {
        // Line 177: Decryption failed coverage
        let key = [0x55u8; 32];
        let payload = b"Secret Data";

        // 1. Encrypt valid data
        let mut buffer = Vec::new();
        {
            let mut writer = EncryptedStream::new(std::io::Cursor::new(&mut buffer), &key);
            writer.write_all(payload).await.unwrap();
            writer.flush().await.unwrap();
        }

        // 2. Corrupt the ciphertext (skip length u32 + nonce 12)
        // Structure: [Length 4][Nonce 12][Ciphertext+Tag ...]
        let offset = 4 + 12;
        if buffer.len() > offset {
            buffer[offset] ^= 0xFF; // Flip bits in ciphertext/tag
        }

        // 3. Try to read back
        let mut reader = EncryptedStream::new(std::io::Cursor::new(&buffer), &key);
        let mut out = Vec::new();
        let err = reader.read_to_end(&mut out).await.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(err.to_string(), "Decryption failed");
    }
}
