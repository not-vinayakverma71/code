// AI Bridge: Communication layer between Lapce UI and lapce-ai backend
// Phase A: IPC Integration - Real shared memory transport

pub mod bridge;
pub mod messages;
pub mod transport;
pub mod shm_transport;

#[cfg(feature = "examples")]
pub mod examples;

pub use bridge::{BridgeClient, BridgeError, ConnectionState};
pub use messages::{InboundMessage, OutboundMessage};
pub use transport::{NoTransport, Transport};
pub use shm_transport::ShmTransport;

impl std::fmt::Debug for dyn Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Transport")
    }
}
