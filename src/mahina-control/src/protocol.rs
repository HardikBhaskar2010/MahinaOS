use crate::error::ControlError;
use serde::{Deserialize, Serialize};

/// Wire format request received over the Unix domain socket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub capability_token: Option<String>,
}

/// Wire format response returned over the Unix domain socket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorPayload {
    pub code: i32,
    pub message: String,
}

impl Response {
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: u64, code: i32, message: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            result: None,
            error: Some(ErrorPayload {
                code,
                message: message.into(),
            }),
        }
    }
}

/// Strongly typed service entry from supervisor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub name: String,
    pub state: String,
    pub pid: i32,
}

/// Strongly typed individual service status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub state: String,
    pub pid: i32,
    pub restart_count: u32,
}

/// Strongly typed user entry (passwords strictly excluded)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserEntry {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub comment: String,
    pub home_dir: String,
    pub shell: String,
    pub groups: Vec<String>,
}

/// Strongly typed installed package entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub file_count: usize,
}

/// Strongly typed filesystem mount entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountEntry {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub options: String,
    pub subvolume: Option<String>,
}

/// Strongly typed block device entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDeviceEntry {
    pub name: String,
    pub size_bytes: u64,
    pub is_rotational: bool,
    pub is_read_only: bool,
}

/// Strongly typed holistic storage overview
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageOverview {
    pub mounts: Vec<MountEntry>,
    pub devices: Vec<BlockDeviceEntry>,
}

/// Strongly typed generation entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationEntry {
    pub id: u32,
    pub description: String,
    pub created_at_epoch_secs: u64,
    pub is_current: bool,
    pub is_healthy: bool,
    pub kernel_version: String,
}

/// Strongly typed internal domain commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainCommand {
    SystemGetState,
    SystemReboot,
    SystemShutdown,
    ServicesList,
    ServicesStatus { name: Option<String> },
    ServicesStart { name: String },
    ServicesStop { name: String },
    ServicesRestart { name: String },
    ServicesReload { name: String },
    UsersList,
    PackagesList,
    PackagesInstall { target: String },
    PackagesRemove { name: String },
    StorageList,
    StorageMount {
        source: String,
        target: String,
        fs_type: Option<String>,
        options: Option<String>,
    },
    GenerationsGetCurrent,
    GenerationsList,
    GenerationsCreate { description: String },
    GenerationsActivate { id: u32 },
    GenerationsMarkHealthy { id: Option<u32> },
    GenerationsRollback { target_id: Option<u32> },
    GenerationsPin { id: u32, pin: bool },
    GenerationsRecoveryCapabilities,
}

impl DomainCommand {
    pub fn parse(method: &str, params: &serde_json::Value) -> Result<Self, ControlError> {
        match method {
            "system.get_state" => Ok(Self::SystemGetState),
            "system.reboot" => Ok(Self::SystemReboot),
            "system.shutdown" => Ok(Self::SystemShutdown),
            "services.list" => Ok(Self::ServicesList),
            "services.status" => {
                let name = params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                Ok(Self::ServicesStatus { name })
            }
            "services.start" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "name".to_string(),
                        reason: "Missing required parameter 'name'".to_string(),
                    })?;
                Ok(Self::ServicesStart {
                    name: name.to_string(),
                })
            }
            "services.stop" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "name".to_string(),
                        reason: "Missing required parameter 'name'".to_string(),
                    })?;
                Ok(Self::ServicesStop {
                    name: name.to_string(),
                })
            }
            "services.restart" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "name".to_string(),
                        reason: "Missing required parameter 'name'".to_string(),
                    })?;
                Ok(Self::ServicesRestart {
                    name: name.to_string(),
                })
            }
            "services.reload" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "name".to_string(),
                        reason: "Missing required parameter 'name'".to_string(),
                    })?;
                Ok(Self::ServicesReload {
                    name: name.to_string(),
                })
            }
            "users.list" => Ok(Self::UsersList),
            "packages.list" => Ok(Self::PackagesList),
            "packages.install" => {
                let target = params
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "target".to_string(),
                        reason: "Missing required parameter 'target'".to_string(),
                    })?;
                Ok(Self::PackagesInstall {
                    target: target.to_string(),
                })
            }
            "packages.remove" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "name".to_string(),
                        reason: "Missing required parameter 'name'".to_string(),
                    })?;
                Ok(Self::PackagesRemove {
                    name: name.to_string(),
                })
            }
            "storage.list" => Ok(Self::StorageList),
            "storage.mount" => {
                let source = params
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "source".to_string(),
                        reason: "Missing required parameter 'source'".to_string(),
                    })?;
                let target = params
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "target".to_string(),
                        reason: "Missing required parameter 'target'".to_string(),
                    })?;
                let fs_type = params.get("fs_type").and_then(|v| v.as_str()).map(|s| s.to_string());
                let options = params.get("options").and_then(|v| v.as_str()).map(|s| s.to_string());
                Ok(Self::StorageMount {
                    source: source.to_string(),
                    target: target.to_string(),
                    fs_type,
                    options,
                })
            }
            "generation.get_current" | "generations.get_current" => Ok(Self::GenerationsGetCurrent),
            "generation.list" | "generations.list" => Ok(Self::GenerationsList),
            "generation.create" | "generations.create" => {
                let desc = params
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("User created generation snapshot");
                Ok(Self::GenerationsCreate {
                    description: desc.to_string(),
                })
            }
            "generation.activate" | "generations.activate" => {
                let id = params
                    .get("id")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32)
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "id".to_string(),
                        reason: "Missing required parameter 'id'".to_string(),
                    })?;
                Ok(Self::GenerationsActivate { id })
            }
            "generation.mark_healthy" | "generations.mark_healthy" => {
                let id = params
                    .get("id")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);
                Ok(Self::GenerationsMarkHealthy { id })
            }
            "generation.rollback" | "generations.rollback" => {
                let target_id = params
                    .get("target_id")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);
                Ok(Self::GenerationsRollback { target_id })
            }
            "generation.pin" | "generations.pin" => {
                let id = params
                    .get("id")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32)
                    .ok_or_else(|| ControlError::InvalidParameter {
                        param: "id".to_string(),
                        reason: "Missing required parameter 'id'".to_string(),
                    })?;
                let pin = params.get("pin").and_then(|v| v.as_bool()).unwrap_or(true);
                Ok(Self::GenerationsPin { id, pin })
            }
            "generation.capabilities" | "generations.capabilities" => {
                Ok(Self::GenerationsRecoveryCapabilities)
            }
            other => Err(ControlError::NotFound(format!("Unknown method '{}'", other))),
        }
    }

    pub fn method_name(&self) -> &'static str {
        match self {
            Self::SystemGetState => "system.get_state",
            Self::SystemReboot => "system.reboot",
            Self::SystemShutdown => "system.shutdown",
            Self::ServicesList => "services.list",
            Self::ServicesStatus { .. } => "services.status",
            Self::ServicesStart { .. } => "services.start",
            Self::ServicesStop { .. } => "services.stop",
            Self::ServicesRestart { .. } => "services.restart",
            Self::ServicesReload { .. } => "services.reload",
            Self::UsersList => "users.list",
            Self::PackagesList => "packages.list",
            Self::PackagesInstall { .. } => "packages.install",
            Self::PackagesRemove { .. } => "packages.remove",
            Self::StorageList => "storage.list",
            Self::StorageMount { .. } => "storage.mount",
            Self::GenerationsGetCurrent => "generation.get_current",
            Self::GenerationsList => "generation.list",
            Self::GenerationsCreate { .. } => "generation.create",
            Self::GenerationsActivate { .. } => "generation.activate",
            Self::GenerationsMarkHealthy { .. } => "generation.mark_healthy",
            Self::GenerationsRollback { .. } => "generation.rollback",
            Self::GenerationsPin { .. } => "generation.pin",
            Self::GenerationsRecoveryCapabilities => "generation.capabilities",
        }
    }

    pub fn is_mutating(&self) -> bool {
        match self {
            Self::SystemGetState
            | Self::ServicesList
            | Self::ServicesStatus { .. }
            | Self::UsersList
            | Self::PackagesList
            | Self::StorageList
            | Self::GenerationsGetCurrent
            | Self::GenerationsList
            | Self::GenerationsRecoveryCapabilities => false,
            Self::SystemReboot
            | Self::SystemShutdown
            | Self::ServicesStart { .. }
            | Self::ServicesStop { .. }
            | Self::ServicesRestart { .. }
            | Self::ServicesReload { .. }
            | Self::PackagesInstall { .. }
            | Self::PackagesRemove { .. }
            | Self::StorageMount { .. }
            | Self::GenerationsCreate { .. }
            | Self::GenerationsActivate { .. }
            | Self::GenerationsMarkHealthy { .. }
            | Self::GenerationsRollback { .. }
            | Self::GenerationsPin { .. } => true,
        }
    }
}

/// Strongly typed system telemetry state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemState {
    pub hostname: String,
    pub uptime_secs: u64,
    pub kernel_version: String,
    pub mem_total_mb: u64,
    pub mem_used_mb: u64,
    pub cpu_count: usize,
    pub active_generation: u32,
}

/// Strongly typed internal domain results
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainResult {
    SystemState(SystemState),
    ServicesList(Vec<ServiceEntry>),
    ServiceStatus(ServiceStatus),
    UsersList(Vec<UserEntry>),
    PackagesList(Vec<PackageEntry>),
    StorageOverview(StorageOverview),
    GenerationCurrent(GenerationEntry),
    GenerationsList(Vec<GenerationEntry>),
    RecoveryCapabilities(crate::boot::RecoveryCapabilities),
    ActionSuccess { action: String, details: String },
    SuccessMessage(String),
    Empty,
}

impl DomainResult {
    pub fn to_value(&self) -> serde_json::Value {
        match self {
            Self::SystemState(state) => serde_json::to_value(state).unwrap_or(serde_json::Value::Null),
            Self::ServicesList(list) => serde_json::to_value(list).unwrap_or(serde_json::Value::Null),
            Self::ServiceStatus(status) => serde_json::to_value(status).unwrap_or(serde_json::Value::Null),
            Self::UsersList(list) => serde_json::to_value(list).unwrap_or(serde_json::Value::Null),
            Self::PackagesList(list) => serde_json::to_value(list).unwrap_or(serde_json::Value::Null),
            Self::StorageOverview(overview) => serde_json::to_value(overview).unwrap_or(serde_json::Value::Null),
            Self::GenerationCurrent(entry) => serde_json::to_value(entry).unwrap_or(serde_json::Value::Null),
            Self::GenerationsList(list) => serde_json::to_value(list).unwrap_or(serde_json::Value::Null),
            Self::RecoveryCapabilities(caps) => serde_json::to_value(caps).unwrap_or(serde_json::Value::Null),
            Self::ActionSuccess { action, details } => {
                serde_json::json!({ "action": action, "details": details })
            }
            Self::SuccessMessage(msg) => serde_json::json!({ "message": msg }),
            Self::Empty => serde_json::json!({}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_deserialization() {
        let json_str = r#"{"id":1,"method":"system.get_state"}"#;
        let req: Request = serde_json::from_str(json_str).expect("deserialize failed");
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "system.get_state");
        assert_eq!(req.capability_token, None);
    }

    #[test]
    fn test_response_serialization() {
        let resp = Response::success(42, serde_json::json!({"status": "healthy"}));
        let serialized = serde_json::to_string(&resp).expect("serialize failed");
        assert!(serialized.contains("\"id\":42"));
        assert!(serialized.contains("\"ok\":true"));
        assert!(serialized.contains("\"status\":\"healthy\""));
    }

    #[test]
    fn test_domain_command_parsing() {
        let cmd = DomainCommand::parse("system.get_state", &serde_json::Value::Null).unwrap();
        assert_eq!(cmd, DomainCommand::SystemGetState);
        assert!(!cmd.is_mutating());

        let svc_cmd = DomainCommand::parse("services.start", &serde_json::json!({"name": "udev"})).unwrap();
        assert_eq!(
            svc_cmd,
            DomainCommand::ServicesStart {
                name: "udev".to_string()
            }
        );
        assert!(svc_cmd.is_mutating());

        let err = DomainCommand::parse("invalid.method", &serde_json::Value::Null).unwrap_err();
        match err {
            ControlError::NotFound(msg) => assert!(msg.contains("invalid.method")),
            _ => panic!("unexpected error"),
        }
    }
}
