use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    PermissionDenied(String),
    PeerCredFailed(String),
    TokenExpired,
    TokenInvalid(String),
    NotFound(String),
    InvalidParameter { param: String, reason: String },
    Io(String),
    Serialization(String),
    Internal(String),
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied(reason) => write!(f, "Permission denied: {}", reason),
            Self::PeerCredFailed(reason) => write!(f, "Failed to determine peer credentials: {}", reason),
            Self::TokenExpired => write!(f, "Capability token has expired"),
            Self::TokenInvalid(reason) => write!(f, "Invalid capability token: {}", reason),
            Self::NotFound(resource) => write!(f, "Resource not found: {}", resource),
            Self::InvalidParameter { param, reason } => {
                write!(f, "Invalid parameter '{}': {}", param, reason)
            }
            Self::Io(msg) => write!(f, "I/O error: {}", msg),
            Self::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<std::io::Error> for ControlError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for ControlError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}
