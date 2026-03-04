use hyper::{Response, StatusCode, header::RETRY_AFTER};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RateLimitZone {
    pub name: String,
    pub key_type: String, // e.g., "$remote_addr"
    pub rate_per_second: f64,
    pub burst: u32,
    pub nodelay: bool,
}

pub struct TokenBucket {
    pub capacity: u32,
    pub available_tokens: f64,
    pub last_refill: Instant,
    pub refill_rate_per_sec: f64,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate_per_sec: f64) -> Self {
        Self {
            capacity,
            available_tokens: capacity as f64,
            last_refill: Instant::now(),
            refill_rate_per_sec,
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let add_tokens = elapsed * self.refill_rate_per_sec;

        if add_tokens > 0.0 {
            self.available_tokens = (self.available_tokens + add_tokens).min(self.capacity as f64);
            self.last_refill = now;
        }
    }

    pub fn acquire(&mut self) -> Result<(), Duration> {
        self.refill();
        if self.available_tokens >= 1.0 {
            self.available_tokens -= 1.0;
            Ok(())
        } else {
            // Need (1.0 - available) / rate seconds
            let needed = 1.0 - self.available_tokens;
            let secs_to_wait = needed / self.refill_rate_per_sec;
            Err(Duration::from_secs_f64(secs_to_wait))
        }
    }
}

pub struct BucketManager {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    zone: RateLimitZone,
}

impl BucketManager {
    pub fn new(zone: RateLimitZone) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            zone,
        }
    }

    pub async fn check_limit(&self, key: &str) -> Option<Duration> {
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.zone.burst, self.zone.rate_per_second));

        match bucket.acquire() {
            Ok(_) => None,
            Err(wait_time) => {
                if self.zone.nodelay {
                    Some(wait_time)
                } else {
                    // Queueing implies we wait. Since returning wait duration simulates it cleanly,
                    // we return the wait time, and the caller can do `tokio::time::sleep(wait_time).await` if not nodelay.
                    // But if it exceeds burst limit, we'd hard reject. TokenBucket handles capacity via burst.
                    Some(wait_time)
                }
            }
        }
    }
}

pub fn create_429_response<B>(retry_after: Duration) -> Response<B>
where
    B: From<String>,
{
    let body = B::from("Rate Limit Exceeded".to_string());
    let mut resp = Response::new(body);
    *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;

    // Retry-After could be in seconds
    let secs = if retry_after.as_secs() == 0 {
        1
    } else {
        retry_after.as_secs()
    };
    resp.headers_mut()
        .insert(RETRY_AFTER, hyper::header::HeaderValue::from(secs));

    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_token_bucket() {
        // Capacity 5, rate 10 req/s.
        let mut bucket = TokenBucket::new(5, 10.0);

        // Should be able to acquire 5 immediately
        assert!(bucket.acquire().is_ok());
        assert!(bucket.acquire().is_ok());
        assert!(bucket.acquire().is_ok());
        assert!(bucket.acquire().is_ok());
        assert!(bucket.acquire().is_ok());

        // 6th should fail
        let result = bucket.acquire();
        assert!(result.is_err());
        let wait_time = result.unwrap_err();
        assert!(wait_time > Duration::from_secs(0));

        // Sleep for 150ms -> should refill ~1.5 tokens
        sleep(Duration::from_millis(150));

        // Should succeed for 1
        assert!(bucket.acquire().is_ok());
        // Might fail for the next one depending on timing, but likely fail
        assert!(bucket.acquire().is_err());
    }

    #[tokio::test]
    async fn test_bucket_manager() {
        let zone = RateLimitZone {
            name: "api_limit".to_string(),
            key_type: "$remote_addr".to_string(),
            rate_per_second: 5.0,
            burst: 2,
            nodelay: true,
        };

        let manager = BucketManager::new(zone);
        let key = "192.168.1.1";

        assert!(manager.check_limit(key).await.is_none()); // Success
        assert!(manager.check_limit(key).await.is_none()); // Success

        // 3rd should fail (burst limit 2)
        let wait = manager.check_limit(key).await;
        assert!(wait.is_some());
    }

    #[test]
    fn test_429_generation() {
        let resp: Response<String> = create_429_response(Duration::from_secs(5));
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(resp.headers().get(RETRY_AFTER).unwrap(), "5");
    }
}
