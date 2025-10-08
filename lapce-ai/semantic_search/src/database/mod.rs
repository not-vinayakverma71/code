// Database module
pub mod cache_interface;
pub mod cache_manager;
pub mod code_index_manager;
pub mod config_interface;
pub mod config_manager;
pub mod listing;
pub mod manager_interface;
pub mod state_manager;
pub mod types;

// Re-export commonly used types
pub use cache_manager::CacheManager;
pub use cache_interface::ICacheManager;
pub use config_manager::{CodeIndexConfigManager, EmbedderProvider};
pub use code_index_manager::CodeIndexStateManager;
pub use state_manager::IndexState;
pub use types::{Database, DatabaseOptions, BaseTable, CreateTableRequest, CreateTableMode, 
                CreateTableData, OpenTableRequest, TableNamesRequest, ListNamespacesRequest,
                CreateNamespaceRequest, DropNamespaceRequest};
