/// Shared Memory IPC Transport for AI Bridge
/// Connects Lapce UI to lapce-ai-rust backend via IPC

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use super::bridge::BridgeError;
use super::messages::{ConnectionStatusType, InboundMessage, OutboundMessage};
use super::transport::Transport;

// TODO: Import from lapce-ai-rust once Cargo dependency is added
// For now, we'll use a runtime handle pattern
// use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

/// ShmTransport: Real IPC connection to lapce-ai backend
pub struct ShmTransport {
    client: Arc<Mutex<Option<IpcClientHandle>>>,
    inbound_queue: Arc<Mutex<VecDeque<InboundMessage>>>,
    status: Arc<Mutex<ConnectionStatusType>>,
    socket_path: String,
    runtime: Arc<tokio::runtime::Runtime>,
}

/// Handle to IPC client with runtime
struct IpcClientHandle {
    // This will hold: lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile
    // For now, using a type-erased approach
    send_fn: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send>,
}

impl ShmTransport {
    /// Create new transport (disconnected initially)
    pub fn new(socket_path: impl Into<String>) -> Self {
        // Create dedicated runtime for IPC operations
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("ai-bridge-ipc")
            .enable_all()
            .build()
            .expect("Failed to create IPC runtime");
        
        Self {
            client: Arc::new(Mutex::new(None)),
            inbound_queue: Arc::new(Mutex::new(VecDeque::new())),
            status: Arc::new(Mutex::new(ConnectionStatusType::Disconnected)),
            socket_path: socket_path.into(),
            runtime: Arc::new(runtime),
        }
    }
}

impl Transport for ShmTransport {
    fn send(&self, message: OutboundMessage) -> Result<(), BridgeError> {
        let client_guard = self.client.lock().unwrap();
        
        let client = client_guard.as_ref()
            .ok_or(BridgeError::Disconnected)?;
        
        // Serialize message to JSON (protocol format)
        let serialized = serde_json::to_vec(&message)
            .map_err(|e| BridgeError::SerializationError(e.to_string()))?;
        
        // Send through IPC client
        let response = (client.send_fn)(&serialized)
            .map_err(|e| BridgeError::SendFailed(e))?;
        
        // If we got a response, deserialize and queue it
        if !response.is_empty() {
            if let Ok(msg) = serde_json::from_slice::<InboundMessage>(&response) {
                self.inbound_queue.lock().unwrap().push_back(msg);
            }
        }
        
        Ok(())
    }
    
    fn try_receive(&self) -> Option<InboundMessage> {
        let mut queue = self.inbound_queue.lock().unwrap();
        queue.pop_front()
    }
    
    fn status(&self) -> ConnectionStatusType {
        self.status.lock().unwrap().clone()
    }
    
    fn connect(&mut self) -> Result<(), BridgeError> {
        let socket_path = self.socket_path.clone();
        let runtime = self.runtime.clone();
        
        eprintln!("[SHM_TRANSPORT] Connecting to: {}", socket_path);
        
        // TODO: Once lapce-ai-rust is added as dependency, use:
        // let ipc_client = runtime.block_on(async {
        //     lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile::connect(&socket_path).await
        // }).map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;
        
        // For now, create a stub that will be replaced
        let handle = IpcClientHandle {
            send_fn: Box::new(move |data: &[u8]| {
                // Placeholder: Echo back for testing
                eprintln!("[SHM_TRANSPORT] Stub send: {} bytes", data.len());
                Ok(vec![]) // Empty response
            }),
        };
        
        *self.client.lock().unwrap() = Some(handle);
        *self.status.lock().unwrap() = ConnectionStatusType::Connected;
        
        eprintln!("[SHM_TRANSPORT] Connected");
        Ok(())
    }
    
    fn disconnect(&mut self) -> Result<(), BridgeError> {
        *self.client.lock().unwrap() = None;
        *self.status.lock().unwrap() = ConnectionStatusType::Disconnected;
        
        eprintln!("[SHM_TRANSPORT] Disconnected");
        Ok(())
    }
}

// ============================================================================
// Background receiver task (for async message handling)
// ============================================================================

impl ShmTransport {
    /// Start background task to receive messages from IPC
    pub fn start_receiver_task(&self) {
        // TODO Phase A: Implement async receiver
        // let client = self.client.clone();
        // let queue = self.inbound_queue.clone();
        // let status = self.status.clone();
        // 
        // tokio::spawn(async move {
        //     loop {
        //         // Check if still connected
        //         if *status.lock().unwrap() != ConnectionStatusType::Connected {
        //             break;
        //         }
        //         
        //         // Try to receive message
        //         if let Some(client_guard) = client.lock().unwrap().as_ref() {
        //             match client_guard.try_receive().await {
        //                 Ok(data) => {
        //                     // Deserialize and queue
        //                     if let Ok(msg) = bincode::deserialize::<InboundMessage>(&data) {
        //                         queue.lock().unwrap().push_back(msg);
        //                     }
        //                 }
        //                 Err(_) => {
        //                     tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        //                 }
        //             }
        //         }
        //     }
        // });
    }
}
