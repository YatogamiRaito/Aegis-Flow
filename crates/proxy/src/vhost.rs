use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VHostError {
    #[error("Invalid server name regex: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone)]
pub enum ServerNameMatcher {
    Exact(String),
    LeadingWildcard(String),  // e.g. .example.com for *.example.com
    TrailingWildcard(String), // e.g. example. for example.*
    Regex(Regex),
}

impl ServerNameMatcher {
    pub fn parse(name: &str) -> Result<Self, VHostError> {
        if name.starts_with('~') {
            let pattern = name[1..].trim();
            let regex = Regex::new(pattern)?;
            return Ok(Self::Regex(regex));
        }

        if name.starts_with("*.") {
            return Ok(Self::LeadingWildcard(name[1..].to_string()));
        }

        if name.ends_with(".*") {
            return Ok(Self::TrailingWildcard(name[..name.len() - 1].to_string()));
        }

        Ok(Self::Exact(name.to_string()))
    }

    pub fn matches(&self, hostname: &str) -> bool {
        match self {
            Self::Exact(exact) => exact.eq_ignore_ascii_case(hostname),
            Self::LeadingWildcard(suffix) => hostname.ends_with(suffix),
            Self::TrailingWildcard(prefix) => hostname.starts_with(prefix),
            Self::Regex(regex) => regex.is_match(hostname),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerBlock {
    #[serde(default)]
    pub server_names: Vec<String>,
    #[serde(default = "default_listen")]
    pub listen: Vec<String>,
    pub ssl_cert: Option<String>,
    pub ssl_key: Option<String>,
    #[serde(default)]
    pub default_server: bool,
    #[serde(default)]
    pub locations: Vec<crate::location::LocationBlock>,
}

fn default_listen() -> Vec<String> {
    vec!["0.0.0.0:80".to_string()]
}

#[derive(Debug)]
pub struct ParsedServerBlock {
    pub config: ServerBlock,
    pub matchers: Vec<ServerNameMatcher>,
}

pub fn parse_server_blocks(
    configs: Vec<ServerBlock>,
) -> Result<Vec<ParsedServerBlock>, VHostError> {
    let mut parsed_blocks = Vec::with_capacity(configs.len());
    for config in configs {
        let mut matchers = Vec::new();
        for name in &config.server_names {
            matchers.push(ServerNameMatcher::parse(name)?);
        }
        parsed_blocks.push(ParsedServerBlock { config, matchers });
    }
    Ok(parsed_blocks)
}

/// Nginx priority:
/// 1. exact match
/// 2. longest leading wildcard
/// 3. longest trailing wildcard
/// 4. first matching regex
/// 5. default list
pub fn select_server<'a>(
    blocks: &'a [ParsedServerBlock],
    hostname: &str,
) -> Option<&'a ParsedServerBlock> {
    let mut best_exact: Option<&'a ParsedServerBlock> = None;
    let mut best_leading: Option<(&'a ParsedServerBlock, usize)> = None;
    let mut best_trailing: Option<(&'a ParsedServerBlock, usize)> = None;
    let mut best_regex: Option<&'a ParsedServerBlock> = None;
    let mut default_server: Option<&'a ParsedServerBlock> = None;

    for block in blocks {
        if block.config.default_server && default_server.is_none() {
            default_server = Some(block);
        }

        for matcher in &block.matchers {
            if matcher.matches(hostname) {
                match matcher {
                    ServerNameMatcher::Exact(_) => {
                        if best_exact.is_none() {
                            best_exact = Some(block);
                        }
                    }
                    ServerNameMatcher::LeadingWildcard(suffix) => {
                        let len = suffix.len();
                        if best_leading.map_or(true, |(_, l)| len > l) {
                            best_leading = Some((block, len));
                        }
                    }
                    ServerNameMatcher::TrailingWildcard(prefix) => {
                        let len = prefix.len();
                        if best_trailing.map_or(true, |(_, l)| len > l) {
                            best_trailing = Some((block, len));
                        }
                    }
                    ServerNameMatcher::Regex(_) => {
                        if best_regex.is_none() {
                            best_regex = Some(block);
                        }
                    }
                }
            }
        }
    }

    if let Some(exact) = best_exact {
        return Some(exact);
    }
    if let Some((leading, _)) = best_leading {
        return Some(leading);
    }
    if let Some((trailing, _)) = best_trailing {
        return Some(trailing);
    }
    if let Some(regex) = best_regex {
        return Some(regex);
    }
    default_server.or_else(|| blocks.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_name_matcher() {
        // exact
        let exact = ServerNameMatcher::parse("example.com").unwrap();
        assert!(exact.matches("example.com"));
        assert!(!exact.matches("www.example.com"));

        // leading wildcard
        let lw = ServerNameMatcher::parse("*.example.com").unwrap();
        assert!(lw.matches("www.example.com"));
        assert!(lw.matches("sub.www.example.com"));
        assert!(!lw.matches("example.com"));

        // trailing wildcard
        let tw = ServerNameMatcher::parse("example.*").unwrap();
        assert!(tw.matches("example.com"));
        assert!(tw.matches("example.org"));
        assert!(!tw.matches("www.example.com"));

        // regex
        let re = ServerNameMatcher::parse("~^www\\d+\\.example\\.com$").unwrap();
        assert!(re.matches("www1.example.com"));
        assert!(re.matches("www99.example.com"));
        assert!(!re.matches("www.example.com"));
    }

    #[test]
    fn test_select_server() {
        let exact_block = ParsedServerBlock {
            config: ServerBlock {
                server_names: vec!["example.com".to_string()],
                listen: vec!["80".to_string()],
                ssl_cert: None,
                ssl_key: None,
                default_server: false,
                locations: vec![],
            },
            matchers: vec![ServerNameMatcher::parse("example.com").unwrap()],
        };

        let leading_block = ParsedServerBlock {
            config: ServerBlock {
                server_names: vec!["*.example.com".to_string()],
                listen: vec!["80".to_string()],
                ssl_cert: None,
                ssl_key: None,
                default_server: false,
                locations: vec![],
            },
            matchers: vec![ServerNameMatcher::parse("*.example.com").unwrap()],
        };

        let default_block = ParsedServerBlock {
            config: ServerBlock {
                server_names: vec!["_".to_string()],
                listen: vec!["80".to_string()],
                ssl_cert: None,
                ssl_key: None,
                default_server: true,
                locations: vec![],
            },
            matchers: vec![ServerNameMatcher::parse("_").unwrap()],
        };

        let blocks = vec![default_block, leading_block, exact_block];

        let selected = select_server(&blocks, "example.com").unwrap();
        assert_eq!(selected.config.server_names[0], "example.com");

        let selected = select_server(&blocks, "www.example.com").unwrap();
        assert_eq!(selected.config.server_names[0], "*.example.com");

        let selected = select_server(&blocks, "unknown.org").unwrap();
        assert_eq!(selected.config.server_names[0], "_");
    }
}
