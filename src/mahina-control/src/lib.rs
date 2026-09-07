pub mod audit;
pub mod auth;
pub mod client;
pub mod daemon;
pub mod error;
pub mod idempotency;
pub mod protocol;
pub mod providers;

pub use audit::AuditLogger;
pub use auth::{CapabilityAuthorizer, IdentityTier, PeerCredentials};
pub use client::ControlClient;
pub use daemon::{ControlDaemon, DEFAULT_SOCKET_PATH};
pub use error::ControlError;
pub use idempotency::IdempotencyStore;
pub use protocol::{DomainCommand, DomainResult, Request, Response, SystemState};
pub use providers::ProviderDispatcher;
