use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode, header};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{error, instrument};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Satisfy mode: controls if BOTH auth_request AND ACL must pass, or just one
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SatisfyMode {
    All,
    Any,
}

impl Default for SatisfyMode {
    fn default() -> Self {
        SatisfyMode::All
    }
}

/// Cached auth decision with expiry
#[derive(Clone, Debug)]
pub struct CachedAuthDecision {
    pub allowed: bool,
    pub injected_headers: HashMap<String, String>,
    pub expires_at: Instant,
}

const AUTH_CACHE_TTL: Duration = Duration::from_secs(60);

/// Simple in-process auth cache: (cache_key → CachedAuthDecision)
static AUTH_CACHE: Lazy<Mutex<HashMap<String, CachedAuthDecision>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Build a cache key from auth URI + relevant request headers
fn build_cache_key(auth_uri: &str, auth_header: Option<&str>, cookie: Option<&str>) -> String {
    format!(
        "{}|{}|{}",
        auth_uri,
        auth_header.unwrap_or("-"),
        cookie.unwrap_or("-")
    )
}

fn cache_get(key: &str) -> Option<CachedAuthDecision> {
    let mut cache = AUTH_CACHE.lock().unwrap();
    if let Some(entry) = cache.get(key) {
        if entry.expires_at > Instant::now() {
            return Some(entry.clone());
        }
        cache.remove(key);
    }
    None
}

fn cache_insert(key: String, decision: CachedAuthDecision) {
    let mut cache = AUTH_CACHE.lock().unwrap();
    // Evict expired entries when cache grows large
    if cache.len() > 4096 {
        cache.retain(|_, v| v.expires_at > Instant::now());
    }
    cache.insert(key, decision);
}

/// Result of an auth request subrequest
pub enum AuthResult {
    /// Authentication succeeded, proxy should continue. Includes optional headers to inject.
    Allowed(HashMap<String, String>),
    /// Authentication failed, proxy should immediately return the provided blocked response.
    Denied(Response<Full<Bytes>>),
    /// Subrequest encountered a system error, fail open or closed based on configuration.
    Error(Response<Full<Bytes>>),
}

/// Perform an async subrequest to an external authentication server.
/// Corresponds to nginx `auth_request` directive.
/// Optionally caches auth decisions by (auth_uri, Authorization, Cookie) for 60s.
#[instrument(skip(req))]
pub async fn check_subrequest<B>(
    req: &Request<B>,
    auth_uri: &str,
    extract_headers: &HashMap<String, String>,
) -> AuthResult
where
    B: hyper::body::Body + Send + 'static,
{
    // --- Cache check: avoid roundtrip for recently-validated sessions ---
    let auth_header_val = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let cookie_val = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok());
    let cache_key = build_cache_key(auth_uri, auth_header_val, cookie_val);

    if let Some(cached) = cache_get(&cache_key) {
        return if cached.allowed {
            AuthResult::Allowed(cached.injected_headers.clone())
        } else {
            let resp = Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(Full::new(Bytes::from("Forbidden (cached)")))
                .unwrap();
            AuthResult::Denied(resp)
        };
    }

    // Build an HTTP client for the subrequest
    let client = reqwest::Client::new();

    // Convert relative or absolute URI
    let target_uri = match auth_uri.parse::<reqwest::Url>() {
        Ok(uri) => uri,
        Err(e) => {
            error!("Auth request URI is invalid: {} - {}", auth_uri, e);
            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(
                    "Internal Server Error (Auth Request Configuration Invalid)",
                )))
                .unwrap();
            return AuthResult::Error(resp);
        }
    };

    // Prepare subrequest — forward headers from original request
    let mut sub_req = client.get(target_uri);
    for (k, v) in req.headers().iter() {
        if let Ok(value) = v.to_str() {
            sub_req = sub_req.header(k.as_str(), value);
        }
    }

    match sub_req.send().await {
        Ok(res) => {
            if res.status().is_success() {
                // Auth allowed (2xx code)
                let mut injected_headers = HashMap::new();
                for (var_name, header_key) in extract_headers {
                    if let Some(h_val) = res.headers().get(header_key) {
                        if let Ok(v_str) = h_val.to_str() {
                            injected_headers.insert(var_name.clone(), v_str.to_string());
                        }
                    }
                }
                // Cache the positive decision
                cache_insert(
                    cache_key,
                    CachedAuthDecision {
                        allowed: true,
                        injected_headers: injected_headers.clone(),
                        expires_at: Instant::now() + AUTH_CACHE_TTL,
                    },
                );
                AuthResult::Allowed(injected_headers)
            } else {
                // Auth denied (401, 403, etc)
                // Return proxy response exactly matching Auth Server's status
                let mut proxy_res = Response::builder().status(res.status());

                // Copy WWW-Authenticate header if present (crucial for Basic Auth flows)
                if let Some(auth_header) = res.headers().get(header::WWW_AUTHENTICATE) {
                    proxy_res = proxy_res.header(header::WWW_AUTHENTICATE, auth_header);
                }

                let body = match res.bytes().await {
                    Ok(b) => Full::new(b),
                    Err(_) => Full::new(Bytes::new()),
                };

                AuthResult::Denied(proxy_res.body(body).unwrap())
            }
        }
        Err(e) => {
            error!("Auth request subrequest failed: {}", e);
            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(
                    "Internal Server Error (Auth Request Failed)",
                )))
                .unwrap();
            AuthResult::Error(resp)
        }
    }
}
