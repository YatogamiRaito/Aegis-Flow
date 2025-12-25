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
                continue;
            }

            // 3. Parse length
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&me.read_buffer[..4]);
            let frame_len = u32::from_be_bytes(len_bytes) as usize;

            if frame_len < NONCE_SIZE + 16 {
                // Minimum: nonce + tag (empty payload)
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Frame too short",
                )));
            }
            if frame_len > MAX_FRAME_SIZE + FRAME_OVERHEAD {
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
                    me.decrypted_buffer.extend_from_slice(&plaintext);
                    me.read_buffer.advance(frame_len);
                    // Loop continues to serve from decrypted_buffer
                }
                Err(_) => {
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

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        match me.encryptor.encrypt(&nonce, buf) {
            Ok(ciphertext_tag) => {
                let frame_len = NONCE_SIZE + ciphertext_tag.len();

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
