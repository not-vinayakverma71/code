// Adapters for external integrations

pub mod traits;
pub mod ipc;
pub mod lapce_diff;
pub mod lapce_terminal;
pub mod context_tracker_adapter;

// Re-export traits
pub use traits::{Adapter, EventEmitter, CommandExecutor, DiffController, ApprovalHandler};
pub use context_tracker_adapter::{ContextTrackerAdapter, get_context_tracker};
