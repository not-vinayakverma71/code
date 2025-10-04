// Native IDE operations - no MCP overhead
// Direct system operations for maximum performance

pub mod filesystem;
pub mod git;
pub mod terminal;

// Re-export commonly used items
pub use filesystem::ops::FileSystemOperations;
pub use git::ops::{GitOperations, DiffManager};
pub use terminal::ops::TerminalOperations;
pub use terminal::execute::CommandExecutor;
pub use filesystem::read::read_file;
pub use filesystem::write::write_file;
pub use filesystem::list::list_files;
