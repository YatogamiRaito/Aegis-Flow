use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RewriteFlag {
    Last,
    Break,
    Redirect,
    Permanent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteRule {
    pub pattern: String,
    pub replacement: String,
    pub flag: Option<RewriteFlag>,
}

pub struct CompiledRewriteRule {
    pub config: RewriteRule,
    pub regex: Regex,
}

impl CompiledRewriteRule {
    pub fn new(config: RewriteRule) -> Result<Self, regex::Error> {
        // We'll wrap in ^ $ maybe if required, but standard regex applies.
        let regex = Regex::new(&config.pattern)?;
        Ok(Self { config, regex })
    }

    /// Evaluates the rewrite rule.
    /// If it matches, returns `Some((rewritten_string, flag))`.
    /// `rewritten_string` will have regex capture groups substituted (e.g., $1).
    pub fn apply(&self, uri: &str) -> Option<(String, Option<RewriteFlag>)> {
        if self.regex.is_match(uri) {
            let replaced = self
                .regex
                .replace(uri, &self.config.replacement)
                .into_owned();
            Some((replaced, self.config.flag.clone()))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnDirective {
    pub code: u16,
    pub text_or_url: Option<String>,
}

impl ReturnDirective {
    pub fn return_response(
        &self,
    ) -> Result<hyper::Response<hyper::body::Bytes>, hyper::http::Error> {
        let mut builder = hyper::Response::builder().status(self.code);

        if let Some(ref txt) = self.text_or_url {
            if self.code == 301
                || self.code == 302
                || self.code == 303
                || self.code == 307
                || self.code == 308
            {
                builder = builder.header("Location", txt);
                return builder.body(hyper::body::Bytes::new());
            } else {
                return builder.body(hyper::body::Bytes::from(txt.clone()));
            }
        }

        builder.body(hyper::body::Bytes::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_rule() {
        let rule = CompiledRewriteRule::new(RewriteRule {
            pattern: "^/users/(\\d+)/profile$".to_string(),
            replacement: "/profile?id=$1".to_string(),
            flag: Some(RewriteFlag::Break),
        })
        .unwrap();

        let (new_uri, flag) = rule.apply("/users/123/profile").unwrap();
        assert_eq!(new_uri, "/profile?id=123");
        assert_eq!(flag, Some(RewriteFlag::Break));

        assert!(rule.apply("/other/path").is_none());
    }

    #[test]
    fn test_rewrite_redirect() {
        let rule = CompiledRewriteRule::new(RewriteRule {
            pattern: "^/old-site/(.*)$".to_string(),
            replacement: "https://new-site.com/$1".to_string(),
            flag: Some(RewriteFlag::Permanent),
        })
        .unwrap();

        let (new_uri, flag) = rule.apply("/old-site/pages/about").unwrap();
        assert_eq!(new_uri, "https://new-site.com/pages/about");
        assert_eq!(flag, Some(RewriteFlag::Permanent));
    }

    #[test]
    fn test_return_directive() {
        let ret = ReturnDirective {
            code: 200,
            text_or_url: Some("OK".to_string()),
        };
        let resp = ret.return_response().unwrap();
        assert_eq!(resp.status(), 200);

        let body_bytes = resp.into_body();
        assert_eq!(body_bytes.as_ref(), b"OK");

        let redir = ReturnDirective {
            code: 301,
            text_or_url: Some("https://example.com/".to_string()),
        };
        let resp2 = redir.return_response().unwrap();
        assert_eq!(resp2.status(), 301);
        assert_eq!(
            resp2.headers().get("Location").unwrap(),
            "https://example.com/"
        );
    }
}
