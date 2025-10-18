/// Shared Memory IPC Transport for AI Bridge
/// Connects Lapce UI to lapce-ai-rust backend via IPC

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use super::bridge::BridgeError;
use super::messages::{ConnectionStatusType, InboundMessage, OutboundMessage};
use super::transport::Transport;

// Platform-specific IPC clients - internal to ShmTransport only
#[cfg(unix)]
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

#[cfg(windows)]
use lapce_ai_rust::ipc::windows_shared_memory::SharedMemoryStream;

/// ShmTransport: Real IPC connection to lapce-ai backend
pub struct ShmTransport {
    client: Arc<Mutex<Option<IpcClientHandle>>>,
    inbound_queue: Arc<Mutex<VecDeque<InboundMessage>>>,
    status: Arc<Mutex<ConnectionStatusType>>,
    socket_path: String,
    runtime: Arc<tokio::runtime::Runtime>,
}

/// Handle to IPC client with runtime (platform-agnostic wrapper)
struct IpcClientHandle {
    #[cfg(unix)]
    client: IpcClientVolatile,
    
    #[cfg(windows)]
    client: SharedMemoryStream,
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
        // Serialize message to JSON (UI protocol)
        let serialized = serde_json::to_vec(&message)
            .map_err(|e| BridgeError::SerializationError(e.to_string()))?;

        let runtime = self.runtime.clone();
        let client_guard = self.client.lock().unwrap();
        let handle = client_guard.as_ref().ok_or(BridgeError::Disconnected)?;

        #[cfg(unix)]
        {
            // Clone the IPC client (cheap; it holds Arcs internally)
            let ipc_client = handle.client.clone();
            // Send bytes to backend; backend echoes or responds per BinaryCodec handlers
            let response = runtime
                .block_on(async move { ipc_client.send_bytes(&serialized).await })
                .map_err(|e| BridgeError::SendFailed(e.to_string()))?;

            // If backend returned a UI message, enqueue it
            if !response.is_empty() {
                if let Ok(msg) = serde_json::from_slice::<InboundMessage>(&response) {
                    self.inbound_queue.lock().unwrap().push_back(msg);
                }
            }
            Ok(())
        }

        #[cfg(windows)]
        {
            let ipc_client = handle.client.clone();
            let response = runtime
                .block_on(async move { 
                    ipc_client.send(&serialized).await
                        .and_then(|_| ipc_client.recv().await)
                        .map(|opt| opt.unwrap_or_default())
                })
                .map_err(|e| BridgeError::SendFailed(format!("Windows IPC error: {}", e)))?;

            if !response.is_empty() {
                if let Ok(msg) = serde_json::from_slice::<InboundMessage>(&response) {
                    self.inbound_queue.lock().unwrap().push_back(msg);
                }
            }
            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(BridgeError::SendFailed(
                "IPC transport not available on this platform".into(),
            ))
        }
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

        #[cfg(unix)]
        {
            // Real IPC connection to lapce-ai backend
            let ipc_client = runtime
                .block_on(async { IpcClientVolatile::connect(&socket_path).await })
                .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

            let handle = IpcClientHandle { client: ipc_client };
            *self.client.lock().unwrap() = Some(handle);
            *self.status.lock().unwrap() = ConnectionStatusType::Connected;
            eprintln!("[SHM_TRANSPORT] Connected via real IPC");
            Ok(())
        }

        #[cfg(windows)]
        {
            let ipc_client = runtime
                .block_on(async { SharedMemoryStream::connect(&socket_path).await })
                .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

            let handle = IpcClientHandle { client: ipc_client };
            *self.client.lock().unwrap() = Some(handle);
            *self.status.lock().unwrap() = ConnectionStatusType::Connected;
            eprintln!("[SHM_TRANSPORT] Connected via Windows IPC");
            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(BridgeError::ConnectionFailed(
                "IPC transport not available on this platform".into(),
            ))
        }
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
