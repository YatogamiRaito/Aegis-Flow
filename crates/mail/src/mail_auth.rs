use reqwest::Client;
/// Mail Auth HTTP Protocol implementation
/// Handles the nginx mail_auth_http protocol where the proxy sends
/// an HTTP request to an auth service to get the backend server address.
use std::collections::HashMap;
use tracing::{debug, error};

/// Information about how the client is authenticating
#[derive(Debug, Clone)]
pub struct MailAuthRequest {
    pub protocol: String, // smtp, imap, pop3
    pub method: String,   // plain, login, cram-md5
    pub user: String,
    pub pass: String,
    pub client_ip: String,
    pub client_host: Option<String>,
}

/// Successful auth response from the auth service
#[derive(Debug, Clone)]
pub struct MailAuthResponse {
    pub server: String,
    pub port: u16,
    pub user: Option<String>, // optionally rewritten username
}

/// Failed auth response
#[derive(Debug, Clone)]
pub struct MailAuthError {
    pub message: String,            // e.g. "Invalid credentials"
    pub error_code: Option<String>, // e.g. "535 5.7.8"
    pub wait_secs: u32,             // seconds before client can retry
}

/// Possible outcomes of an auth HTTP call
#[derive(Debug)]
pub enum MailAuthResult {
    Ok(MailAuthResponse),
    Denied(MailAuthError),
    Error(String),
}

/// Send the mail auth HTTP request to the configured auth service.
/// This implements the nginx mail_auth_http protocol.
pub async fn authenticate(auth_url: &str, req: &MailAuthRequest) -> MailAuthResult {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    debug!("Mail auth request to {} for user={}", auth_url, req.user);

    let mut headers = HashMap::new();
    headers.insert("Auth-Method", req.method.clone());
    headers.insert("Auth-User", req.user.clone());
    headers.insert("Auth-Pass", req.pass.clone());
    headers.insert("Auth-Protocol", req.protocol.clone());
    headers.insert("Client-IP", req.client_ip.clone());
    if let Some(ref host) = req.client_host {
        headers.insert("Client-Host", host.clone());
    }

    let mut request = client.get(auth_url);
    for (k, v) in &headers {
        request = request.header(*k, v.as_str());
    }

    match request.send().await {
        Ok(res) => {
            let status_val = res
                .headers()
                .get("Auth-Status")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("error")
                .to_string();

            if status_val == "OK" {
                let server = res
                    .headers()
                    .get("Auth-Server")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("127.0.0.1")
                    .to_string();
                let port: u16 = res
                    .headers()
                    .get("Auth-Port")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(25);
                let user = res
                    .headers()
                    .get("Auth-User")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                MailAuthResult::Ok(MailAuthResponse { server, port, user })
            } else {
                let error_code = res
                    .headers()
                    .get("Auth-Error-Code")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let wait_secs: u32 = res
                    .headers()
                    .get("Auth-Wait")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                MailAuthResult::Denied(MailAuthError {
                    message: status_val,
                    error_code,
                    wait_secs,
                })
            }
        }
        Err(e) => {
            error!("Mail auth HTTP request failed: {}", e);
            MailAuthResult::Error(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_auth_request_fields() {
        let req = MailAuthRequest {
            protocol: "smtp".to_string(),
            method: "plain".to_string(),
            user: "user@example.com".to_string(),
            pass: "secret".to_string(),
            client_ip: "1.2.3.4".to_string(),
            client_host: Some("mail.example.com".to_string()),
        };
        assert_eq!(req.protocol, "smtp");
        assert_eq!(req.user, "user@example.com");
        assert_eq!(req.client_host, Some("mail.example.com".to_string()));
    }

    #[test]
    fn test_mail_auth_response() {
        let resp = MailAuthResponse {
            server: "10.0.0.5".to_string(),
            port: 25,
            user: None,
        };
        assert_eq!(resp.server, "10.0.0.5");
        assert_eq!(resp.port, 25);
    }

    #[test]
    fn test_mail_auth_error() {
        let err = MailAuthError {
            message: "Invalid credentials".to_string(),
            error_code: Some("535 5.7.8".to_string()),
            wait_secs: 3,
        };
        assert_eq!(err.wait_secs, 3);
        assert!(err.error_code.is_some());
    }
}
