use crate::upstream::{LoadBalanceStrategy, UpstreamServer};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Debug)]
pub struct RuntimeServer {
    pub config: UpstreamServer,
    pub active_connections: AtomicU64,
}

impl RuntimeServer {
    pub fn new(config: UpstreamServer) -> Self {
        Self {
            config,
            active_connections: AtomicU64::new(0),
        }
    }
}

pub struct LoadBalancer {
    pub strategy: LoadBalanceStrategy,
    pub servers: Vec<RuntimeServer>,
    rr_counter: AtomicUsize,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalanceStrategy, configs: Vec<UpstreamServer>) -> Self {
        let servers = configs.into_iter().map(RuntimeServer::new).collect();
        Self {
            strategy,
            servers,
            rr_counter: AtomicUsize::new(0),
        }
    }

    fn available_servers(&self) -> Vec<&RuntimeServer> {
        let mut primaries = Vec::new();
        let mut backups = Vec::new();

        for s in &self.servers {
            if s.config.down {
                continue;
            }
            if s.config.backup {
                backups.push(s);
            } else {
                primaries.push(s);
            }
        }

        if primaries.is_empty() {
            backups
        } else {
            primaries
        }
    }

    pub fn select_server(&self, hash_key: Option<&str>) -> Option<&RuntimeServer> {
        let available = self.available_servers();
        if available.is_empty() {
            return None;
        }

        match &self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                // Weighted Round Robin
                let total_weight: u32 = available.iter().map(|s| s.config.weight).sum();
                if total_weight == 0 {
                    return None;
                }

                let idx = self.rr_counter.fetch_add(1, Ordering::Relaxed) as u32 % total_weight;

                let mut current_weight = 0;
                for s in available {
                    current_weight += s.config.weight;
                    if idx < current_weight {
                        return Some(s);
                    }
                }
                None
            }
            LoadBalanceStrategy::LeastConnections => {
                // Least connections, tie-breaker with weight
                available.into_iter().min_by(|a, b| {
                    let conn_a = a.active_connections.load(Ordering::Relaxed);
                    let conn_b = b.active_connections.load(Ordering::Relaxed);
                    conn_a
                        .cmp(&conn_b)
                        .then_with(|| b.config.weight.cmp(&a.config.weight))
                })
            }
            LoadBalanceStrategy::IpHash => {
                // Consistent hashing like Ketama
                let key = hash_key.unwrap_or("");
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                let hash_val = hasher.finish();

                // Simplified hash ring: directly mod total weight for steady distribution
                let total_weight: u32 = available.iter().map(|s| s.config.weight).sum();
                if total_weight == 0 {
                    return None;
                }

                let idx = (hash_val % (total_weight as u64)) as u32;
                let mut current_weight = 0;
                for s in available {
                    current_weight += s.config.weight;
                    if idx < current_weight {
                        return Some(s);
                    }
                }
                None
            }
            LoadBalanceStrategy::GenericHash(_var_name) => {
                let key = hash_key.unwrap_or("");
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                let hash_val = hasher.finish();
                let idx = (hash_val % (available.len() as u64)) as usize;
                Some(available[idx])
            }
            LoadBalanceStrategy::PowerOfTwoChoices => {
                if available.len() == 1 {
                    return Some(available[0]);
                }

                // Extract P2C key or use rr_counter for pseudo-random
                let rand1 = self.rr_counter.fetch_add(1, Ordering::Relaxed) % available.len();
                let rand2 = self.rr_counter.fetch_add(1, Ordering::Relaxed) % available.len();

                let s1 = available[rand1];
                let s2 = available[rand2];

                let conn1 = s1.active_connections.load(Ordering::Relaxed);
                let conn2 = s2.active_connections.load(Ordering::Relaxed);

                if conn1 <= conn2 { Some(s1) } else { Some(s2) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_server(addr: &str, weight: u32, down: bool, backup: bool) -> UpstreamServer {
        UpstreamServer {
            addr: addr.to_string(),
            weight,
            max_connections: None,
            backup,
            down,
        }
    }

    #[test]
    fn test_weighted_round_robin() {
        let servers = vec![
            dummy_server("s1", 3, false, false),
            dummy_server("s2", 1, false, false),
        ];

        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin, servers);

        // weights are 3 and 1, total 4.
        // indices 0,1,2 will go to s1, index 3 goes to s2
        assert_eq!(lb.select_server(None).unwrap().config.addr, "s1"); // 0
        assert_eq!(lb.select_server(None).unwrap().config.addr, "s1"); // 1
        assert_eq!(lb.select_server(None).unwrap().config.addr, "s1"); // 2
        assert_eq!(lb.select_server(None).unwrap().config.addr, "s2"); // 3

        assert_eq!(lb.select_server(None).unwrap().config.addr, "s1"); // 4
    }

    #[test]
    fn test_least_connections() {
        let servers = vec![
            dummy_server("s1", 1, false, false),
            dummy_server("s2", 1, false, false), // we will give s2 more weight later
        ];

        let mut lb = LoadBalancer::new(LoadBalanceStrategy::LeastConnections, servers);
        lb.servers[0]
            .active_connections
            .store(10, Ordering::Relaxed);
        lb.servers[1].active_connections.store(5, Ordering::Relaxed);

        assert_eq!(lb.select_server(None).unwrap().config.addr, "s2");

        // Test tie-breaking by weight (higher weight wins)
        lb.servers[0].active_connections.store(5, Ordering::Relaxed);
        lb.servers[0].config.weight = 5;
        lb.servers[1].config.weight = 1;

        assert_eq!(lb.select_server(None).unwrap().config.addr, "s1");
    }

    #[test]
    fn test_ip_hash() {
        let servers = vec![
            dummy_server("s1", 1, false, false),
            dummy_server("s2", 1, false, false),
        ];

        let lb = LoadBalancer::new(LoadBalanceStrategy::IpHash, servers);
        let s1 = lb
            .select_server(Some("192.168.1.100"))
            .unwrap()
            .config
            .addr
            .clone();
        let s2 = lb
            .select_server(Some("192.168.1.100"))
            .unwrap()
            .config
            .addr
            .clone();

        assert_eq!(s1, s2); // Always same routing
    }

    #[test]
    fn test_p2c() {
        let servers = vec![
            dummy_server("s1", 1, false, false),
            dummy_server("s2", 1, false, false),
            dummy_server("s3", 1, false, false),
        ];

        let lb = LoadBalancer::new(LoadBalanceStrategy::PowerOfTwoChoices, servers);
        // Force s1 and s2 to have high connections
        lb.servers[0]
            .active_connections
            .store(100, Ordering::Relaxed);
        lb.servers[1]
            .active_connections
            .store(100, Ordering::Relaxed);
        lb.servers[2].active_connections.store(0, Ordering::Relaxed);

        // P2C should favor s3 when it's selected
        let mut s3_chosen = false;
        for _ in 0..10 {
            if lb.select_server(None).unwrap().config.addr == "s3" {
                s3_chosen = true;
            }
        }
        assert!(s3_chosen);
    }
}
