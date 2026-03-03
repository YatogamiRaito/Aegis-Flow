use anyhow::{bail, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyHeader {
    pub source_addr: SocketAddr,
    pub dest_addr: SocketAddr,
}

impl ProxyHeader {
    /// Parses a PROXY Protocol v1 string.
    /// Example: `PROXY TCP4 198.51.100.22 203.0.113.7 35646 80\r\n`
    pub fn parse_v1(input: &[u8]) -> Result<Option<(Self, usize)>> {
        let header_str = match std::str::from_utf8(input) {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };

        if !header_str.starts_with("PROXY ") {
            if input.len() >= 6 {
                bail!("Invalid PROXY protocol v1 signature");
            }
            return Ok(None);
        }

        let end_idx = match header_str.find("\r\n") {
            Some(idx) => idx,
            None => {
                // The max length of a v1 header is 107 bytes
                if input.len() >= 107 {
                    bail!("PROXY protocol v1 header too long without CRLF");
                }
                return Ok(None);
            }
        };

        let line = &header_str[..end_idx];
        let parts: Vec<&str> = line.split(' ').collect();

        if parts.len() < 2 {
            bail!("Malformed PROXY protocol v1 header");
        }

        let parsed_header = match parts[1] {
            "UNKNOWN" => {
                // Valid but we drop the connection info
                ProxyHeader {
                    source_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
                    dest_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
                }
            }
            "TCP4" | "TCP6" => {
                if parts.len() != 6 {
                    bail!("Malformed PROXY protocol v1 TCP header");
                }

                let src_ip: IpAddr = parts[2].parse()?;
                let dst_ip: IpAddr = parts[3].parse()?;
                let src_port: u16 = parts[4].parse()?;
                let dst_port: u16 = parts[5].parse()?;

                ProxyHeader {
                    source_addr: SocketAddr::new(src_ip, src_port),
                    dest_addr: SocketAddr::new(dst_ip, dst_port),
                }
            }
            _ => bail!("Unsupported PROXY protocol v1 family"),
        };

        Ok(Some((parsed_header, end_idx + 2)))
    }

    /// Generates a PROXY Protocol v1 payload mapped to standard TCP bounds
    pub fn to_v1_bytes(&self) -> Vec<u8> {
        let family = match self.source_addr.ip() {
            IpAddr::V4(_) => "TCP4",
            IpAddr::V6(_) => "TCP6",
        };

        format!(
            "PROXY {} {} {} {} {}\r\n",
            family,
            self.source_addr.ip(),
            self.dest_addr.ip(),
            self.source_addr.port(),
            self.dest_addr.port()
        )
        .into_bytes()
    }

    /// Parses a PROXY Protocol v2 binary header.
    /// V2 signature: \x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A
    pub fn parse_v2(input: &[u8]) -> Result<Option<(Self, usize)>> {
        let signature = b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A";
        
        if input.len() < 16 {
            // Check if it at least starts with the signature before dropping
            for i in 0..input.len() {
                if input[i] != signature[i] {
                    return Ok(None); // Not a v2 or incomplete signature
                }
            }
            return Ok(None); // Need more bytes
        }

        if &input[0..12] != signature {
            return Ok(None);
        }

        let version_cmd = input[12];
        if (version_cmd & 0xF0) != 0x20 {
            bail!("Unsupported PROXY protocol version");
        }

        let fam_prot = input[13];
        let len = u16::from_be_bytes([input[14], input[15]]) as usize;

        if input.len() < 16 + len {
            return Ok(None); // Incomplete packet
        }

        // Only parse if command is PROXY (0x01)
        if (version_cmd & 0x0F) != 0x01 {
            return Ok(Some((
                ProxyHeader {
                    source_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
                    dest_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
                },
                16 + len,
            )));
        }

        let payload = &input[16..16 + len];

        let header = match fam_prot {
            0x11 => {
                // AF_INET + STREAM
                if len < 12 {
                    bail!("Truncated IPv4 PROXY header");
                }
                let src_ip = Ipv4Addr::new(payload[0], payload[1], payload[2], payload[3]);
                let dst_ip = Ipv4Addr::new(payload[4], payload[5], payload[6], payload[7]);
                let src_port = u16::from_be_bytes([payload[8], payload[9]]);
                let dst_port = u16::from_be_bytes([payload[10], payload[11]]);

                ProxyHeader {
                    source_addr: SocketAddr::new(IpAddr::V4(src_ip), src_port),
                    dest_addr: SocketAddr::new(IpAddr::V4(dst_ip), dst_port),
                }
            }
            0x21 => {
                // AF_INET6 + STREAM
                if len < 36 {
                    bail!("Truncated IPv6 PROXY header");
                }
                let mut src_octets = [0u8; 16];
                src_octets.copy_from_slice(&payload[0..16]);
                let mut dst_octets = [0u8; 16];
                dst_octets.copy_from_slice(&payload[16..32]);

                let src_ip = Ipv6Addr::from(src_octets);
                let dst_ip = Ipv6Addr::from(dst_octets);
                let src_port = u16::from_be_bytes([payload[32], payload[33]]);
                let dst_port = u16::from_be_bytes([payload[34], payload[35]]);

                ProxyHeader {
                    source_addr: SocketAddr::new(IpAddr::V6(src_ip), src_port),
                    dest_addr: SocketAddr::new(IpAddr::V6(dst_ip), dst_port),
                }
            }
            _ => {
                // Unspecified or unsupported AF
                ProxyHeader {
                    source_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
                    dest_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
                }
            }
        };

        Ok(Some((header, 16 + len)))
    }

    /// Generates a PROXY Protocol v2 binary header.
    pub fn to_v2_bytes(&self) -> Vec<u8> {
        let signature = b"\x0D\x0A\x0D\x0A\x00\x0D\x0A\x51\x55\x49\x54\x0A";
        
        // Version 2 | Command PROXY
        let version_cmd = 0x21u8;
        
        let mut buf = Vec::with_capacity(signature.len() + 4 + 36);
        buf.extend_from_slice(signature);
        buf.push(version_cmd);

        match (self.source_addr.ip(), self.dest_addr.ip()) {
            (IpAddr::V4(src), IpAddr::V4(dst)) => {
                buf.push(0x11); // AF_INET + STREAM
                buf.extend_from_slice(&12u16.to_be_bytes());
                buf.extend_from_slice(&src.octets());
                buf.extend_from_slice(&dst.octets());
            }
            (IpAddr::V6(src), IpAddr::V6(dst)) => {
                buf.push(0x21); // AF_INET6 + STREAM
                buf.extend_from_slice(&36u16.to_be_bytes());
                buf.extend_from_slice(&src.octets());
                buf.extend_from_slice(&dst.octets());
            }
            _ => {
                // Unspecified
                buf.push(0x00);
                buf.extend_from_slice(&0u16.to_be_bytes());
            }
        }
        
        if let (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) = (self.source_addr.ip(), self.dest_addr.ip()) {
            buf.extend_from_slice(&self.source_addr.port().to_be_bytes());
            buf.extend_from_slice(&self.dest_addr.port().to_be_bytes());
        }

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v1_tcp4() {
        let input = b"PROXY TCP4 192.168.1.100 10.0.0.1 50000 80\r\nSome other data";
        let (header, bytes_read) = ProxyHeader::parse_v1(input).unwrap().unwrap();
        
        assert_eq!(bytes_read, 44);
        assert_eq!(header.source_addr, "192.168.1.100:50000".parse().unwrap());
        assert_eq!(header.dest_addr, "10.0.0.1:80".parse().unwrap());
    }

    #[test]
    fn test_to_v1_bytes() {
        let header = ProxyHeader {
            source_addr: "192.168.1.100:50000".parse().unwrap(),
            dest_addr: "10.0.0.1:80".parse().unwrap(),
        };
        let bytes = header.to_v1_bytes();
        assert_eq!(bytes, b"PROXY TCP4 192.168.1.100 10.0.0.1 50000 80\r\n");
    }

    #[test]
    fn test_v2_roundtrip_ipv4() {
        let src = "192.168.1.100:50000".parse().unwrap();
        let dst = "10.0.0.1:80".parse().unwrap();
        let header = ProxyHeader {
            source_addr: src,
            dest_addr: dst,
        };

        let encoded = header.to_v2_bytes();
        let (decoded, bytes_read) = ProxyHeader::parse_v2(&encoded).unwrap().unwrap();

        assert_eq!(bytes_read, 28);
        assert_eq!(decoded.source_addr, src);
        assert_eq!(decoded.dest_addr, dst);
    }
}
