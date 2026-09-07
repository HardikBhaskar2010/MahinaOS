use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_AUDIT_LOG_PATH: &str = "/var/log/mahina/control-audit.log";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub timestamp_epoch_secs: u64,
    pub request_id: Option<String>,
    pub caller_uid: u32,
    pub caller_pid: i32,
    pub identity_tier: String,
    pub operation: String,
    pub authorization: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct AuditLogger {
    log_file_path: Option<PathBuf>,
    in_memory: Mutex<Vec<AuditEvent>>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            log_file_path: Some(PathBuf::from(DEFAULT_AUDIT_LOG_PATH)),
            in_memory: Mutex::new(Vec::new()),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            log_file_path: None,
            in_memory: Mutex::new(Vec::new()),
        }
    }

    pub fn with_file_path(path: impl AsRef<Path>) -> Self {
        Self {
            log_file_path: Some(path.as_ref().to_path_buf()),
            in_memory: Mutex::new(Vec::new()),
        }
    }

    pub fn log(
        &self,
        request_id: Option<String>,
        caller_uid: u32,
        caller_pid: i32,
        identity_tier: &str,
        operation: &str,
        authorized: bool,
        result_ok: bool,
        error_msg: Option<String>,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let event = AuditEvent {
            timestamp_epoch_secs: now,
            request_id,
            caller_uid,
            caller_pid,
            identity_tier: identity_tier.to_string(),
            operation: operation.to_string(),
            authorization: if authorized { "approved" } else { "rejected" }.to_string(),
            result: if result_ok { "success" } else { "error" }.to_string(),
            error: error_msg,
        };

        if let Ok(mut mem) = self.in_memory.lock() {
            mem.push(event.clone());
        }

        if let Some(ref path) = self.log_file_path {
            if let Ok(json_line) = serde_json::to_string(&event) {
                if let Some(parent) = path.parent() {
                    let _ = create_dir_all(parent);
                }
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                    let _ = writeln!(file, "{}", json_line);
                }
            }
        }
    }

    pub fn get_in_memory_events(&self) -> Vec<AuditEvent> {
        self.in_memory.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logger_in_memory() {
        let logger = AuditLogger::in_memory();
        logger.log(
            Some("req-uuid-1".to_string()),
            950,
            120,
            "LunaAI",
            "system.get_state",
            true,
            true,
            None,
        );

        let events = logger.get_in_memory_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].caller_uid, 950);
        assert_eq!(events[0].identity_tier, "LunaAI");
        assert_eq!(events[0].operation, "system.get_state");
        assert_eq!(events[0].authorization, "approved");
        assert_eq!(events[0].result, "success");
    }
}
