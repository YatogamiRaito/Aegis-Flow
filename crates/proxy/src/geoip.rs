use std::net::IpAddr;
use std::collections::BTreeMap;

/// GeoIP module: map IP addresses to country codes and metadata
/// In production this would use maxminddb, here we provide the interface
/// with in-memory country data.
#[derive(Debug, Clone, PartialEq)]
pub struct GeoIpRecord {
    pub country_code: String,
    pub country_name: String,
    pub asn: Option<u32>,
}

/// In-memory GeoIP database (for testing, production uses maxminddb MMDB)
pub struct GeoIpDatabase {
    country_map: Vec<(IpRange, GeoIpRecord)>,
}

#[derive(Debug, Clone)]
pub struct IpRange {
    pub start: u128,
    pub end: u128,
}

impl IpRange {
    pub fn from_cidr(cidr: &str) -> Option<Self> {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        
        let ip: IpAddr = parts[0].parse().ok()?;
        let prefix_len: u8 = parts[1].parse().ok()?;
        
        match ip {
            IpAddr::V4(v4) => {
                let ip_int = u32::from(v4) as u128;
                let mask: u128 = if prefix_len == 0 { 0 } else {
                    (u32::MAX << (32 - prefix_len)) as u128
                };
                let start = ip_int & mask;
                let end = start | (!mask & 0xFFFF_FFFF);
                Some(IpRange { start, end })
            }
            IpAddr::V6(v6) => {
                let ip_int = u128::from_be_bytes(v6.octets());
                let mask = if prefix_len == 0 { 0 } else { !((1u128 << (128 - prefix_len)) - 1) };
                let start = ip_int & mask;
                let end = start | !mask;
                Some(IpRange { start, end })
            }
        }
    }

    pub fn contains(&self, ip: u128) -> bool {
        ip >= self.start && ip <= self.end
    }
}

fn ip_to_u128(ip: IpAddr) -> u128 {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            u128::from(u32::from_be_bytes(octets))
        }
        IpAddr::V6(v6) => u128::from_be_bytes(v6.octets()),
    }
}

impl GeoIpDatabase {
    pub fn new() -> Self {
        Self { country_map: Vec::new() }
    }

    pub fn add_range(&mut self, cidr: &str, record: GeoIpRecord) {
        if let Some(range) = IpRange::from_cidr(cidr) {
            self.country_map.push((range, record));
        }
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<&GeoIpRecord> {
        let ip_int = ip_to_u128(ip);
        self.country_map.iter()
            .find(|(range, _)| range.contains(ip_int))
            .map(|(_, record)| record)
    }
}

/// Geo directive: map IP CIDR ranges to variable values
pub struct GeoDirective {
    pub source_var: String,
    pub target_var: String,
    pub default: Option<String>,
    pub ranges: Vec<(IpRange, String)>,
}

impl GeoDirective {
    pub fn new(source_var: &str, target_var: &str) -> Self {
        Self {
            source_var: source_var.to_string(),
            target_var: target_var.to_string(),
            default: None,
            ranges: Vec::new(),
        }
    }

    pub fn with_default(mut self, default: &str) -> Self {
        self.default = Some(default.to_string());
        self
    }

    pub fn add_range(&mut self, cidr: &str, value: &str) {
        if let Some(range) = IpRange::from_cidr(cidr) {
            self.ranges.push((range, value.to_string()));
        }
    }

    pub fn resolve(&self, ip: IpAddr) -> Option<String> {
        let ip_int = ip_to_u128(ip);
        
        // Find most specific (smallest range)
        let mut best: Option<(u128, &str)> = None;
        for (range, val) in &self.ranges {
            if range.contains(ip_int) {
                let size = range.end - range.start;
                if best.is_none() || size < best.unwrap().0 {
                    best = Some((size, val.as_str()));
                }
            }
        }
        
        best.map(|(_, v)| v.to_string())
            .or_else(|| self.default.clone())
    }
}

/// Country-based access control list
pub struct CountryAcl {
    pub denied_countries: Vec<String>,
    pub allowed_countries: Vec<String>, // empty = allow all
}

impl CountryAcl {
    pub fn deny_countries(countries: Vec<&str>) -> Self {
        Self {
            denied_countries: countries.iter().map(|s| s.to_uppercase()).collect(),
            allowed_countries: Vec::new(),
        }
    }

    pub fn is_allowed(&self, country_code: &str) -> bool {
        let code = country_code.to_uppercase();
        if !self.denied_countries.is_empty() && self.denied_countries.contains(&code) {
            return false;
        }
        if !self.allowed_countries.is_empty() && !self.allowed_countries.contains(&code) {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geoip_lookup() {
        let mut db = GeoIpDatabase::new();
        db.add_range("8.8.8.0/24", GeoIpRecord {
            country_code: "US".to_string(),
            country_name: "United States".to_string(),
            asn: Some(15169),
        });
        db.add_range("1.1.1.0/24", GeoIpRecord {
            country_code: "AU".to_string(),
            country_name: "Australia".to_string(),
            asn: Some(13335),
        });

        let us = db.lookup("8.8.8.8".parse().unwrap());
        assert!(us.is_some());
        assert_eq!(us.unwrap().country_code, "US");

        let au = db.lookup("1.1.1.1".parse().unwrap());
        assert!(au.is_some());
        assert_eq!(au.unwrap().country_code, "AU");

        let unknown = db.lookup("192.168.1.1".parse().unwrap());
        assert!(unknown.is_none());
    }

    #[test]
    fn test_geo_directive_cidr_match() {
        let mut geo = GeoDirective::new("$remote_addr", "$backend");
        geo.add_range("10.0.0.0/8", "internal");
        geo.add_range("0.0.0.0/0", "external");
        geo = geo.with_default("external");
        
        let internal = geo.resolve("10.5.0.1".parse().unwrap());
        // 10.x.x.x should match internal (more specific)
        assert_eq!(internal, Some("internal".to_string()));
    }

    #[test]
    fn test_geo_directive_default() {
        let geo = GeoDirective::new("$remote_addr", "$zone")
            .with_default("default_zone");
        
        let result = geo.resolve("8.8.8.8".parse().unwrap());
        assert_eq!(result, Some("default_zone".to_string()));
    }

    #[test]
    fn test_country_acl() {
        let acl = CountryAcl::deny_countries(vec!["CN", "RU", "KP"]);
        
        assert!(!acl.is_allowed("CN"));
        assert!(!acl.is_allowed("ru")); // case insensitive
        assert!(acl.is_allowed("US"));
        assert!(acl.is_allowed("DE"));
    }
}
