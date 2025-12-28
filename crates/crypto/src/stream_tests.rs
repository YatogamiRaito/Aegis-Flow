// Additional tests for stream.rs

#[cfg(test)]
mod additional_stream_tests {
    use super::super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_stream_multiple_small_writes() {
        let key = [0xAAu8; 32];
        let mut buffer = Vec::new();
        let mut stream = EncryptedStream::new(&mut buffer, &key);

        // Write multiple small chunks
        stream.write_all(b"a").await.unwrap();
        stream.write_all(b"b").await.unwrap();
        stream.write_all(b"c").await.unwrap();
        stream.flush().await.unwrap();

        assert!(!buffer.is_empty());
    }

    #[tokio::test]
    async fn test_stream_empty_write() {
        let key = [0xBBu8; 32];
        let mut buffer = Vec::new();
        let mut stream = EncryptedStream::new(&mut buffer, &key);

        stream.write_all(b"").await.unwrap();
        stream.flush().await.unwrap();

        // Empty write shouldnot create frames
        assert!(buffer.is_empty() || buffer.len() < 50);
    }

    #[tokio::test]
    async fn test_stream_read_after_write() {
        let key = [0xCCu8; 32];
        let mut buffer = Vec::new();
        
        // Write encrypted data
        {
            let mut writer = EncryptedStream::new(&mut buffer, &key);
            writer.write_all(b"test data").await.unwrap();
            writer.flush().await.unwrap();
        }

        // Read it back
        let mut reader = EncryptedStream::new(io::Cursor::new(&buffer), &key);
        let mut decrypted = Vec::new();
        reader.read_to_end(&mut decrypted).await.unwrap();

        assert_eq!(decrypted, b"test data");
    }

    #[tokio::test]
    async fn test_stream_large_data() {
        let key = [0xDDu8; 32];
        let large_data = vec![0x42u8; 100_000]; // 100KB
        let mut buffer = Vec::new();

        {
            let mut writer = EncryptedStream::new(&mut buffer, &key);
            writer.write_all(&large_data).await.unwrap();
            writer.flush().await.unwrap();
        }

        let mut reader = EncryptedStream::new(io::Cursor::new(&buffer), &key);
        let mut decrypted = Vec::new();
        reader.read_to_end(&mut decrypted).await.unwrap();

        assert_eq!(decrypted, large_data);
    }
}
