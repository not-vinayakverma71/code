// AI Bridge: Communication layer between Lapce UI and lapce-ai backend
// Phase 0.1: Foundation - Bridge contracts without implementation

pub mod bridge;
pub mod messages;
pub mod transport;

pub use bridge::{BridgeClient, BridgeError, ConnectionState};
pub use messages::{InboundMessage, OutboundMessage};
pub use transport::{NoTransport, Transport};

impl std::fmt::Debug for dyn Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Transport")
    }
}
