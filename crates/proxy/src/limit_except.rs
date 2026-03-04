use crate::config::LimitExceptConfig;
use crate::metrics;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};

/// Validates whether the incoming HTTP Request's Method is permitted by the Location's `limit_except` block.
/// Returns an early 405 Method Not Allowed Response if rejected, or None if permitted.
pub fn check_method<B>(
    config: &LimitExceptConfig,
    req: &Request<B>,
) -> Option<Response<Full<Bytes>>> {
    // If no methods are defined, there are no limitations for this block
    if config.methods.is_empty() {
        return None;
    }

    let req_method = req.method().as_str();

    // Check if the request's method is in the allowed list
    let is_allowed = config
        .methods
        .iter()
        .any(|m| m.eq_ignore_ascii_case(req_method));

    if !is_allowed {
        // Evaluate the `deny` directive (e.g. "all").
        // Right now we only check if deny is configured to effectively reject the request.
        if config.deny.eq_ignore_ascii_case("all") {
            // we'll record a raw WAF counter inc here if accessible, otherwise record a WAF event route
            // To be safe against metrics scope:
            metrics::record_error("method_not_allowed");

            return Some(
                Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Full::new(Bytes::from(format!(
                        "405 Method Not Allowed: {}\n",
                        req_method
                    ))))
                    .unwrap(),
            );
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_except_allowed() {
        let config = LimitExceptConfig {
            methods: vec!["GET".to_string(), "POST".to_string()],
            deny: "all".to_string(),
        };

        let req = Request::builder().method("GET").uri("/").body(()).unwrap();
        assert!(check_method(&config, &req).is_none());
    }

    #[test]
    fn test_limit_except_denied() {
        let config = LimitExceptConfig {
            methods: vec!["GET".to_string()],
            deny: "all".to_string(),
        };

        let req = Request::builder()
            .method("DELETE")
            .uri("/")
            .body(())
            .unwrap();
        let res = check_method(&config, &req).unwrap();
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
