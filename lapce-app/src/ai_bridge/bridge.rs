// BridgeClient: Main interface for AI communication
//
// Design: The UI interacts only with BridgeClient, which routes messages
// through the Transport layer. This abstraction allows swapping transports
// (NoTransport → ShmTransport) without changing UI code.

use std::sync::{Arc, Mutex};

use super::messages::{ConnectionStatusType, InboundMessage, OutboundMessage};
use super::transport::Transport;

// ============================================================================
// BridgeClient: Main UI interface
// ============================================================================

/// BridgeClient manages communication with the lapce-ai backend
#[derive(Debug)]
pub struct BridgeClient {
    transport: Arc<Mutex<Box<dyn Transport>>>,
}

impl BridgeClient {
    /// Create a new BridgeClient with the given transport
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            transport: Arc::new(Mutex::new(transport)),
        }
    }

    /// Send a message to the backend
    pub fn send(&self, message: OutboundMessage) -> Result<(), BridgeError> {
        let transport = self.transport.lock().unwrap();
        transport.send(message)
    }

    /// Try to receive an inbound message (non-blocking)
    pub fn try_receive(&self) -> Option<InboundMessage> {
        let transport = self.transport.lock().unwrap();
        transport.try_receive()
    }

    /// Get current connection status
    pub fn status(&self) -> ConnectionStatusType {
        let transport = self.transport.lock().unwrap();
        transport.status()
    }

    /// Attempt to connect
    pub fn connect(&self) -> Result<(), BridgeError> {
        let mut transport = self.transport.lock().unwrap();
        transport.connect()
    }

    /// Disconnect
    pub fn disconnect(&self) -> Result<(), BridgeError> {
        let mut transport = self.transport.lock().unwrap();
        transport.disconnect()
    }
}

// ============================================================================
// Connection state
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

// ============================================================================
// Error types
// ============================================================================

#[derive(Debug, Clone, thiserror::Error)]
pub enum BridgeError {
    #[error("Bridge is disconnected")]
    Disconnected,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),
}
