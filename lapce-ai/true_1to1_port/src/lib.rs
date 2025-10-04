// TRUE 1:1 Translation from TypeScript IPC Implementation
// This matches the exact structure and behavior of the original

pub mod node_ipc;
pub mod ipc_server;
pub mod ipc_client;
pub mod types;
pub mod crypto;

pub use ipc_server::IpcServer;
pub use ipc_client::IpcClient;
pub use types::*;
