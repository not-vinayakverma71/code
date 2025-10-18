// IPC module exports

pub mod ipc_messages;
pub mod ipc_config;
pub mod config_validation_tests;
pub mod binary_codec;
pub mod canonical_header;
#[cfg(unix)]
pub mod framed_shm_stream;
#[cfg(unix)]
pub mod shm_metrics;
pub mod shm_namespace;
#[cfg(unix)]
pub mod shm_permissions;
#[cfg(unix)]
pub mod shm_notifier;

// New cross-process safe modules
pub mod ring_header_volatile;
#[cfg(unix)]
pub mod control_socket;
#[cfg(unix)]
pub mod shm_buffer_volatile;
#[cfg(target_os = "linux")]
pub mod shm_buffer_futex;
#[cfg(target_os = "macos")]
pub mod shm_buffer_macos;
pub mod platform_buffer;
#[cfg(unix)]
pub mod eventfd_doorbell;
#[cfg(target_os = "linux")]
pub mod futex;
#[cfg(target_os = "macos")]
pub mod kqueue_doorbell;
#[cfg(unix)]
pub mod posix_sem_sync;
#[cfg(windows)]
pub mod windows_event;
#[cfg(windows)]
pub mod windows_sync;
#[cfg(windows)]
pub mod shm_buffer_windows;
#[cfg(unix)]
pub mod fd_pass;
#[cfg(unix)]
pub mod ipc_server_volatile;
#[cfg(unix)]
pub mod ipc_client_volatile;
pub mod spsc_shm_ring;
pub mod shm_waiter_cross_os;
pub mod shm_stream_optimized;
pub mod shm_io_workers;
pub mod shm_metrics_optimized;
pub mod shm_listener_optimized;
#[cfg(unix)]
pub mod crash_recovery;
pub mod errors;
pub mod ipc_server;
pub mod ipc_client;
#[cfg(unix)]
pub mod shared_memory_complete;
pub mod zero_copy_codec;
pub mod cross_platform_ipc;
pub mod macos_shared_memory;
#[cfg(windows)]
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
pub mod context_routes;
pub mod provider_routes;
pub mod provider_config;

// Re-export commonly used types
pub use ipc_server::IpcServer;
pub use ipc_client::{IpcClient, IpcClientStats};
pub use errors::{IpcError, IpcResult};
pub use ipc_config::IpcConfig;
pub use ipc_messages::MessageType;
#[cfg(unix)]
pub use shared_memory_complete::{SharedMemoryBuffer, SharedMemoryListener, SharedMemoryStream};
#[cfg(unix)]
pub use framed_shm_stream::FramedShmStream;
pub use canonical_header::{CanonicalHeader, MessageType as CanonicalMessageType};
#[cfg(windows)]
pub use windows_shared_memory::{SharedMemoryBuffer, SharedMemoryListener, SharedMemoryStream};
pub use crate::connection_pool_manager::{ConnectionPoolManager, PoolConfig, ConnectionStats};
pub use auto_reconnection::AutoReconnectionManager;
pub use handler_registration_types::WebviewMessage;