pub mod audit;
pub mod auth;
pub mod client;
pub mod daemon;
pub mod error;
pub mod idempotency;
pub mod protocol;
pub mod providers;
pub mod sha256;
pub mod state;

pub use audit::AuditLogger;
pub use auth::{CapabilityAuthorizer, IdentityTier, PeerCredentials};
pub use client::ControlClient;
pub use daemon::{ControlDaemon, DEFAULT_SOCKET_PATH};
pub use error::ControlError;
pub use idempotency::IdempotencyStore;
pub use protocol::{DomainCommand, DomainResult, Request, Response, SystemState};
pub use providers::ProviderDispatcher;
pub use sha256::Sha256;
pub use state::{
    BootHealthStatus, BootStage, BootState, GenerationManifest, GenerationRegistry,
    GenerationStatus, GenerationSummary, StateStore,
};
