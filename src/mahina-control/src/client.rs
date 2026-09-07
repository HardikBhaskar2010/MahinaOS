use crate::daemon::DEFAULT_SOCKET_PATH;
use crate::error::ControlError;
use crate::protocol::{
    PackageEntry, Request, Response, ServiceEntry, ServiceStatus, StorageOverview, SystemState,
    UserEntry,
};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

pub struct ControlClient {
    _socket_path: PathBuf,
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    next_id: u64,
}

impl ControlClient {
    pub fn connect(socket_path: impl AsRef<Path>) -> Result<Self, ControlError> {
        let stream = UnixStream::connect(&socket_path)
            .map_err(|e| ControlError::Io(format!("Failed to connect to control socket '{:?}': {}", socket_path.as_ref(), e)))?;
        let reader = BufReader::new(stream.try_clone()?);

        Ok(Self {
            _socket_path: socket_path.as_ref().to_path_buf(),
            stream,
            reader,
            next_id: 1,
        })
    }

    pub fn default_connect() -> Result<Self, ControlError> {
        Self::connect(DEFAULT_SOCKET_PATH)
    }

    pub fn call(
        &mut self,
        method: &str,
        params: serde_json::Value,
        request_id: Option<String>,
        capability_token: Option<String>,
    ) -> Result<serde_json::Value, ControlError> {
        let id = self.next_id;
        self.next_id += 1;

        let req = Request {
            id,
            method: method.to_string(),
            params,
            request_id,
            capability_token,
        };

        let mut raw_line = serde_json::to_string(&req)?;
        raw_line.push('\n');

        self.stream.write_all(raw_line.as_bytes())?;
        self.stream.flush()?;

        let mut resp_line = String::new();
        self.reader.read_line(&mut resp_line)?;

        let resp: Response = serde_json::from_str(resp_line.trim())?;
        if resp.ok {
            Ok(resp.result.unwrap_or(serde_json::Value::Null))
        } else {
            let err = resp.error.map(|e| e.message).unwrap_or_else(|| "Unknown error".to_string());
            Err(ControlError::Internal(err))
        }
    }

    pub fn system_get_state(&mut self) -> Result<SystemState, ControlError> {
        let val = self.call("system.get_state", serde_json::json!({}), None, None)?;
        let state: SystemState = serde_json::from_value(val)?;
        Ok(state)
    }

    pub fn system_reboot(&mut self, token: Option<String>) -> Result<(), ControlError> {
        let _ = self.call("system.reboot", serde_json::json!({}), None, token)?;
        Ok(())
    }

    pub fn system_shutdown(&mut self, token: Option<String>) -> Result<(), ControlError> {
        let _ = self.call("system.shutdown", serde_json::json!({}), None, token)?;
        Ok(())
    }

    pub fn services_list(&mut self) -> Result<Vec<ServiceEntry>, ControlError> {
        let val = self.call("services.list", serde_json::json!({}), None, None)?;
        let list: Vec<ServiceEntry> = serde_json::from_value(val)?;
        Ok(list)
    }

    pub fn services_status(&mut self, name: Option<&str>) -> Result<ServiceStatus, ControlError> {
        let params = match name {
            Some(n) => serde_json::json!({ "name": n }),
            None => serde_json::json!({}),
        };
        let val = self.call("services.status", params, None, None)?;
        let status: ServiceStatus = serde_json::from_value(val)?;
        Ok(status)
    }

    pub fn services_start(&mut self, name: &str, token: Option<String>) -> Result<String, ControlError> {
        let val = self.call(
            "services.start",
            serde_json::json!({ "name": name }),
            None,
            token,
        )?;
        let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Started").to_string();
        Ok(msg)
    }

    pub fn services_stop(&mut self, name: &str, token: Option<String>) -> Result<String, ControlError> {
        let val = self.call(
            "services.stop",
            serde_json::json!({ "name": name }),
            None,
            token,
        )?;
        let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Stopped").to_string();
        Ok(msg)
    }

    pub fn services_restart(&mut self, name: &str, token: Option<String>) -> Result<String, ControlError> {
        let val = self.call(
            "services.restart",
            serde_json::json!({ "name": name }),
            None,
            token,
        )?;
        let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Restarted").to_string();
        Ok(msg)
    }

    pub fn services_reload(&mut self, name: &str, token: Option<String>) -> Result<String, ControlError> {
        let val = self.call(
            "services.reload",
            serde_json::json!({ "name": name }),
            None,
            token,
        )?;
        let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Reloaded").to_string();
        Ok(msg)
    }

    pub fn users_list(&mut self) -> Result<Vec<UserEntry>, ControlError> {
        let val = self.call("users.list", serde_json::json!({}), None, None)?;
        let list: Vec<UserEntry> = serde_json::from_value(val)?;
        Ok(list)
    }

    pub fn packages_list(&mut self) -> Result<Vec<PackageEntry>, ControlError> {
        let val = self.call("packages.list", serde_json::json!({}), None, None)?;
        let list: Vec<PackageEntry> = serde_json::from_value(val)?;
        Ok(list)
    }

    pub fn packages_install(&mut self, target: &str, token: Option<String>) -> Result<String, ControlError> {
        let val = self.call(
            "packages.install",
            serde_json::json!({ "target": target }),
            None,
            token,
        )?;
        let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Installed").to_string();
        Ok(msg)
    }

    pub fn packages_remove(&mut self, name: &str, token: Option<String>) -> Result<String, ControlError> {
        let val = self.call(
            "packages.remove",
            serde_json::json!({ "name": name }),
            None,
            token,
        )?;
        let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Removed").to_string();
        Ok(msg)
    }

    pub fn storage_list(&mut self) -> Result<StorageOverview, ControlError> {
        let val = self.call("storage.list", serde_json::json!({}), None, None)?;
        let overview: StorageOverview = serde_json::from_value(val)?;
        Ok(overview)
    }

    pub fn storage_mount(
        &mut self,
        source: &str,
        target: &str,
        fs_type: Option<&str>,
        options: Option<&str>,
        token: Option<String>,
    ) -> Result<String, ControlError> {
        let mut params = serde_json::json!({
            "source": source,
            "target": target,
        });
        if let Some(fst) = fs_type {
            params["fs_type"] = serde_json::Value::String(fst.to_string());
        }
        if let Some(opts) = options {
            params["options"] = serde_json::Value::String(opts.to_string());
        }

        let val = self.call("storage.mount", params, None, token)?;
        let msg = val.get("message").and_then(|m| m.as_str()).unwrap_or("Mounted").to_string();
        Ok(msg)
    }
}
