// Native Git operations - Direct git2 library usage without MCP overhead
// Faster than MCP-based tools, no protocol serialization needed

pub mod ops;        // Main git operations (from git_tool.rs)
pub mod diff;       // Diff application operations (from apply_diff.rs)
pub mod batch_diff; // Batch diff operations (from multi_apply_diff.rs)

// Re-export commonly used items
pub use ops::{GitOperations, GitTool, DiffManager};
pub use diff::apply_diff;
pub use batch_diff::BatchDiffApplier;
