//! Context Tracking
//!
//! Direct 1:1 port from Codex/src/core/context-tracking/
//! Tracks file operations to detect stale context and prevent diff edit failures.
//!
//! Key features:
//! - Track files read/edited by AI vs user
//! - Mark existing entries stale when file is re-read
//! - File change detection via IPC events (replaces VSCode watchers)
//! - Persist task_metadata.json in task directory
//! - Get-and-clear queues for recently modified and checkpoint-possible files

pub mod file_context_tracker;
pub mod file_context_tracker_types;

pub use file_context_tracker::*;
pub use file_context_tracker_types::*;
