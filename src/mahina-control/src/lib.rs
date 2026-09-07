pub mod audit;
pub mod auth;
pub mod boot;
pub mod btrfs;
pub mod client;
pub mod daemon;
pub mod error;
pub mod health;
pub mod idempotency;
pub mod lifecycle;
pub mod protocol;
pub mod providers;
pub mod sha256;
pub mod state;

pub use audit::AuditLogger;
pub use auth::{CapabilityAuthorizer, IdentityTier, PeerCredentials};
pub use boot::{LimineConfigGenerator, RecoveryCapabilities};
pub use btrfs::BtrfsEngine;
pub use client::ControlClient;
pub use daemon::{ControlDaemon, DEFAULT_SOCKET_PATH};
pub use error::ControlError;
pub use health::{HealthEvaluator, HealthReport};
pub use idempotency::IdempotencyStore;
pub use lifecycle::GenerationLifecycleManager;
pub use protocol::{DomainCommand, DomainResult, Request, Response, SystemState};
pub use providers::ProviderDispatcher;
pub use sha256::Sha256;
pub use state::{
    BootHealthStatus, BootStage, BootState, GenerationManifest, GenerationRegistry,
    GenerationStatus, GenerationSummary, StateStore,
};
