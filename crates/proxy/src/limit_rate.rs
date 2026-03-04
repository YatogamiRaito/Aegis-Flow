use http_body_util::BodyExt;
use hyper::body::{Body, Bytes};
use hyper::{Request, Response, StatusCode};
use std::time::Duration;
use tokio::time::timeout;

pub struct RateAndSizeLimits {
    pub limit_rate: u64,       // bytes per second
    pub limit_rate_after: u64, // free burst size
    pub client_max_body_size: u64,
    pub client_header_timeout: Duration,
    pub client_body_timeout: Duration,
}

// Check content-length upfront if present
pub fn check_body_size_limit<B>(req: &Request<B>, max_size: u64) -> Result<(), Response<String>> {
    if let Some(cl) = req.headers().get(hyper::header::CONTENT_LENGTH) {
        if let Ok(len_str) = cl.to_str() {
            if let Ok(len) = len_str.parse::<u64>() {
                if len > max_size {
                    let mut res = Response::new("Payload Too Large".to_string());
                    *res.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
                    return Err(res);
                }
            }
        }
    }
    Ok(())
}

// Timeout wrapper for reading a full body (simulate body timeout)
pub async fn read_body_with_timeout<B>(
    body: B,
    max_size: u64,
    timeout_duration: Duration,
) -> Result<Bytes, Response<String>>
where
    B: Body + Unpin,
    B::Error: std::fmt::Debug,
{
    match timeout(timeout_duration, body.collect()).await {
        Ok(Ok(collected)) => {
            let bytes = collected.to_bytes();
            if bytes.len() as u64 > max_size {
                let mut res = Response::new("Payload Too Large".to_string());
                *res.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
                return Err(res);
            }
            Ok(bytes)
        }
        Ok(Err(_)) => {
            // Body read error
            let mut res = Response::new("Bad Request".to_string());
            *res.status_mut() = StatusCode::BAD_REQUEST;
            Err(res)
        }
        Err(_) => {
            // Timeout
            let mut res = Response::new("Request Timeout".to_string());
            *res.status_mut() = StatusCode::REQUEST_TIMEOUT;
            Err(res)
        }
    }
}

// Throttling isn't easily implemented on an un-streamed body or without a custom body wrapper,
// but we can simulate the `limit_rate` logic conceptually.
pub async fn apply_limit_rate(bytes_sent: u64, limits: &RateAndSizeLimits) {
    if limits.limit_rate == 0 {
        return;
    }

    if bytes_sent > limits.limit_rate_after {
        let overage = bytes_sent - limits.limit_rate_after;
        // Simple sleep based on overage and target rate
        let secs = overage as f64 / limits.limit_rate as f64;
        tokio::time::sleep(Duration::from_secs_f64(secs)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;
    use tokio::time::Instant;

    #[test]
    fn test_body_size_limit_header() {
        let req = Request::builder()
            .header("content-length", "2000")
            .body(())
            .unwrap();

        assert!(check_body_size_limit(&req, 1000).is_err());
        assert!(check_body_size_limit(&req, 3000).is_ok());
    }

    #[tokio::test]
    async fn test_read_body_with_timeout() {
        let body = Full::new(Bytes::from("hello world"));

        // Allowed size, generous timeout
        let res = read_body_with_timeout(body, 100, Duration::from_secs(1)).await;
        assert!(res.is_ok());

        // Too large
        let body2 = Full::new(Bytes::from("hello world"));
        let res2 = read_body_with_timeout(body2, 5, Duration::from_secs(1)).await;
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err().status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_limit_rate_sleep() {
        let limits = RateAndSizeLimits {
            limit_rate: 1000,
            limit_rate_after: 500,
            client_max_body_size: 1024,
            client_header_timeout: Duration::from_secs(5),
            client_body_timeout: Duration::from_secs(5),
        };

        let start = Instant::now();
        apply_limit_rate(400, &limits).await; // Under burst, no sleep
        assert!(start.elapsed() < Duration::from_millis(50));

        let start2 = Instant::now();
        apply_limit_rate(1500, &limits).await; // 1000 over, rate 1000/s -> 1 sec sleep
        assert!(start2.elapsed() >= Duration::from_millis(900));
    }
}
