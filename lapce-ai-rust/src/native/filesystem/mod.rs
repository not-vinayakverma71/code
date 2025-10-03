// Native filesystem operations - Direct I/O without MCP protocol overhead
// 10x faster than MCP-based tools, no serialization/deserialization overhead

pub mod ops;      // Main filesystem operations (from filesystem_tool.rs)
pub mod read;     // File reading operations
pub mod write;    // File writing operations
pub mod list;     // Directory listing operations
pub mod search;   // File search operations
pub mod edit;     // File editing operations
pub mod replace;  // Search and replace operations
pub mod insert;   // Content insertion operations
pub mod watch;    // File watching operations

// Re-export commonly used items
pub use ops::FileSystemOperations;
// Functions don't exist as standalone exports - commenting out
// pub use read::read_file;
// pub use write::{write_file, append_file};
// pub use list::{list_files, list_dirs};
// pub use search::{search_files, search_in_files};
