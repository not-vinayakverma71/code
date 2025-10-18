// Native terminal operations - Direct system execution without MCP overhead
// Faster command execution, no protocol serialization needed

pub mod ops;      // Main terminal operations (from terminal_tool.rs)
pub mod execute;  // Command execution operations (from execute_command.rs)

// Re-export commonly used items
pub use ops::{TerminalOperations, TerminalTool};
pub use execute::CommandExecutor;
// pub use execute::CommandOutput; // Type doesn't exist
