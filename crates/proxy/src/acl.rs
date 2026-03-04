use ipnetwork::IpNetwork;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AclAction {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct AclRule {
    pub network: Option<IpNetwork>, // None means "all"
    pub action: AclAction,
}

pub struct AclEngine {
    rules: Vec<AclRule>,
}

impl AclEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(
        &mut self,
        cidr: &str,
        action: AclAction,
    ) -> Result<(), ipnetwork::IpNetworkError> {
        if cidr == "all" {
            self.rules.push(AclRule {
                network: None,
                action,
            });
            return Ok(());
        }

        // Try parsing directly as IpNetwork, which also supports just IP (e.g., 10.0.0.1 -> 10.0.0.1/32)
        let network: IpNetwork = cidr.parse()?;
        self.rules.push(AclRule {
            network: Some(network),
            action,
        });

        Ok(())
    }

    pub fn check_ip(&self, ip: IpAddr) -> AclAction {
        for rule in &self.rules {
            match &rule.network {
                Some(net) => {
                    if net.contains(ip) {
                        return rule.action.clone();
                    }
                }
                None => {
                    // "all"
                    return rule.action.clone();
                }
            }
        }
        // Implicit deny if no rules matched
        AclAction::Deny
    }
}

pub fn check_access(acl: Option<&AclEngine>, ip: IpAddr) -> bool {
    if let Some(engine) = acl {
        engine.check_ip(ip) == AclAction::Allow
    } else {
        true // No ACL = everything allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acl_engine() {
        let mut engine = AclEngine::new();

        // allow 192.168.1.0/24;
        engine.add_rule("192.168.1.0/24", AclAction::Allow).unwrap();
        // deny 10.0.0.1;
        engine.add_rule("10.0.0.1", AclAction::Deny).unwrap();
        // allow 10.0.0.0/8;
        engine.add_rule("10.0.0.0/8", AclAction::Allow).unwrap();
        // deny all;
        engine.add_rule("all", AclAction::Deny).unwrap();

        // 192.168.1.50 -> matches 1st rule -> Allow
        assert_eq!(
            engine.check_ip("192.168.1.50".parse().unwrap()),
            AclAction::Allow
        );

        // 10.0.0.1 -> matches 2nd rule -> Deny (first match wins)
        assert_eq!(
            engine.check_ip("10.0.0.1".parse().unwrap()),
            AclAction::Deny
        );

        // 10.0.0.2 -> matches 3rd rule -> Allow
        assert_eq!(
            engine.check_ip("10.0.0.2".parse().unwrap()),
            AclAction::Allow
        );

        // 8.8.8.8 -> matches 4th rule (all) -> Deny
        assert_eq!(engine.check_ip("8.8.8.8".parse().unwrap()), AclAction::Deny);
    }

    #[test]
    fn test_implicit_deny() {
        let mut engine = AclEngine::new();
        engine.add_rule("192.168.1.0/24", AclAction::Allow).unwrap();

        assert_eq!(
            engine.check_ip("10.0.0.1".parse().unwrap()),
            AclAction::Deny
        );
    }

    #[test]
    fn test_ipv6() {
        let mut engine = AclEngine::new();
        engine.add_rule("2001:0db8::/32", AclAction::Allow).unwrap();

        assert_eq!(
            engine.check_ip("2001:0db8:85a3::8a2e:0370:7334".parse().unwrap()),
            AclAction::Allow
        );
        assert_eq!(
            engine.check_ip("2002:0db8::1".parse().unwrap()),
            AclAction::Deny
        );
    }
}
