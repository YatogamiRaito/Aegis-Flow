use crate::upstream::StickyConfig;
use hyper::http::{HeaderValue, Request, Response};

pub fn check_sticky_session<B>(req: &Request<B>, config: &StickyConfig) -> Option<String> {
    if let Some(cookie_header) = req.headers().get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for pair in cookie_str.split(';') {
                let pair = pair.trim();
                if let Some((name, value)) = pair.split_once('=') {
                    if name == config.cookie_name {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn issue_sticky_session<B>(res: &mut Response<B>, config: &StickyConfig, server_addr: &str) {
    let cookie_val = format!("{}={}; Path=/; HttpOnly", config.cookie_name, server_addr);
    if let Ok(hv) = HeaderValue::from_str(&cookie_val) {
        res.headers_mut().append("set-cookie", hv);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sticky_sessions() {
        let config = StickyConfig {
            cookie_name: "ROUTEID".to_string(),
        };

        // Check extracting cookie
        let mut req = Request::builder()
            .header("cookie", "user_id=123; ROUTEID=backend-1; test=abc")
            .body(())
            .unwrap();

        let val = check_sticky_session(&req, &config);
        assert_eq!(val, Some("backend-1".to_string()));

        let req_empty = Request::builder().body(()).unwrap();
        assert_eq!(check_sticky_session(&req_empty, &config), None);

        // Check issuing cookie
        let mut res = Response::builder().body(()).unwrap();
        issue_sticky_session(&mut res, &config, "backend-2");

        let cookie_set = res.headers().get("set-cookie").unwrap().to_str().unwrap();
        assert!(cookie_set.contains("ROUTEID=backend-2"));
        assert!(cookie_set.contains("HttpOnly"));
        assert!(cookie_set.contains("Path=/"));
    }
}
