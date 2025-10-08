// IPC module exports

pub mod ipc_messages;
pub mod ipc_config;
pub mod ipc_server;
pub mod shared_memory_complete;
pub mod cross_platform_ipc;
pub mod macos_shared_memory;
pub mod windows_shared_memory;
pub mod auto_reconnection;
pub mod connection_pool;
pub mod buffer_management;
pub mod handler_registration;
pub mod handler_registration_types;
pub mod message_routing_dispatch;
pub mod error_recovery;
pub mod health_server;
pub mod security;
pub mod binary_codec;
pub mod circuit_breaker;
pub mod unified_metrics;
pub mod ipc_scheduler;
pub mod ipc_connection_reuse;
pub mod codex_messages;
pub mod protocol_fuzz;
pub mod handshake_control;
pub mod zero_copy_codec;

// Re-export commonly used types
pub use ipc_server::{IpcServer, IpcError};
pub use ipc_config::IpcConfig;
pub use ipc_messages::MessageType;
pub use shared_memory_complete::{SharedMemoryBuffer, SharedMemoryListener, SharedMemoryStream};
pub use connection_pool::ConnectionPool;
pub use auto_reconnection::AutoReconnectionManager;
pub use handler_registration_types::WebviewMessage;