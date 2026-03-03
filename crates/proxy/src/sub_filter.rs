use regex::Regex;

/// Sub-filter: search and replace in response body
pub struct SubFilter {
    pub search: String,
    pub replace: String,
    pub once: bool,    // replace first occurrence only if true
    pub types: Vec<String>, // content-types to process
}

impl SubFilter {
    pub fn new(search: &str, replace: &str) -> Self {
        Self {
            search: search.to_string(),
            replace: replace.to_string(),
            once: false,
            types: vec!["text/html".to_string()],
        }
    }

    pub fn with_once(mut self, once: bool) -> Self {
        self.once = once;
        self
    }

    pub fn with_types(mut self, types: Vec<&str>) -> Self {
        self.types = types.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn should_process(&self, content_type: &str) -> bool {
        let ct = content_type.split(';').next().unwrap_or("").trim();
        self.types.iter().any(|t| ct == t || t == "*")
    }

    pub fn apply(&self, body: &str) -> String {
        if self.once {
            body.replacen(&self.search, &self.replace, 1)
        } else {
            body.replace(&self.search, &self.replace)
        }
    }
}

/// Body injection: prepend/append content
pub struct BodyInjection {
    pub prepend: Option<String>,
    pub append: Option<String>,
}

impl BodyInjection {
    pub fn new() -> Self {
        Self { prepend: None, append: None }
    }

    pub fn with_prepend(mut self, content: &str) -> Self {
        self.prepend = Some(content.to_string());
        self
    }

    pub fn with_append(mut self, content: &str) -> Self {
        self.append = Some(content.to_string());
        self
    }

    pub fn apply(&self, body: &str) -> String {
        let mut result = String::new();
        if let Some(ref p) = self.prepend {
            result.push_str(p);
        }
        result.push_str(body);
        if let Some(ref a) = self.append {
            result.push_str(a);
        }
        result
    }
}

/// SSI-style include directive parser
#[derive(Debug, PartialEq)]
pub enum SsiDirective {
    IncludeVirtual(String),
    IncludeFile(String),
    EchoVar(String),
    SetVar(String, String),
    Unknown(String),
}

pub fn parse_ssi_directives(input: &str) -> Vec<SsiDirective> {
    let re = Regex::new(r"<!--#(\w+)\s*(.*?)-->").unwrap();
    let attr_re = Regex::new(r#"(\w+)="([^"]*)""#).unwrap();
    
    let mut result = Vec::new();
    for cap in re.captures_iter(input) {
        let directive_name = &cap[1];
        let attrs_str = &cap[2];
        
        let mut attrs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for attr in attr_re.captures_iter(attrs_str) {
            attrs.insert(attr[1].to_string(), attr[2].to_string());
        }
        
        match directive_name {
            "include" => {
                if let Some(v) = attrs.get("virtual") {
                    result.push(SsiDirective::IncludeVirtual(v.clone()));
                } else if let Some(f) = attrs.get("file") {
                    result.push(SsiDirective::IncludeFile(f.clone()));
                }
            }
            "echo" => {
                if let Some(v) = attrs.get("var") {
                    result.push(SsiDirective::EchoVar(v.clone()));
                }
            }
            "set" => {
                if let (Some(var), Some(val)) = (attrs.get("var"), attrs.get("value")) {
                    result.push(SsiDirective::SetVar(var.clone(), val.clone()));
                }
            }
            other => {
                result.push(SsiDirective::Unknown(other.to_string()));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_filter_all() {
        let sf = SubFilter::new("foo", "bar");
        let result = sf.apply("foo is foo not baz");
        assert_eq!(result, "bar is bar not baz");
    }

    #[test]
    fn test_sub_filter_once() {
        let sf = SubFilter::new("foo", "bar").with_once(true);
        let result = sf.apply("foo is foo not baz");
        assert_eq!(result, "bar is foo not baz");
    }

    #[test]
    fn test_sub_filter_content_type() {
        let sf = SubFilter::new("foo", "bar");
        assert!(sf.should_process("text/html"));
        assert!(sf.should_process("text/html; charset=utf-8"));
        assert!(!sf.should_process("application/json"));
    }

    #[test]
    fn test_body_injection() {
        let inj = BodyInjection::new()
            .with_prepend("<header>")
            .with_append("</footer>");
        
        let result = inj.apply("body content");
        assert_eq!(result, "<header>body content</footer>");
    }

    #[test]
    fn test_ssi_include_virtual() {
        let html = r#"<html><!--#include virtual="/header.html"--></html>"#;
        let directives = parse_ssi_directives(html);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0], SsiDirective::IncludeVirtual("/header.html".to_string()));
    }

    #[test]
    fn test_ssi_echo_var() {
        let html = r#"Hello <!--#echo var="REQUEST_URI"-->"#;
        let directives = parse_ssi_directives(html);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0], SsiDirective::EchoVar("REQUEST_URI".to_string()));
    }

    #[test]
    fn test_ssi_set_var() {
        let html = r#"<!--#set var="name" value="world"-->"#;
        let directives = parse_ssi_directives(html);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0], SsiDirective::SetVar("name".to_string(), "world".to_string()));
    }
}
