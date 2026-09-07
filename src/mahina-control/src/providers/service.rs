use crate::error::ControlError;
use crate::protocol::{ServiceEntry, ServiceStatus};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

pub const DEFAULT_LUNA_INIT_SOCKET: &str = "/run/luna-init.sock";

#[derive(Debug, Deserialize)]
struct LunaInitResponse {
    ok: bool,
    #[serde(default)]
    services: Option<Vec<ServiceEntry>>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    pid: Option<i32>,
    #[serde(default)]
    restart_count: Option<u32>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServiceProvider {
    socket_path: PathBuf,
}

impl ServiceProvider {
    pub fn new() -> Self {
        Self {
            socket_path: PathBuf::from(DEFAULT_LUNA_INIT_SOCKET),
        }
    }

    pub fn with_socket_path(path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: path.as_ref().to_path_buf(),
        }
    }

    fn send_command(&self, cmd_json: &str) -> Result<LunaInitResponse, ControlError> {
        if !self.socket_path.exists() {
            return Err(ControlError::NotFound(format!(
                "luna-init supervisor socket not found at {:?}. Is luna-init running?",
                self.socket_path
            )));
        }

        let mut stream = UnixStream::connect(&self.socket_path).map_err(|e| {
            ControlError::Io(format!(
                "Failed to connect to luna-init socket at {:?}: {}",
                self.socket_path, e
            ))
        })?;

        stream.write_all(cmd_json.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        if line.trim().is_empty() {
            return Err(ControlError::Internal("Empty response from luna-init supervisor".to_string()));
        }

        let resp: LunaInitResponse = serde_json::from_str(line.trim())
            .map_err(|e| ControlError::Internal(format!("Malformed response from luna-init: {}", e)))?;

        if !resp.ok {
            let err_msg = resp.error.unwrap_or_else(|| "Supervisor operation failed".to_string());
            return Err(ControlError::Internal(err_msg));
        }

        Ok(resp)
    }

    pub fn list(&self) -> Result<Vec<ServiceEntry>, ControlError> {
        let resp = self.send_command(r#"{"cmd":"list"}"#)?;
        Ok(resp.services.unwrap_or_default())
    }

    pub fn status(&self, name: Option<&str>) -> Result<ServiceStatus, ControlError> {
        let cmd = match name {
            Some(n) => format!(r#"{{"cmd":"status","name":"{}"}}"#, n),
            None => r#"{"cmd":"status"}"#.to_string(),
        };

        let resp = self.send_command(&cmd)?;
        Ok(ServiceStatus {
            name: resp.name.unwrap_or_else(|| name.unwrap_or("system").to_string()),
            state: resp.state.unwrap_or_else(|| "UNKNOWN".to_string()),
            pid: resp.pid.unwrap_or(0),
            restart_count: resp.restart_count.unwrap_or(0),
        })
    }

    pub fn start(&self, name: &str) -> Result<(), ControlError> {
        let cmd = format!(r#"{{"cmd":"start","name":"{}"}}"#, name);
        self.send_command(&cmd)?;
        Ok(())
    }

    pub fn stop(&self, name: &str) -> Result<(), ControlError> {
        let cmd = format!(r#"{{"cmd":"stop","name":"{}"}}"#, name);
        self.send_command(&cmd)?;
        Ok(())
    }

    pub fn restart(&self, name: &str) -> Result<(), ControlError> {
        let cmd = format!(r#"{{"cmd":"restart","name":"{}"}}"#, name);
        self.send_command(&cmd)?;
        Ok(())
    }

    pub fn reload(&self, name: &str) -> Result<(), ControlError> {
        let cmd = format!(r#"{{"cmd":"reload","name":"{}"}}"#, name);
        self.send_command(&cmd)?;
        Ok(())
    }
}

impl Default for ServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;

    #[test]
    fn test_service_provider_socket_missing() {
        let provider = ServiceProvider::with_socket_path("/tmp/nonexistent-luna-init.sock");
        let res = provider.list();
        assert!(res.is_err());
        match res.unwrap_err() {
            ControlError::NotFound(msg) => assert!(msg.contains("not found")),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_service_provider_mock_interaction() {
        let socket_path = "/tmp/test-luna-init-mock.sock";
        let _ = std::fs::remove_file(socket_path);

        let listener = UnixListener::bind(socket_path).expect("failed to bind mock listener");

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept failed");
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();

            if line.contains("list") {
                let resp = r#"{"ok":true,"services":[{"name":"udev","state":"RUNNING","pid":120},{"name":"dbus","state":"RUNNING","pid":135}]}"#;
                stream.write_all(resp.as_bytes()).unwrap();
                stream.write_all(b"\n").unwrap();
            }
        });

        let provider = ServiceProvider::with_socket_path(socket_path);
        let list = provider.list().expect("list failed");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "udev");
        assert_eq!(list[0].state, "RUNNING");
        assert_eq!(list[0].pid, 120);

        handle.join().unwrap();
        let _ = std::fs::remove_file(socket_path);
    }
}
