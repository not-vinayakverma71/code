// Transport layer abstraction for AI Bridge
// Supports NoTransport (disconnected), and later ShmTransport (shared memory IPC)

use std::sync::Arc;

use super::bridge::BridgeError;
use super::messages::{ConnectionStatusType, InboundMessage, OutboundMessage};

/// Transport trait for sending/receiving messages
/// Note: Not async for now - will be when we add real IPC
pub trait Transport: Send + Sync {
    /// Send a message to the backend
    fn send(&self, message: OutboundMessage) -> Result<(), BridgeError>;

    /// Receive the next message from the backend (non-blocking)
    fn try_receive(&self) -> Option<InboundMessage>;

    /// Get current connection status
    fn status(&self) -> ConnectionStatusType;

    /// Attempt to connect/reconnect
    fn connect(&mut self) -> Result<(), BridgeError>;

    /// Disconnect gracefully
    fn disconnect(&mut self) -> Result<(), BridgeError>;
}

// ============================================================================
// NoTransport: Disconnected state implementation (Phase 0)
// ============================================================================

/// NoTransport: Always disconnected, no message exchange
/// This is the default transport until IPC is ready (Phase A/B)
#[derive(Debug)]
pub struct NoTransport {}

impl NoTransport {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NoTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for NoTransport {
    fn send(&self, _message: OutboundMessage) -> Result<(), BridgeError> {
        // NoTransport: messages go nowhere
        Err(BridgeError::Disconnected)
    }

    fn try_receive(&self) -> Option<InboundMessage> {
        // NoTransport: never receives messages
        None
    }

    fn status(&self) -> ConnectionStatusType {
        ConnectionStatusType::Disconnected
    }

    fn connect(&mut self) -> Result<(), BridgeError> {
        Err(BridgeError::Disconnected)
    }

    fn disconnect(&mut self) -> Result<(), BridgeError> {
        Ok(())
    }
}

// ============================================================================
// Future: ShmTransport for shared-memory IPC (Phase A)
// ============================================================================

// TODO Phase A: Implement ShmTransport using shared memory protocol
// pub struct ShmTransport {
//     sender: ShmSender,
//     receiver: ShmReceiver,
//     status: Arc<Mutex<ConnectionStatusType>>,
// }
