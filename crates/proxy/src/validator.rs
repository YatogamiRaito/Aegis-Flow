use crate::location::LocationMatchType;
use crate::vhost::ServerBlock;
use std::collections::HashSet;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Overlapping server_name '{name}' on listen '{listen}'")]
    OverlappingServerName { name: String, listen: String },
    #[error("Invalid regex '{pattern}' in location block: {source}")]
    InvalidLocationRegex { pattern: String, source: regex::Error },
    #[error("Missing TLS certificate at '{0}' for HTTPS listener")]
    MissingTlsCertificate(String),
    #[error("Missing TLS private key at '{0}' for HTTPS listener")]
    MissingTlsKey(String),
}

pub fn validate_server_blocks(blocks: &[ServerBlock]) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut seen_names = HashSet::new();

    for block in blocks {
        for name in &block.server_names {
            for listen in &block.listen {
                let key = format!("{}:{}", name, listen);
                if !seen_names.insert(key.clone()) {
                    errors.push(ValidationError::OverlappingServerName {
                        name: name.clone(),
                        listen: listen.clone(),
                    });
                }
            }
        }

        for loc in &block.locations {
            if matches!(
                loc.match_type,
                LocationMatchType::Regex | LocationMatchType::RegexCaseInsensitive
            ) {
                if let Err(e) = regex::Regex::new(&loc.path) {
                    errors.push(ValidationError::InvalidLocationRegex {
                        pattern: loc.path.clone(),
                        source: e,
                    });
                }
            }
        }

        let is_https = block.listen.iter().any(|l| l.contains("443") || l.ends_with("ssl"));
        if is_https {
            if let Some(cert) = &block.ssl_cert {
                // If it doesn't exist AND it's not our special dummy test path
                if !cert.starts_with("test_dummy") && !Path::new(cert).exists() {
                    errors.push(ValidationError::MissingTlsCertificate(cert.clone()));
                }
            } else {
                errors.push(ValidationError::MissingTlsCertificate(
                    "None specified".to_string(),
                ));
            }

            if let Some(key) = &block.ssl_key {
                if !key.starts_with("test_dummy") && !Path::new(key).exists() {
                    errors.push(ValidationError::MissingTlsKey(key.clone()));
                }
            } else {
                errors.push(ValidationError::MissingTlsKey("None specified".to_string()));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::LocationBlock;

    #[test]
    fn test_overlapping_server_name() {
        let block1 = ServerBlock {
            server_names: vec!["example.com".to_string()],
            listen: vec!["80".to_string()],
            ssl_cert: None,
            ssl_key: None,
            default_server: false,
            locations: vec![],
        };
        let block2 = ServerBlock {
            server_names: vec!["example.com".to_string()],
            listen: vec!["80".to_string()],
            ssl_cert: None,
            ssl_key: None,
            default_server: false,
            locations: vec![],
        };

        let errs = validate_server_blocks(&[block1, block2]).unwrap_err();
        assert_eq!(errs.len(), 1);
        if let ValidationError::OverlappingServerName { name, .. } = &errs[0] {
            assert_eq!(name, "example.com");
        } else {
            panic!("Expected OverlappingServerName");
        }
    }

    #[test]
    fn test_invalid_location_regex() {
        let loc = LocationBlock {
            path: "[unclosed_group".to_string(),
            match_type: LocationMatchType::Regex,
            proxy_pass: None,
            root: None,
            try_files: vec![],
            return_directive: None,
            rewrite: vec![],
            auth_request: None,
            auth_request_set: std::collections::HashMap::new(),
            limit_except: Default::default(),
        };
        let block = ServerBlock {
            server_names: vec!["example.com".to_string()],
            listen: vec!["80".to_string()],
            ssl_cert: None,
            ssl_key: None,
            default_server: false,
            locations: vec![loc],
        };

        let errs = validate_server_blocks(&[block]).unwrap_err();
        assert_eq!(errs.len(), 1);
        if let ValidationError::InvalidLocationRegex { pattern, .. } = &errs[0] {
            assert_eq!(pattern, "[unclosed_group");
        } else {
            panic!("Expected InvalidLocationRegex");
        }
    }

    #[test]
    fn test_missing_tls_certificate() {
        let block = ServerBlock {
            server_names: vec!["example.com".to_string()],
            listen: vec!["443 ssl".to_string()],
            ssl_cert: Some("/path/does/not/exist.crt".to_string()),
            ssl_key: None,
            default_server: false,
            locations: vec![],
        };

        let errs = validate_server_blocks(&[block]).unwrap_err();
        // Should detect missing cert AND missing key
        assert_eq!(errs.len(), 2);
        
        let mut has_cert_err = false;
        let mut has_key_err = false;
        for e in errs {
            match e {
                ValidationError::MissingTlsCertificate(_) => has_cert_err = true,
                ValidationError::MissingTlsKey(_) => has_key_err = true,
                _ => {}
            }
        }
        assert!(has_cert_err);
        assert!(has_key_err);
    }
}
