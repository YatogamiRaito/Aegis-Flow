use base64::{Engine as _, engine::general_purpose};
use bcrypt::verify;
use hyper::header::AUTHORIZATION;
use hyper::{Request, Response, StatusCode};

pub struct BasicAuthConfig {
    // Map of username -> bcrypt hash
    pub users: std::collections::HashMap<String, String>,
    pub realm: String,
}

impl BasicAuthConfig {
    pub fn new(realm: &str) -> Self {
        Self {
            users: std::collections::HashMap::new(),
            realm: realm.to_string(),
        }
    }

    pub fn add_user(&mut self, username: &str, hash: &str) {
        self.users.insert(username.to_string(), hash.to_string());
    }

    pub fn check_auth<B>(&self, req: &Request<B>) -> bool {
        if let Some(auth_val) = req.headers().get(AUTHORIZATION) {
            if let Ok(auth_str) = auth_val.to_str() {
                if auth_str.starts_with("Basic ") {
                    let encoded = &auth_str[6..];
                    if let Ok(decoded) = general_purpose::STANDARD.decode(encoded) {
                        if let Ok(cred_str) = String::from_utf8(decoded) {
                            if let Some((user, pass)) = cred_str.split_once(':') {
                                if let Some(hash) = self.users.get(user) {
                                    if let Ok(valid) = verify(pass, hash) {
                                        return valid;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }
}

pub fn create_401_response<B>(realm: &str) -> Response<B>
where
    B: Default,
{
    let mut resp = Response::new(B::default());
    *resp.status_mut() = StatusCode::UNAUTHORIZED;
    let auth_header = format!("Basic realm=\"{}\"", realm);
    resp.headers_mut().insert(
        hyper::header::WWW_AUTHENTICATE,
        hyper::header::HeaderValue::from_str(&auth_header).unwrap(),
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use bcrypt::{DEFAULT_COST, hash};

    #[test]
    fn test_basic_auth() {
        let mut config = BasicAuthConfig::new("Restricted");

        let pw_hash = hash("secret", DEFAULT_COST).unwrap();
        config.add_user("admin", &pw_hash);

        // admin:secret -> YWRtaW46c2VjcmV0
        let req = Request::builder()
            .header(AUTHORIZATION, "Basic YWRtaW46c2VjcmV0")
            .body(())
            .unwrap();

        assert!(config.check_auth(&req));

        // wrong password -> root:secret -> root is not added
        let req2 = Request::builder()
            .header(AUTHORIZATION, "Basic cm9vdDpzZWNyZXQ=")
            .body(())
            .unwrap();

        assert!(!config.check_auth(&req2));

        // admin:wrong -> YWRtaW46d3Jvbmc=
        let req3 = Request::builder()
            .header(AUTHORIZATION, "Basic YWRtaW46d3Jvbmc=")
            .body(())
            .unwrap();

        assert!(!config.check_auth(&req3));
    }

    #[test]
    fn test_401() {
        let res: Response<String> = create_401_response("MyRealm");
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            res.headers().get(hyper::header::WWW_AUTHENTICATE).unwrap(),
            "Basic realm=\"MyRealm\""
        );
    }
}
