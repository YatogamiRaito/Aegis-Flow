use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Error rate too high, rejecting
    HalfOpen, // Testing if the upstream recovered
}

pub struct CircuitBreaker {
    pub state: CircuitState,
    pub error_threshold_percent: u8,
    pub window_size: Duration,
    pub open_time: Duration,
    
    pub total_requests: u64,
    pub total_errors: u64,
    pub last_state_change: Instant,
    pub window_start: Instant,
}

impl CircuitBreaker {
    pub fn new(error_threshold_percent: u8, window_size_ms: u64, open_time_ms: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            error_threshold_percent,
            window_size: Duration::from_millis(window_size_ms),
            open_time: Duration::from_millis(open_time_ms),
            total_requests: 0,
            total_errors: 0,
            last_state_change: Instant::now(),
            window_start: Instant::now(),
        }
    }

    pub fn acquire(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= self.window_size {
            // Reset sliding window
            self.total_requests = 0;
            self.total_errors = 0;
            self.window_start = now;
        }

        match self.state {
            CircuitState::Closed => true, // Allowed
            CircuitState::Open => {
                // Check if open_time has passed
                if now.duration_since(self.last_state_change) >= self.open_time {
                    self.state = CircuitState::HalfOpen;
                    self.last_state_change = now;
                    true // Allow a single probe request
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => false, // Only 1 concurrent probe allowed, block others
        }
    }

    pub fn record_success(&mut self) {
        if self.state == CircuitState::HalfOpen {
            self.state = CircuitState::Closed;
            self.last_state_change = Instant::now();
            self.total_requests = 0;
            self.total_errors = 0;
            self.window_start = Instant::now();
        } else {
            self.total_requests += 1;
        }
    }

    pub fn record_failure(&mut self) {
        if self.state == CircuitState::HalfOpen {
            self.state = CircuitState::Open;
            self.last_state_change = Instant::now();
            return;
        }

        self.total_requests += 1;
        self.total_errors += 1;

        if self.total_requests > 5 { // Arbitrary min request count to compute %
            let err_rate = (self.total_errors * 100) / self.total_requests;
            if err_rate >= (self.error_threshold_percent as u64) {
                self.state = CircuitState::Open;
                self.last_state_change = Instant::now();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_transitions() {
        // threshold 50%, window 10s, open 5s
        let mut cb = CircuitBreaker::new(50, 10000, 5000);
        
        assert_eq!(cb.state, CircuitState::Closed);
        
        // 4 successes
        for _ in 0..4 {
            assert!(cb.acquire());
            cb.record_success();
        }
        
        // 4 failures (total requests > 5 now, error rate 50%)
        for _ in 0..4 {
            assert!(cb.acquire());
            cb.record_failure();
        }
        
        assert_eq!(cb.state, CircuitState::Open);
        assert!(!cb.acquire()); // Blocked

        // Simulate time passing (5s) for HalfOpen
        cb.last_state_change = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();
        
        assert!(cb.acquire()); // Should allow probe
        assert_eq!(cb.state, CircuitState::HalfOpen);
        assert!(!cb.acquire()); // Shouldn't allow second probe

        // Probe fails -> Open
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);

        // Wait another 5s
        cb.last_state_change = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();
        assert!(cb.acquire()); // Probe
        assert_eq!(cb.state, CircuitState::HalfOpen);

        // Probe succeeds -> Closed
        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
        assert!(cb.acquire()); // Fully open again
    }
}
