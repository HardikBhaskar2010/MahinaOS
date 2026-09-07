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

/// Strongly typed internal domain commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainCommand {
    SystemGetState,
    SystemReboot,
    SystemShutdown,
}

impl DomainCommand {
    pub fn parse(method: &str, _params: &serde_json::Value) -> Result<Self, ControlError> {
        match method {
            "system.get_state" => Ok(Self::SystemGetState),
            "system.reboot" => Ok(Self::SystemReboot),
            "system.shutdown" => Ok(Self::SystemShutdown),
            other => Err(ControlError::NotFound(format!("Unknown method '{}'", other))),
        }
    }

    pub fn method_name(&self) -> &'static str {
        match self {
            Self::SystemGetState => "system.get_state",
            Self::SystemReboot => "system.reboot",
            Self::SystemShutdown => "system.shutdown",
        }
    }

    pub fn is_mutating(&self) -> bool {
        match self {
            Self::SystemGetState => false,
            Self::SystemReboot | Self::SystemShutdown => true,
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
    SuccessMessage(String),
    Empty,
}

impl DomainResult {
    pub fn to_value(&self) -> serde_json::Value {
        match self {
            Self::SystemState(state) => serde_json::to_value(state).unwrap_or(serde_json::Value::Null),
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

        let err = DomainCommand::parse("invalid.method", &serde_json::Value::Null).unwrap_err();
        match err {
            ControlError::NotFound(msg) => assert!(msg.contains("invalid.method")),
            _ => panic!("unexpected error"),
        }
    }
}
