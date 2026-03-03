use rcgen::{generate_simple_self_signed, CertifiedKey};
use std::net::IpAddr;

/// Check if the given IP is a private/local IP (RFC 1918, loopback, link-local)
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || (v4.octets()[0] == 169 && v4.octets()[1] == 254)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unicast_link_local()
        }
    }
}

/// Generate a self-signed TLS certificate for the given hostnames
pub fn generate_self_signed(domains: Vec<String>) -> Result<(String, String), rcgen::Error> {
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(domains)?;
    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();
    Ok((cert_pem, key_pem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_ip_detection() {
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(is_private_ip("127.0.0.1".parse().unwrap()));
        assert!(is_private_ip("::1".parse().unwrap()));
        
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip("1.1.1.1".parse().unwrap()));
    }

    #[test]
    fn test_self_signed_cert() {
        let domains = vec!["localhost".to_string(), "example.local".to_string()];
        let result = generate_self_signed(domains);
        assert!(result.is_ok());
        
        let (cert_pem, key_pem) = result.unwrap();
        assert!(cert_pem.contains("CERTIFICATE"));
        assert!(key_pem.contains("PRIVATE KEY"));
    }
}
