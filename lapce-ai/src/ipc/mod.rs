// IPC module exports

pub mod ipc_messages;
pub mod ipc_config;
pub mod config_validation_tests;
pub mod binary_codec;
pub mod errors;
pub mod ipc_server;
#[cfg(unix)]
pub mod shared_memory_complete;
pub mod zero_copy_codec;
#[cfg(unix)]
pub mod shm_namespace;
pub mod cross_platform_ipc;
pub mod macos_shared_memory;
pub mod windows_shared_memory;
pub mod auto_reconnection;
pub mod buffer_management;
pub mod handler_registration;
pub mod handler_registration_types;
pub mod message_routing_dispatch;
pub mod error_recovery;
pub mod health_server;
pub mod security;
pub mod circuit_breaker;
pub mod unified_metrics;
pub mod ipc_scheduler;
pub mod ipc_connection_reuse;
pub mod codex_messages;
pub mod protocol_fuzz;
pub mod handshake_control;
pub mod tool_router;

// Re-export commonly used types
pub use ipc_server::IpcServer;
pub use errors::{IpcError, IpcResult};
pub use ipc_config::IpcConfig;
pub use ipc_messages::MessageType;
#[cfg(unix)]
pub use shared_memory_complete::{SharedMemoryBuffer, SharedMemoryListener, SharedMemoryStream};
pub use crate::connection_pool_manager::{ConnectionPoolManager, PoolConfig, ConnectionStats};
pub use auto_reconnection::AutoReconnectionManager;
pub use handler_registration_types::WebviewMessage;