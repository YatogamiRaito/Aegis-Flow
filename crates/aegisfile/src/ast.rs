use crate::parser::{Directive, SiteBlock};

// Simplified internal representation of proxy config
#[derive(Debug, Clone, PartialEq)]
pub struct ProxyLocation {
    pub path: String,
    pub upstream: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiteConfig {
    pub domains: Vec<String>,
    pub locations: Vec<ProxyLocation>,
    pub file_server_root: Option<String>,
    pub redirect: Option<(String, u16)>, // (target, status)
}

pub fn convert_sites(sites: &[SiteBlock]) -> Vec<SiteConfig> {
    sites.iter().map(|site| convert_site(site)).collect()
}

fn convert_site(site: &SiteBlock) -> SiteConfig {
    let mut locations = Vec::new();
    let mut file_server_root = None;
    let mut redirect = None;

    for directive in &site.directives {
        match directive.name.as_str() {
            "reverse_proxy" => {
                if directive.args.len() >= 2 {
                    locations.push(ProxyLocation {
                        path: directive.args[0].clone(),
                        upstream: directive.args[1].clone(),
                    });
                }
            }
            "file_server" => {
                if !directive.args.is_empty() {
                    file_server_root = Some(directive.args[0].clone());
                } else {
                    file_server_root = Some("./".to_string());
                }
            }
            "redirect" => {
                if directive.args.len() >= 1 {
                    let status = if directive.args.len() >= 2 {
                        directive.args[1].parse::<u16>().unwrap_or(301)
                    } else {
                        301
                    };
                    redirect = Some((directive.args[0].clone(), status));
                }
            }
            _ => {}
        }
    }
    
    SiteConfig {
        domains: site.domains.clone(),
        locations,
        file_server_root,
        redirect,
    }
}

// Simple formatter: re-emits aegisfile format from sites
pub fn format_sites(sites: &[SiteBlock]) -> String {
    let mut out = String::new();
    for site in sites {
        out.push_str(&site.domains.join(", "));
        out.push_str(" {\n");
        for directive in &site.directives {
            out.push_str("    ");
            out.push_str(&directive.name);
            for arg in &directive.args {
                out.push(' ');
                if arg.contains(' ') {
                    out.push('"');
                    out.push_str(arg);
                    out.push('"');
                } else {
                    out.push_str(arg);
                }
            }
            out.push('\n');
        }
        out.push_str("}\n");
    }
    out
}

// Validate sites: return errors as strings
pub fn validate_sites(sites: &[SiteBlock]) -> Vec<String> {
    let mut errors = Vec::new();
    
    for site in sites {
        if site.domains.is_empty() {
            errors.push("Found site block with no domains".to_string());
        }
        for directive in &site.directives {
            match directive.name.as_str() {
                "reverse_proxy" => {
                    if directive.args.len() < 2 {
                        errors.push(format!(
                            "reverse_proxy requires at least 2 arguments (path, upstream) in site {:?}",
                            site.domains
                        ));
                    }
                }
                "redirect" => {
                    if directive.args.is_empty() {
                        errors.push(format!("redirect requires at least 1 argument in site {:?}", site.domains));
                    }
                }
                _ => {}
            }
        }
    }
    
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_ast_to_config() {
        let input = "example.com {\n    reverse_proxy /api localhost:3000\n    file_server /static\n}\n";
        let sites = parse(input);
        let configs = convert_sites(&sites);
        
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].domains, vec!["example.com"]);
        assert_eq!(configs[0].locations.len(), 1);
        assert_eq!(configs[0].locations[0].path, "/api");
        assert_eq!(configs[0].locations[0].upstream, "localhost:3000");
        assert_eq!(configs[0].file_server_root, Some("/static".to_string()));
    }

    #[test]
    fn test_formatter() {
        let input = "example.com {\n    reverse_proxy /api localhost:3000\n}\n";
        let sites = parse(input);
        let formatted = format_sites(&sites);
        
        assert!(formatted.contains("example.com {"));
        assert!(formatted.contains("reverse_proxy /api localhost:3000"));
        assert!(formatted.contains('}'));
    }

    #[test]
    fn test_validation() {
        let input = "example.com {\n    reverse_proxy /api\n}\n"; // missing upstream
        let sites = parse(input);
        let errors = validate_sites(&sites);
        
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("reverse_proxy")));
    }

    #[test]
    fn test_validation_valid() {
        let input = "example.com {\n    reverse_proxy /api localhost:3000\n}\n";
        let sites = parse(input);
        let errors = validate_sites(&sites);
        assert!(errors.is_empty());
    }
}
