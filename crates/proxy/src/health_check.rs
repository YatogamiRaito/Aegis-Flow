use crate::upstream::HealthCheckConfig;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Debug)]
pub struct ServerHealthState {
    pub addr: String,
    pub config: HealthCheckConfig,
    pub status: HealthStatus,
    pub consecutive_successes: u32,
    pub consecutive_failures: u32,
    pub passive_failures: u32,
}

impl ServerHealthState {
    pub fn new(addr: String, config: HealthCheckConfig) -> Self {
        Self {
            addr,
            config,
            status: HealthStatus::Unknown,
            consecutive_successes: 0,
            consecutive_failures: 0,
            passive_failures: 0,
        }
    }

    pub fn record_success(&mut self) -> bool {
        self.consecutive_failures = 0;
        self.consecutive_successes += 1;
        self.passive_failures = 0;

        if self.consecutive_successes >= self.config.healthy_threshold
            && self.status != HealthStatus::Healthy
        {
            self.status = HealthStatus::Healthy;
            return true; // Status changed
        }
        false
    }

    pub fn record_failure(&mut self) -> bool {
        self.consecutive_successes = 0;
        self.consecutive_failures += 1;

        if self.consecutive_failures >= self.config.unhealthy_threshold
            && self.status != HealthStatus::Unhealthy
        {
            self.status = HealthStatus::Unhealthy;
            return true; // Status changed
        }
        false
    }

    pub fn record_passive_failure(&mut self, max_fails: u32) -> bool {
        self.consecutive_successes = 0;
        self.passive_failures += 1;

        if self.passive_failures >= max_fails && self.status != HealthStatus::Unhealthy {
            self.status = HealthStatus::Unhealthy;
            return true;
        }
        false
    }
}

pub async fn perform_health_check(addr: &str, path: &str) -> bool {
    if let Ok(stream) = TcpStream::connect(addr).await {
        let io = TokioIo::new(stream);
        let (mut sender, conn) = match http1::Builder::new().handshake(io).await {
            Ok(c) => c,
            Err(_) => return false,
        };

        tokio::spawn(async move {
            let _ = conn.await;
        });

        let req = hyper::Request::builder()
            .uri(path)
            .header("Host", addr)
            .body(http_body_util::Empty::<hyper::body::Bytes>::new())
            .unwrap();

        if let Ok(res) = sender.send_request(req).await {
            let status = res.status();
            return status.is_success() || status.is_redirection();
        }
    }
    false
}

pub fn start_active_health_checks(
    addr: String,
    config: HealthCheckConfig,
    mut status_rx: mpsc::Receiver<()>, // dummy receiver to allow graceful shutdown
) -> mpsc::Receiver<HealthStatus> {
    let (tx, rx) = mpsc::channel(1);
    let mut state = ServerHealthState::new(addr.clone(), config);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_millis(state.config.interval_ms)) => {
                    let addr_clone = addr.clone();
                    let path_clone = state.config.path.clone();
                    
                    let timeout_res = timeout(
                        Duration::from_millis(state.config.timeout_ms),
                        perform_health_check(&addr_clone, &path_clone)
                    ).await;

                    let success = match timeout_res {
                        Ok(res) => res,
                        Err(_) => false, // timeout
                    };

                    let changed = if success {
                        state.record_success()
                    } else {
                        state.record_failure()
                    };

                    if changed {
                        let _ = tx.send(state.status.clone()).await;
                    }
                }
                _ = status_rx.recv() => {
                    break;
                }
            }
        }
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_health_state() {
        let config = HealthCheckConfig {
            interval_ms: 1000,
            timeout_ms: 500,
            healthy_threshold: 2,
            unhealthy_threshold: 3,
            path: "/health".to_string(),
        };

        let mut state = ServerHealthState::new("127.0.0.1:80".to_string(), config);
        
        // 1st failure (0 -> 1) -> No change (Unknown)
        assert!(!state.record_failure());
        assert_eq!(state.status, HealthStatus::Unknown);
        assert_eq!(state.consecutive_failures, 1);
        
        // 2nd failure (1 -> 2) -> No change (Unknown)
        assert!(!state.record_failure());
        assert_eq!(state.status, HealthStatus::Unknown);
        assert_eq!(state.consecutive_failures, 2);

        // 3rd failure (2 -> 3) -> Reached threshold, changed to Unhealthy!
        assert!(state.record_failure());
        assert_eq!(state.status, HealthStatus::Unhealthy);

        // 4th failure (3 -> 4) -> No change (Already Unhealthy)
        assert!(!state.record_failure());
        
        // 1st success (4 failures -> 1 success, resets failure counter) -> No change
        assert!(!state.record_success());
        assert_eq!(state.status, HealthStatus::Unhealthy);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.consecutive_successes, 1);

        // 2nd success (1 -> 2 successes) -> Reached healthy threshold, changed to Healthy!
        assert!(state.record_success());
        assert_eq!(state.status, HealthStatus::Healthy);
    }
    
    #[test]
    fn test_passive_health_checks() {
        let config = HealthCheckConfig {
            interval_ms: 1000,
            timeout_ms: 500,
            healthy_threshold: 2,
            unhealthy_threshold: 3,
            path: "/health".to_string(),
        };

        let mut state = ServerHealthState::new("127.0.0.1:80".to_string(), config);
        
        let max_fails = 2;
        assert!(!state.record_passive_failure(max_fails));
        assert!(state.record_passive_failure(max_fails)); // 2 passive failures == Unhealthy
        assert_eq!(state.status, HealthStatus::Unhealthy);
    }
}
