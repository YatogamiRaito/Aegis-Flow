use std::path::PathBuf;
use tokio::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CertMetadata {
    pub domains: Vec<String>,
    pub issuer: String,
    pub not_after: String,  // RFC 3339 timestamp
    pub cert_path: String,
    pub key_path: String,
}

pub struct CertStorage {
    pub base_dir: PathBuf,
}

impl CertStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub async fn save_cert(&self, domain: &str, cert_pem: &str, key_pem: &str) -> std::io::Result<(PathBuf, PathBuf)> {
        let dir = self.base_dir.join(domain);
        fs::create_dir_all(&dir).await?;
        
        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("key.pem");
        
        fs::write(&cert_path, cert_pem).await?;
        fs::write(&key_path, key_pem).await?;
        
        Ok((cert_path, key_path))
    }

    pub async fn load_cert(&self, domain: &str) -> std::io::Result<(String, String)> {
        let dir = self.base_dir.join(domain);
        
        let cert_pem = fs::read_to_string(dir.join("cert.pem")).await?;
        let key_pem = fs::read_to_string(dir.join("key.pem")).await?;
        
        Ok((cert_pem, key_pem))
    }

    pub async fn save_metadata(&self, domain: &str, meta: &CertMetadata) -> std::io::Result<()> {
        let dir = self.base_dir.join(domain);
        fs::create_dir_all(&dir).await?;
        
        let meta_json = serde_json::to_string_pretty(meta).unwrap();
        fs::write(dir.join("metadata.json"), meta_json).await?;
        
        Ok(())
    }

    pub async fn load_metadata(&self, domain: &str) -> std::io::Result<CertMetadata> {
        let dir = self.base_dir.join(domain);
        let json = fs::read_to_string(dir.join("metadata.json")).await?;
        let meta: CertMetadata = serde_json::from_str(&json).unwrap();
        Ok(meta)
    }
    
    pub async fn list_domains(&self) -> std::io::Result<Vec<String>> {
        let mut domains = Vec::new();
        let mut entries = fs::read_dir(&self.base_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    domains.push(name.to_string());
                }
            }
        }
        
        Ok(domains)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cert_storage() {
        let tmpdir = tempdir().unwrap();
        let storage = CertStorage::new(tmpdir.path().to_path_buf());
        
        let cert_pem = "-----BEGIN CERTIFICATE-----\nfakedata\n-----END CERTIFICATE-----\n";
        let key_pem = "-----BEGIN EC PRIVATE KEY-----\nfakekey\n-----END EC PRIVATE KEY-----\n";
        
        storage.save_cert("example.com", cert_pem, key_pem).await.unwrap();
        
        let (loaded_cert, loaded_key) = storage.load_cert("example.com").await.unwrap();
        assert_eq!(loaded_cert, cert_pem);
        assert_eq!(loaded_key, key_pem);
    }

    #[tokio::test]
    async fn test_metadata_storage() {
        let tmpdir = tempdir().unwrap();
        let storage = CertStorage::new(tmpdir.path().to_path_buf());
        
        let meta = CertMetadata {
            domains: vec!["example.com".to_string()],
            issuer: "Let's Encrypt".to_string(),
            not_after: "2026-01-01T00:00:00Z".to_string(),
            cert_path: "/path/to/cert.pem".to_string(),
            key_path: "/path/to/key.pem".to_string(),
        };
        
        storage.save_metadata("example.com", &meta).await.unwrap();
        
        let loaded = storage.load_metadata("example.com").await.unwrap();
        assert_eq!(loaded.domains[0], "example.com");
        assert_eq!(loaded.issuer, "Let's Encrypt");
    }
}
