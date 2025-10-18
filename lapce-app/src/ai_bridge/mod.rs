// AI Bridge: Communication layer between Lapce UI and lapce-ai backend
// Phase A: IPC Integration - Real shared memory transport

pub mod bridge;
pub mod messages;
pub mod transport;
pub mod shm_transport;
pub mod terminal_bridge;
pub mod context_bridge;

#[cfg(feature = "examples")]
pub mod examples;

pub use bridge::{BridgeClient, BridgeError, ConnectionState};
pub use messages::{InboundMessage, OutboundMessage, CommandSource, TerminalOp, FileContextSource};
pub use transport::{NoTransport, Transport};
pub use shm_transport::ShmTransport;
pub use terminal_bridge::TerminalBridge;
pub use context_bridge::ContextBridge;

// Default socket path - can be overridden by LAPCE_AI_SOCKET env var
pub fn default_socket_path() -> String {
    std::env::var("LAPCE_AI_SOCKET")
        .unwrap_or_else(|_| "/tmp/lapce_ai.sock".to_string())
}

impl std::fmt::Debug for dyn Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Transport")
    }
}
