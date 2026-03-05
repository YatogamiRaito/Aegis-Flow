/// Aegisfile Lexer & Parser
///
/// Example format (not runnable code):
///
/// ```text
/// example.com {
///     reverse_proxy /api/* localhost:3000
///     file_server /static /var/www
/// }
/// ```

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    QuotedString(String),
    LBrace,
    RBrace,
    Comment(String),
    Newline,
    EOF,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                chars.next();
                tokens.push(Token::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::RBrace);
            }
            '#' => {
                // Comment - consume until end of line
                let mut comment = String::new();
                chars.next(); // consume '#'
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    comment.push(c);
                    chars.next();
                }
                tokens.push(Token::Comment(comment.trim().to_string()));
            }
            '"' => {
                chars.next(); // consume opening quote
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next();
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(Token::QuotedString(s));
            }
            '\n' => {
                chars.next();
                tokens.push(Token::Newline);
            }
            ' ' | '\t' | '\r' => {
                chars.next(); // skip whitespace
            }
            _ => {
                // Identifier
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '#' {
                        break;
                    }
                    ident.push(c);
                    chars.next();
                }
                if !ident.is_empty() {
                    tokens.push(Token::Identifier(ident));
                }
            }
        }
    }
    tokens.push(Token::EOF);
    tokens
}

#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    pub name: String,
    pub args: Vec<String>,
    pub block: Option<Vec<Directive>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiteBlock {
    pub domains: Vec<String>,
    pub directives: Vec<Directive>,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        tok
    }

    fn skip_newlines(&mut self) {
        while self.peek() == &Token::Newline {
            self.advance();
        }
    }

    pub fn parse_sites(&mut self) -> Vec<SiteBlock> {
        let mut sites = Vec::new();
        while self.peek() != &Token::EOF {
            self.skip_newlines();
            if self.peek() == &Token::EOF {
                break;
            }

            // Skip comments
            if let Token::Comment(_) = self.peek() {
                self.advance();
                continue;
            }

            // Collect domain names
            let mut domains = Vec::new();
            while let Token::Identifier(name) = self.peek().clone() {
                domains.push(name);
                self.advance();
                // Multiple domains on same line
                if let Token::Identifier(_) = self.peek() {
                    // comma-separated or space-separated; just collect
                } else {
                    break;
                }
            }

            self.skip_newlines();

            if self.peek() == &Token::LBrace {
                self.advance(); // consume {
                let directives = self.parse_directives();
                sites.push(SiteBlock {
                    domains,
                    directives,
                });
            }
        }
        sites
    }

    fn parse_directives(&mut self) -> Vec<Directive> {
        let mut directives = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek() {
                Token::RBrace | Token::EOF => {
                    self.advance();
                    break;
                }
                Token::Comment(_) => {
                    self.advance();
                }
                Token::Identifier(_) => {
                    let dir = self.parse_directive();
                    directives.push(dir);
                }
                _ => {
                    self.advance();
                }
            }
        }
        directives
    }

    fn parse_directive(&mut self) -> Directive {
        let name = if let Token::Identifier(s) = self.advance().clone() {
            s
        } else {
            String::new()
        };
        let mut args = Vec::new();
        let mut block = None;

        loop {
            match self.peek() {
                Token::Newline | Token::EOF => {
                    self.advance();
                    break;
                }
                Token::LBrace => {
                    self.advance();
                    block = Some(self.parse_directives());
                    break;
                }
                Token::Identifier(s) => {
                    let s = s.clone();
                    args.push(s);
                    self.advance();
                }
                Token::QuotedString(s) => {
                    let s = s.clone();
                    args.push(s);
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        Directive { name, args, block }
    }
}

pub fn parse(input: &str) -> Vec<SiteBlock> {
    let tokens = tokenize(input);
    let mut parser = Parser::new(tokens);
    parser.parse_sites()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let input = "example.com { reverse_proxy /api localhost:3000 }";
        let tokens = tokenize(input);

        assert!(tokens.contains(&Token::LBrace));
        assert!(tokens.contains(&Token::RBrace));
        assert!(
            tokens
                .iter()
                .any(|t| matches!(t, Token::Identifier(s) if s == "example.com"))
        );
        assert!(
            tokens
                .iter()
                .any(|t| matches!(t, Token::Identifier(s) if s == "reverse_proxy"))
        );
    }

    #[test]
    fn test_lexer_comment() {
        let input = "# this is a comment\nexample.com { }";
        let tokens = tokenize(input);
        assert!(
            tokens
                .iter()
                .any(|t| matches!(t, Token::Comment(s) if s.contains("this is a comment")))
        );
    }

    #[test]
    fn test_lexer_quoted_string() {
        let input = r#"reverse_proxy "/path/to" localhost:3000"#;
        let tokens = tokenize(input);
        assert!(
            tokens
                .iter()
                .any(|t| matches!(t, Token::QuotedString(s) if s == "/path/to"))
        );
    }

    #[test]
    fn test_parser_site_block() {
        let input = "example.com {\n    reverse_proxy /api localhost:3000\n}\n";
        let sites = parse(input);

        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].domains, vec!["example.com"]);
        assert_eq!(sites[0].directives.len(), 1);
        assert_eq!(sites[0].directives[0].name, "reverse_proxy");
        assert_eq!(sites[0].directives[0].args, vec!["/api", "localhost:3000"]);
    }

    #[test]
    fn test_parser_multiple_sites() {
        let input = "example.com {\n    reverse_proxy /api localhost:3000\n}\napi.example.com {\n    file_server\n}\n";
        let sites = parse(input);
        assert_eq!(sites.len(), 2);
    }

    #[test]
    fn test_parser_nested_block() {
        let input = "example.com {\n    reverse_proxy {\n        to localhost:3000\n    }\n}\n";
        let sites = parse(input);
        assert_eq!(sites.len(), 1);
        assert!(sites[0].directives[0].block.is_some());
    }
}
