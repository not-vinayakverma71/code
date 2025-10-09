// Adapters for external integrations

pub mod traits;
pub mod ipc;
pub mod lapce_diff;
pub mod lapce_terminal;

// Re-export traits
pub use traits::{Adapter, EventEmitter, CommandExecutor, DiffController, ApprovalHandler};
