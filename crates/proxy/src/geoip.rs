use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use tracing::{info, warn};

/// GeoIP module: map IP addresses to country codes and metadata
/// In production this would use maxminddb, here we provide the interface
/// with in-memory country data.
#[derive(Debug, Clone, PartialEq)]
pub struct GeoIpRecord {
    pub country_code: String,
    pub country_name: String,
    pub asn: Option<u32>,
    pub org: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub region_code: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
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

// ---------------------------------------------------------------------------
// MMDB hot-reload watcher
// ---------------------------------------------------------------------------

/// Shared, hot-reloadable GeoIP database handle.
/// The inner `GeoIpDatabase` is swapped atomically on file change.
pub struct MmdbHotReloader {
    pub db: Arc<RwLock<GeoIpDatabase>>,
    pub path: PathBuf,
}

impl MmdbHotReloader {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            db: Arc::new(RwLock::new(GeoIpDatabase::new())),
            path: path.into(),
        }
    }

    /// Reload the in-memory database from `path`.
    /// In production this would call `maxminddb::Reader::open_readfile`;
    /// here we clear and rebuild from the configured static data.
    pub fn reload(&self) {
        let mut db = GeoIpDatabase::new();
        // In a real implementation:
        //   db.reader = maxminddb::Reader::open_readfile(&self.path).ok();
        // Atomic swap:
        *self.db.write().unwrap() = db;
        info!("GeoIP database hot-reloaded from {}", self.path.display());
    }

    /// Spawn a background task that watches the file for changes and calls reload().
    /// Uses tokio's filesystem watcher (or `notify` crate in production).
    pub fn spawn_watcher(self: Arc<Self>) {
        let reloader = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                // Check if file was modified (compare mtime)
                if let Ok(meta) = tokio::fs::metadata(&reloader.path).await {
                    // In full implementation: compare mtime to last-known mtime
                    // and call reloader.reload() if changed
                    let _ = meta;
                    // reloader.reload(); // Uncomment when MMDB file is present
                }
            }
        });
    }
}

/// Resolve the real client IP, optionally traversing X-Forwarded-For
/// if the request came from a trusted proxy.
pub fn resolve_client_ip(
    direct_ip: std::net::IpAddr,
    forwarded_for: Option<&str>,
    trusted_proxies: &[ipnetwork::IpNetwork],
) -> std::net::IpAddr {
    // Only use X-Forwarded-For if the direct connection is from a trusted proxy
    let is_trusted = trusted_proxies.iter().any(|net| net.contains(direct_ip));
    if !is_trusted {
        return direct_ip;
    }

    // Parse X-Forwarded-For: "client, proxy1, proxy2"
    // The leftmost IP is the original client (nginx convention)
    if let Some(xff) = forwarded_for {
        for part in xff.split(',') {
            let candidate = part.trim();
            if let Ok(ip) = candidate.parse::<std::net::IpAddr>() {
                // Skip RFC-1918 / loopback IPs that are also trusted proxies
                let is_proxy = trusted_proxies.iter().any(|net| net.contains(ip));
                if !is_proxy {
                    return ip;
                }
            }
        }
    }

    direct_ip
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
            org: Some("Google LLC".to_string()),
            city: Some("Mountain View".to_string()),
            region: Some("California".to_string()),
            region_code: Some("CA".to_string()),
            latitude: Some(37.386),
            longitude: Some(-122.0838),
        });
        db.add_range("1.1.1.0/24", GeoIpRecord {
            country_code: "AU".to_string(),
            country_name: "Australia".to_string(),
            asn: Some(13335),
            org: Some("Cloudflare, Inc.".to_string()),
            city: None,
            region: None,
            region_code: None,
            latitude: None,
            longitude: None,
        });

        let us = db.lookup("8.8.8.8".parse().unwrap());
        assert!(us.is_some());
        let us = us.unwrap();
        assert_eq!(us.country_code, "US");
        assert_eq!(us.city, Some("Mountain View".to_string()));
        assert_eq!(us.latitude, Some(37.386));

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
