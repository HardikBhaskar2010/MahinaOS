use crate::audit::AuditLogger;
use crate::auth::{CapabilityAuthorizer, PeerCredentials};
use crate::error::ControlError;
use crate::idempotency::IdempotencyStore;
use crate::protocol::{DomainCommand, Request, Response};
use crate::providers::ProviderDispatcher;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const DEFAULT_SOCKET_PATH: &str = "/run/mahina/control.sock";

pub struct ControlDaemon {
    socket_path: PathBuf,
    dispatcher: Arc<ProviderDispatcher>,
    idempotency: Arc<IdempotencyStore>,
    audit: Arc<AuditLogger>,
}

impl ControlDaemon {
    pub fn new(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            dispatcher: Arc::new(ProviderDispatcher::new()),
            idempotency: Arc::new(IdempotencyStore::new()),
            audit: Arc::new(AuditLogger::new()),
        }
    }

    pub fn with_components(
        socket_path: impl AsRef<Path>,
        dispatcher: Arc<ProviderDispatcher>,
        idempotency: Arc<IdempotencyStore>,
        audit: Arc<AuditLogger>,
    ) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            dispatcher,
            idempotency,
            audit,
        }
    }

    /// Process a single raw JSON request line from a peer with kernel-verified credentials
    pub fn process_request(&self, peer: &PeerCredentials, raw_line: &str) -> Response {
        let req: Request = match serde_json::from_str(raw_line.trim()) {
            Ok(r) => r,
            Err(e) => {
                self.audit.log(
                    None,
                    peer.uid,
                    peer.pid,
                    "Unknown",
                    "invalid.json",
                    false,
                    false,
                    Some(e.to_string()),
                );
                return Response::error(0, 400, format!("Malformed JSON request: {}", e));
            }
        };

        // Check idempotency cache for duplicate requests
        if let Some(ref req_id) = req.request_id {
            if let Some(cached_resp) = self.idempotency.get(req_id) {
                return cached_resp;
            }
        }

        // Parse strongly typed domain command
        let cmd = match DomainCommand::parse(&req.method, &req.params) {
            Ok(c) => c,
            Err(e) => {
                self.audit.log(
                    req.request_id.clone(),
                    peer.uid,
                    peer.pid,
                    "Unknown",
                    &req.method,
                    false,
                    false,
                    Some(e.to_string()),
                );
                return Response::error(req.id, 404, e.to_string());
            }
        };

        // Authorize caller using kernel-derived credentials
        let auth_res = CapabilityAuthorizer::authorize(
            peer,
            &cmd,
            req.capability_token.as_deref(),
        );

        let tier = match auth_res {
            Ok(t) => t,
            Err(err) => {
                self.audit.log(
                    req.request_id.clone(),
                    peer.uid,
                    peer.pid,
                    "Unauthorized",
                    cmd.method_name(),
                    false,
                    false,
                    Some(err.to_string()),
                );
                return Response::error(req.id, 403, err.to_string());
            }
        };

        // Dispatch command to domain provider
        let response = match self.dispatcher.dispatch(cmd.clone(), peer) {
            Ok(domain_res) => {
                self.audit.log(
                    req.request_id.clone(),
                    peer.uid,
                    peer.pid,
                    tier.as_str(),
                    cmd.method_name(),
                    true,
                    true,
                    None,
                );
                Response::success(req.id, domain_res.to_value())
            }
            Err(err) => {
                self.audit.log(
                    req.request_id.clone(),
                    peer.uid,
                    peer.pid,
                    tier.as_str(),
                    cmd.method_name(),
                    true,
                    false,
                    Some(err.to_string()),
                );
                Response::error(req.id, 500, err.to_string())
            }
        };

        // Store mutating request result in idempotency cache
        if cmd.is_mutating() {
            if let Some(req_id) = req.request_id {
                self.idempotency.insert(req_id, response.clone());
            }
        }

        response
    }

    /// Handles a single incoming Unix stream connection
    pub fn handle_client(&self, mut stream: UnixStream) -> Result<(), ControlError> {
        use std::os::unix::io::AsRawFd;
        let peer = PeerCredentials::from_raw_fd(stream.as_raw_fd())?;

        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();

        while reader.read_line(&mut line)? > 0 {
            if line.trim().is_empty() {
                line.clear();
                continue;
            }

            let response = self.process_request(&peer, &line);
            let mut serialized = serde_json::to_string(&response)?;
            serialized.push('\n');

            stream.write_all(serialized.as_bytes())?;
            stream.flush()?;
            line.clear();
        }

        Ok(())
    }

    /// Starts the control plane listener loop on the Unix domain socket
    pub fn listen(&self) -> Result<(), ControlError> {
        if let Some(parent) = self.socket_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        // Set restrictive socket permissions (0660)
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.socket_path, fs::Permissions::from_mode(0o660));
        }

        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let _ = self.handle_client(s);
                }
                Err(e) => {
                    eprintln!("mahina-control: connection error: {}", e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_process_request_get_state() {
        let daemon = ControlDaemon::with_components(
            "/tmp/test-ctrl.sock",
            Arc::new(ProviderDispatcher::new()),
            Arc::new(IdempotencyStore::new()),
            Arc::new(AuditLogger::in_memory()),
        );

        let peer = PeerCredentials::new(42, 1000, 1000);
        let req_json = r#"{"id":10,"method":"system.get_state"}"#;
        let resp = daemon.process_request(&peer, req_json);

        assert_eq!(resp.id, 10);
        assert!(resp.ok);
        assert!(resp.result.is_some());

        let res_val = resp.result.unwrap();
        assert!(res_val.get("hostname").is_some());
        assert!(res_val.get("active_generation").is_some());
    }

    #[test]
    fn test_daemon_unauthorized_reboot_rejected() {
        let audit = Arc::new(AuditLogger::in_memory());
        let daemon = ControlDaemon::with_components(
            "/tmp/test-ctrl.sock",
            Arc::new(ProviderDispatcher::new()),
            Arc::new(IdempotencyStore::new()),
            audit.clone(),
        );

        let unprivileged_peer = PeerCredentials::new(42, 1000, 1000);
        let req_json = r#"{"id":11,"method":"system.reboot"}"#;
        let resp = daemon.process_request(&unprivileged_peer, req_json);

        assert_eq!(resp.id, 11);
        assert!(!resp.ok);
        assert_eq!(resp.error.unwrap().code, 403);

        // Verify audit log captured rejection
        let events = audit.get_in_memory_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].authorization, "rejected");
    }

    #[test]
    fn test_daemon_idempotency_deduplication() {
        let daemon = ControlDaemon::with_components(
            "/tmp/test-ctrl.sock",
            Arc::new(ProviderDispatcher::new()),
            Arc::new(IdempotencyStore::new()),
            Arc::new(AuditLogger::in_memory()),
        );

        let root_peer = PeerCredentials::new(1, 0, 0);
        let req_json = r#"{"id":12,"method":"system.reboot","request_id":"uuid-test-1"}"#;

        let resp1 = daemon.process_request(&root_peer, req_json);
        assert!(resp1.ok);

        let resp2 = daemon.process_request(&root_peer, req_json);
        assert_eq!(resp1, resp2);
    }
}
