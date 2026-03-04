#[derive(Debug, PartialEq)]
pub enum HttpRange {
    /// Range `start-end` (both inclusive)
    Range(u64, u64),
    /// Range `start-` (to end of file)
    StartOnly(u64),
    /// Range `-suffix` (last N bytes)
    Suffix(u64),
}

impl HttpRange {
    /// Parse the `Range` HTTP header value (e.g., "bytes=0-499")
    pub fn parse(header_value: &str) -> Option<Vec<HttpRange>> {
        if !header_value.starts_with("bytes=") {
            return None;
        }

        let ranges_str = &header_value[6..];
        let mut ranges = Vec::new();

        for part in ranges_str.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if part.starts_with('-') {
                if let Ok(suffix) = part[1..].parse::<u64>() {
                    ranges.push(HttpRange::Suffix(suffix));
                } else {
                    return None; // Invalid syntax
                }
            } else if part.ends_with('-') {
                if let Ok(start) = part[..part.len() - 1].parse::<u64>() {
                    ranges.push(HttpRange::StartOnly(start));
                } else {
                    return None; // Invalid syntax
                }
            } else {
                let parts: Vec<&str> = part.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(start), Ok(end)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
                    {
                        if start <= end {
                            ranges.push(HttpRange::Range(start, end));
                        } else {
                            return None; // Invalid range (start > end)
                        }
                    } else {
                        return None; // Invalid syntax
                    }
                } else {
                    return None; // Unknown format
                }
            }
        }

        if ranges.is_empty() {
            None
        } else {
            Some(ranges)
        }
    }

    /// Resolve against a specific content size, returning absolute inclusive byte bounds.
    /// Returns None if the range is unsatisfiable.
    pub fn resolve(&self, total_size: u64) -> Option<(u64, u64)> {
        if total_size == 0 {
            return None;
        }

        match self {
            HttpRange::Range(start, end) => {
                if *start >= total_size {
                    None
                } else {
                    let actual_end = std::cmp::min(*end, total_size - 1);
                    Some((*start, actual_end))
                }
            }
            HttpRange::StartOnly(start) => {
                if *start >= total_size {
                    None
                } else {
                    Some((*start, total_size - 1))
                }
            }
            HttpRange::Suffix(suffix) => {
                if *suffix == 0 {
                    None
                } else {
                    let start = if *suffix >= total_size {
                        0
                    } else {
                        total_size - *suffix
                    };
                    Some((start, total_size - 1))
                }
            }
        }
    }
}

/// Helper to generate a multi-part boundary string
pub fn generate_boundary() -> String {
    format!("aegis-flow-boundary-{:016x}", rand::random::<u64>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        assert_eq!(
            HttpRange::parse("bytes=0-499"),
            Some(vec![HttpRange::Range(0, 499)])
        );
        assert_eq!(
            HttpRange::parse("bytes=500-"),
            Some(vec![HttpRange::StartOnly(500)])
        );
        assert_eq!(
            HttpRange::parse("bytes=-500"),
            Some(vec![HttpRange::Suffix(500)])
        );
        assert_eq!(
            HttpRange::parse("bytes=0-100, 200-300"),
            Some(vec![HttpRange::Range(0, 100), HttpRange::Range(200, 300)])
        );
        assert_eq!(
            HttpRange::parse("bytes=500-, -500"),
            Some(vec![HttpRange::StartOnly(500), HttpRange::Suffix(500)])
        );
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(HttpRange::parse("lines=0-100"), None); // wrong unit
        assert_eq!(HttpRange::parse("bytes=100-0"), None); // start > end
        assert_eq!(HttpRange::parse("bytes=abc-def"), None);
        assert_eq!(HttpRange::parse("bytes=-"), None);
    }

    #[test]
    fn test_resolve() {
        let size = 1000;
        assert_eq!(HttpRange::Range(0, 499).resolve(size), Some((0, 499)));
        assert_eq!(HttpRange::Range(500, 1500).resolve(size), Some((500, 999))); // Truncates
        assert_eq!(HttpRange::Range(1000, 1500).resolve(size), None); // Unsatisfiable

        assert_eq!(HttpRange::StartOnly(500).resolve(size), Some((500, 999)));
        assert_eq!(HttpRange::StartOnly(1000).resolve(size), None);

        assert_eq!(HttpRange::Suffix(500).resolve(size), Some((500, 999)));
        assert_eq!(HttpRange::Suffix(1500).resolve(size), Some((0, 999))); // Resolves to toàn bộ file
        assert_eq!(HttpRange::Suffix(0).resolve(size), None); // Unsatisfiable per spec in some interpretations, although browsers rarely send `-0`
    }
}
