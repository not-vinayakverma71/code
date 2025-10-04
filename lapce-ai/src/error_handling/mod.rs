// Error Handling Module - 1:1 Translation from TypeScript
// HOUR 1: Core Error Type System

pub mod errors;
pub mod context;
pub mod classifier;
pub mod recovery_system;
pub mod recovery_strategies;
pub mod circuit_breaker;
pub mod retry_policy;
pub mod degradation_manager;
pub mod state_recovery;
pub mod component_manager;
pub mod error_reporter;
pub mod panic_handler;

pub use errors::{LapceError, Result};
pub use recovery_system::{RecoverySystem, RecoveryAction};
pub use circuit_breaker::CircuitBreaker;
pub use retry_policy::RetryPolicy;
pub use degradation_manager::DegradationManager;
pub use state_recovery::StateRecovery;
pub use component_manager::ComponentManager;
pub use error_reporter::ErrorReporter;
