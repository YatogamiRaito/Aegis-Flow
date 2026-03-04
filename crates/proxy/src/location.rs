use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum LocationError {
    #[error("Invalid location regex: {0}")]
    InvalidRegex(#[from] regex::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocationMatchType {
    Exact,               // =
    Prefix,              // (none)
    PreferredPrefix,     // ^~
    Regex,               // ~
    RegexCaseInsensitive // ~*
}

impl Default for LocationMatchType {
    fn default() -> Self {
        LocationMatchType::Prefix
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationBlock {
    pub path: String,
    #[serde(default)]
    pub match_type: LocationMatchType,
    pub proxy_pass: Option<String>,
    pub root: Option<String>,
    #[serde(default)]
    pub try_files: Vec<String>,
    pub return_directive: Option<crate::rewrite::ReturnDirective>,
    #[serde(default)]
    pub rewrite: Vec<crate::rewrite::RewriteRule>,
    pub auth_request: Option<String>,
    #[serde(default)]
    pub auth_request_set: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub limit_except: crate::config::LimitExceptConfig,
}

#[derive(Debug)]
pub struct ParsedLocationBlock {
    pub config: LocationBlock,
    pub regex: Option<Regex>,
}

impl ParsedLocationBlock {
    pub fn parse(config: LocationBlock) -> Result<Self, LocationError> {
        let regex = match config.match_type {
            LocationMatchType::Regex => Some(Regex::new(&config.path)?),
            LocationMatchType::RegexCaseInsensitive => {
                Some(Regex::new(&format!("(?i){}", config.path))?)
            }
            _ => None,
        };

        Ok(Self { config, regex })
    }
}

/// Nginx location priority:
/// 1. Exact match (= /path)
/// 2. Preferred prefix match (^~ /static/)
/// 3. Regex match (~, ~*) (in order of appearance)
/// 4. Longest prefix match (/api/)
pub fn match_location<'a>(
    locations: &'a [ParsedLocationBlock],
    uri: &str,
) -> Option<&'a ParsedLocationBlock> {
    let mut exact_match: Option<&'a ParsedLocationBlock> = None;
    let mut preferred_prefix_match: Option<(&'a ParsedLocationBlock, usize)> = None;
    let mut regex_match: Option<&'a ParsedLocationBlock> = None;
    let mut longest_prefix_match: Option<(&'a ParsedLocationBlock, usize)> = None;

    for loc in locations {
        match loc.config.match_type {
            LocationMatchType::Exact => {
                if loc.config.path == uri {
                    exact_match = Some(loc);
                    break; // Priority 1, we can stop evaluating
                }
            }
            LocationMatchType::PreferredPrefix => {
                if uri.starts_with(&loc.config.path) {
                    let len = loc.config.path.len();
                    if preferred_prefix_match.map_or(true, |(_, l)| len > l) {
                        preferred_prefix_match = Some((loc, len));
                    }
                }
            }
            LocationMatchType::Regex | LocationMatchType::RegexCaseInsensitive => {
                // Return first matching regex unless a preferred prefix already matched
                // Wait, preferred prefix actually stops regex evaluation in nginx,
                // but we should just evaluate all and then choose by priority at the end.
                if regex_match.is_none() {
                    if let Some(re) = &loc.regex {
                        if re.is_match(uri) {
                            regex_match = Some(loc);
                        }
                    }
                }
            }
            LocationMatchType::Prefix => {
                if uri.starts_with(&loc.config.path) {
                    let len = loc.config.path.len();
                    if longest_prefix_match.map_or(true, |(_, l)| len > l) {
                        longest_prefix_match = Some((loc, len));
                    }
                }
            }
        }
    }

    if let Some(exact) = exact_match {
        return Some(exact);
    }

    // Guard: if there are no locations, we can't use first() as a fallback
    if locations.is_empty() {
        return regex_match;
    }

    // Nginx rule: if the longest prefix match has the "^~" modifier,
    // then regular expressions are not checked.
    // So if preferred_prefix is longer or equal to longest_prefix, use it.
    let (pref_loc, pref_len) = preferred_prefix_match.unwrap_or((locations.first().unwrap(), 0));
    let (long_loc, long_len) = longest_prefix_match.unwrap_or((locations.first().unwrap(), 0));

    if preferred_prefix_match.is_some() && pref_len >= long_len {
        return Some(pref_loc);
    }

    if let Some(regex) = regex_match {
        return Some(regex);
    }

    if longest_prefix_match.is_some() {
        return Some(long_loc);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_match_priority() {
        let loc1 = ParsedLocationBlock::parse(LocationBlock {
            path: "/api/".to_string(),
            match_type: LocationMatchType::Prefix,
            proxy_pass: None,
            root: None,
            try_files: vec![],
            return_directive: None,
            rewrite: vec![],
            auth_request: None,
            auth_request_set: std::collections::HashMap::new(),
            limit_except: crate::config::LimitExceptConfig { methods: vec![], deny: "all".to_string() },
        }).unwrap();

        let loc2 = ParsedLocationBlock::parse(LocationBlock {
            path: "/api/exact".to_string(),
            match_type: LocationMatchType::Exact,
            proxy_pass: None,
            root: None,
            try_files: vec![],
            return_directive: None,
            rewrite: vec![],
            auth_request: None,
            auth_request_set: std::collections::HashMap::new(),
            limit_except: crate::config::LimitExceptConfig { methods: vec![], deny: "all".to_string() },
        }).unwrap();

        let loc3 = ParsedLocationBlock::parse(LocationBlock {
            path: "^/api/.*\\.jpg$".to_string(),
            match_type: LocationMatchType::Regex,
            proxy_pass: None,
            root: None,
            try_files: vec![],
            return_directive: None,
            rewrite: vec![],
            auth_request: None,
            auth_request_set: std::collections::HashMap::new(),
            limit_except: crate::config::LimitExceptConfig { methods: vec![], deny: "all".to_string() },
        }).unwrap();

        let loc4 = ParsedLocationBlock::parse(LocationBlock {
            path: "/api/static/".to_string(),
            match_type: LocationMatchType::PreferredPrefix,
            proxy_pass: None,
            root: None,
            try_files: vec![],
            return_directive: None,
            rewrite: vec![],
            auth_request: None,
            auth_request_set: std::collections::HashMap::new(),
            limit_except: crate::config::LimitExceptConfig { methods: vec![], deny: "all".to_string() },
        }).unwrap();

        let locs = vec![loc1, loc2, loc3, loc4];

        // Exact match
        let m = match_location(&locs, "/api/exact").unwrap();
        assert_eq!(m.config.match_type, LocationMatchType::Exact);

        // Preferred Prefix match over regex
        let m = match_location(&locs, "/api/static/image.jpg").unwrap();
        assert_eq!(m.config.match_type, LocationMatchType::PreferredPrefix);

        // Regex match over Prefix
        let m = match_location(&locs, "/api/image.jpg").unwrap();
        assert_eq!(m.config.match_type, LocationMatchType::Regex);

        // Longest Prefix Match
        let m = match_location(&locs, "/api/other").unwrap();
        assert_eq!(m.config.path, "/api/");
        assert_eq!(m.config.match_type, LocationMatchType::Prefix);
    }
}
