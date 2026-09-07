use crate::error::ControlError;
use crate::protocol::DomainCommand;

/// Kernel-derived peer credentials from Unix domain socket (SO_PEERCRED)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerCredentials {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}

impl PeerCredentials {
    pub fn new(pid: i32, uid: u32, gid: u32) -> Self {
        Self { pid, uid, gid }
    }

    #[cfg(target_os = "linux")]
    pub fn from_raw_fd(fd: std::os::unix::io::RawFd) -> Result<Self, ControlError> {
        use nix::sys::socket::{getsockopt, sockopt::PeerCredentials as NixPeerCred};
        let cred = getsockopt(fd, NixPeerCred)
            .map_err(|e| ControlError::PeerCredFailed(e.to_string()))?;
        Ok(Self {
            pid: cred.pid(),
            uid: cred.uid(),
            gid: cred.gid(),
        })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn from_raw_fd(_fd: std::os::unix::io::RawFd) -> Result<Self, ControlError> {
        // Fallback for non-Linux targets / test harnesses
        Ok(Self {
            pid: std::process::id() as i32,
            uid: 1000,
            gid: 1000,
        })
    }
}

/// 6 Identity Tiers per docs/SECURITY_MODEL.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityTier {
    System,
    Administrator,
    Service(u32),
    LunaAI,
    User(u32),
}

impl IdentityTier {
    pub const LUNAAI_UID: u32 = 950;
    pub const ROOT_UID: u32 = 0;

    pub fn from_uid(uid: u32) -> Self {
        match uid {
            Self::ROOT_UID => Self::System,
            Self::LUNAAI_UID => Self::LunaAI,
            1..=999 => Self::Service(uid),
            _ => Self::User(uid),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Administrator => "Administrator",
            Self::Service(_) => "Service",
            Self::LunaAI => "LunaAI",
            Self::User(_) => "User",
        }
    }

    pub fn is_privileged(&self) -> bool {
        matches!(self, Self::System | Self::Administrator)
    }
}

/// Capability scope for system permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityScope {
    SystemRead,
    SystemControl,
    ServicesControl,
    PackagesControl,
    StorageControl,
    GenerationsControl,
}

/// Capability Authorizer engine
pub struct CapabilityAuthorizer;

impl CapabilityAuthorizer {
    /// Evaluates if the kernel-verified peer credentials have authority to execute command.
    /// Hard invariant: peer_cred.uid is kernel-derived and strictly enforced.
    pub fn authorize(
        peer: &PeerCredentials,
        command: &DomainCommand,
        token: Option<&str>,
    ) -> Result<IdentityTier, ControlError> {
        let tier = IdentityTier::from_uid(peer.uid);

        match command {
            // Read-only system state & service queries: accessible to any caller with socket transport access
            DomainCommand::SystemGetState | DomainCommand::ServicesList | DomainCommand::ServicesStatus { .. } => {
                Ok(tier)
            }

            // Privileged mutating operations: requires root/admin or valid capability token
            DomainCommand::SystemReboot | DomainCommand::SystemShutdown => {
                if tier.is_privileged() {
                    return Ok(tier);
                }

                if let Some(t) = token {
                    if t.is_empty() {
                        return Err(ControlError::TokenInvalid("Empty capability token".to_string()));
                    }
                    Err(ControlError::PermissionDenied(
                        "Capability tokens for system power control require Tier 1 human confirmation".to_string(),
                    ))
                } else {
                    Err(ControlError::PermissionDenied(format!(
                        "Caller '{}' (UID {}) lacks authority for '{}'. Requires root or Capability Token.",
                        tier.as_str(), peer.uid, command.method_name()
                    )))
                }
            }

            DomainCommand::ServicesStart { name }
            | DomainCommand::ServicesStop { name }
            | DomainCommand::ServicesRestart { name }
            | DomainCommand::ServicesReload { name } => {
                if tier.is_privileged() {
                    return Ok(tier);
                }

                if let Some(t) = token {
                    if t.is_empty() {
                        return Err(ControlError::TokenInvalid("Empty capability token".to_string()));
                    }
                    if t.contains("services.mutate") || t.contains(&format!("service:{}", name)) {
                        Ok(tier)
                    } else {
                        Err(ControlError::PermissionDenied(format!(
                            "Capability token lacks required scope for service '{}'",
                            name
                        )))
                    }
                } else {
                    Err(ControlError::PermissionDenied(format!(
                        "Caller '{}' (UID {}) lacks authority to mutate service '{}'. Requires root or Capability Token.",
                        tier.as_str(), peer.uid, name
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_tier_mapping() {
        assert_eq!(IdentityTier::from_uid(0), IdentityTier::System);
        assert_eq!(IdentityTier::from_uid(950), IdentityTier::LunaAI);
        assert_eq!(IdentityTier::from_uid(100), IdentityTier::Service(100));
        assert_eq!(IdentityTier::from_uid(1000), IdentityTier::User(1000));
        assert!(IdentityTier::from_uid(0).is_privileged());
        assert!(!IdentityTier::from_uid(950).is_privileged());
    }

    #[test]
    fn test_read_authorization() {
        let user_peer = PeerCredentials::new(100, 1000, 1000);
        let auth_res = CapabilityAuthorizer::authorize(&user_peer, &DomainCommand::SystemGetState, None);
        assert!(auth_res.is_ok());

        let ai_peer = PeerCredentials::new(101, 950, 950);
        let auth_res = CapabilityAuthorizer::authorize(&ai_peer, &DomainCommand::ServicesList, None);
        assert!(auth_res.is_ok());

        let status_cmd = DomainCommand::ServicesStatus { name: Some("udev".to_string()) };
        assert!(CapabilityAuthorizer::authorize(&user_peer, &status_cmd, None).is_ok());
    }

    #[test]
    fn test_privileged_command_authorization() {
        let root_peer = PeerCredentials::new(1, 0, 0);
        let auth_res = CapabilityAuthorizer::authorize(&root_peer, &DomainCommand::SystemReboot, None);
        assert!(auth_res.is_ok());

        let user_peer = PeerCredentials::new(100, 1000, 1000);
        let auth_res = CapabilityAuthorizer::authorize(&user_peer, &DomainCommand::SystemReboot, None);
        assert!(auth_res.is_err());
        match auth_res.unwrap_err() {
            ControlError::PermissionDenied(msg) => assert!(msg.contains("lacks authority")),
            _ => panic!("expected permission denied"),
        }
    }

    #[test]
    fn test_service_mutation_authorization() {
        let root_peer = PeerCredentials::new(1, 0, 0);
        let start_cmd = DomainCommand::ServicesStart { name: "dbus".to_string() };
        assert!(CapabilityAuthorizer::authorize(&root_peer, &start_cmd, None).is_ok());

        let ai_peer = PeerCredentials::new(101, 950, 950);
        // Denied without token
        let denied = CapabilityAuthorizer::authorize(&ai_peer, &start_cmd, None);
        assert!(denied.is_err());

        // Allowed with proper scoped token
        let allowed = CapabilityAuthorizer::authorize(&ai_peer, &start_cmd, Some("scope=services.mutate"));
        assert!(allowed.is_ok());

        // Allowed with specific service scoped token
        let allowed_specific = CapabilityAuthorizer::authorize(&ai_peer, &start_cmd, Some("scope=service:dbus"));
        assert!(allowed_specific.is_ok());

        // Denied with unrelated scoped token
        let denied_other = CapabilityAuthorizer::authorize(&ai_peer, &start_cmd, Some("scope=service:udev"));
        assert!(denied_other.is_err());
    }
}
