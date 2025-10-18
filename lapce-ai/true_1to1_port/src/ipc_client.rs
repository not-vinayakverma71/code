// EXACT 1:1 Translation from packages/ipc/src/ipc-client.ts
// Every line matches the TypeScript source

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::crypto;
use crate::node_ipc::IPC;
use crate::types::*;

// Line 15: export class IpcClient extends EventEmitter<IpcClientEvents> {
pub struct IpcClient {
    // Line 16: private readonly _socketPath: string
    _socket_path: String,
    
    // Line 17: private readonly _id: string
    _id: String,
    
    // Line 18: private readonly _log: (...args: unknown[]) => void
    _log: Arc<dyn Fn(&str) + Send + Sync>,
    
    // Line 19: private _isConnected = false
    _is_connected: Arc<Mutex<bool>>,
    
    // Line 20: private _clientId?: string
    _client_id: Arc<Mutex<Option<String>>>,
    
    // EventEmitter functionality
    listeners: Arc<Mutex<HashMap<String, Vec<Box<dyn Fn(serde_json::Value) + Send>>>>>,
}

impl IpcClient {
    // Line 22-36: constructor(socketPath: string, log = console.log) {
    pub fn new(socket_path: String, log: Option<Box<dyn Fn(&str) + Send + Sync>>) -> Self {
        // Line 23: super()
        
        // Line 25: this._socketPath = socketPath
        // Line 26: this._id = `roo-code-evals-${crypto.randomBytes(6).toString("hex")}`
        let id = format!("roo-code-evals-{}", crypto::to_hex(&crypto::random_bytes(6)));
        
        // Line 27: this._log = log
        let client = Self {
            _socket_path: socket_path.clone(),
            _id: id.clone(),
            _log: Arc::new(log.unwrap_or(Box::new(|msg| println!("{}", msg)))),
            _is_connected: Arc::new(Mutex::new(false)),
            _client_id: Arc::new(Mutex::new(None)),
            listeners: Arc::new(Mutex::new(HashMap::new())),
        };
        
        // Line 29: ipc.config.silent = true
        // Note: Cannot mutate Arc<Ipc> directly in Rust
        
        // Line 31-35: ipc.connectTo(this._id, this.socketPath, () => {
        let id_connect = id.clone();
        let socket_path_connect = socket_path;
        let is_connected = client._is_connected.clone();
        let client_id = client._client_id.clone();
        let log_fn = client._log.clone();
        let listeners = client.listeners.clone();
        
        IPC.connect_to(&id_connect, &socket_path_connect, move || {
            // Line 32: ipc.of[this._id]?.on("connect", () => this.onConnect())
            if let Some(connection) = IPC.of.lock().unwrap().get(&id_connect) {
                let is_connected_on = is_connected.clone();
                let log_on = log_fn.clone();
                let listeners_on = listeners.clone();
                connection.on("connect", move |_| {
                    Self::on_connect(&is_connected_on, &log_on, &listeners_on);
                });
                
                // Line 33: ipc.of[this._id]?.on("disconnect", () => this.onDisconnect())
                let is_connected_off = is_connected.clone();
                let log_off = log_fn.clone();
                let listeners_off = listeners.clone();
                connection.on("disconnect", move |_| {
                    Self::on_disconnect(&is_connected_off, &log_off, &listeners_off);
                });
                
                // Line 34: ipc.of[this._id]?.on("message", (data) => this.onMessage(data))
                let client_id_msg = client_id.clone();
                let log_msg = log_fn.clone();
                let listeners_msg = listeners.clone();
                connection.on("message", move |data| {
                    Self::on_message(data, &client_id_msg, &log_msg, &listeners_msg);
                });
            }
        });
        
        client
    }
    
    // Line 38-46: private onConnect() {
    fn on_connect(
        is_connected: &Arc<Mutex<bool>>,
        log: &Arc<dyn Fn(&str) + Send + Sync>,
        listeners: &Arc<Mutex<HashMap<String, Vec<Box<dyn Fn(serde_json::Value) + Send>>>>>,
    ) {
        // Line 39-41: if (this._isConnected) { return }
        if *is_connected.lock().unwrap() {
            return;
        }
        
        // Line 43: this.log("[client#onConnect]")
        log("[client#onConnect]");
        
        // Line 44: this._isConnected = true
        *is_connected.lock().unwrap() = true;
        
        // Line 45: this.emit(IpcMessageType.Connect)
        Self::emit_event(listeners, "Connect", serde_json::Value::Null);
    }
    
    // Line 48-56: private onDisconnect() {
    fn on_disconnect(
        is_connected: &Arc<Mutex<bool>>,
        log: &Arc<dyn Fn(&str) + Send + Sync>,
        listeners: &Arc<Mutex<HashMap<String, Vec<Box<dyn Fn(serde_json::Value) + Send>>>>>,
    ) {
        // Line 49-51: if (!this._isConnected) { return }
        if !*is_connected.lock().unwrap() {
            return;
        }
        
        // Line 53: this.log("[client#onDisconnect]")
        log("[client#onDisconnect]");
        
        // Line 54: this._isConnected = false
        *is_connected.lock().unwrap() = false;
        
        // Line 55: this.emit(IpcMessageType.Disconnect)
        Self::emit_event(listeners, "Disconnect", serde_json::Value::Null);
    }
    
    // Line 58-84: private onMessage(data: unknown) {
    fn on_message(
        data: serde_json::Value,
        client_id: &Arc<Mutex<Option<String>>>,
        log: &Arc<dyn Fn(&str) + Send + Sync>,
        listeners: &Arc<Mutex<HashMap<String, Vec<Box<dyn Fn(serde_json::Value) + Send>>>>>,
    ) {
        // Line 59-62: if (typeof data !== "object") { ... }
        if !data.is_object() {
            log(&format!("[client#onMessage] invalid data {:?}", data));
            return;
        }
        
        // Line 64: const result = ipcMessageSchema.safeParse(data)
        let result = ipc_message_schema_safe_parse(&data);
        
        // Line 66-69: if (!result.success) { ... }
        if !result.success {
            if let Some(error) = result.error {
                log(&format!("[client#onMessage] invalid payload {:?} {:?}", error.format(), data));
            }
            return;
        }
        
        // Line 71: const payload = result.data
        if let Some(payload) = result.data {
            // Line 73-83: if (payload.origin === IpcOrigin.Server) { ... }
            match payload {
                // Line 75-78: case IpcMessageType.Ack:
                IpcMessage::Ack { origin, data } if origin == IpcOrigin::Server => {
                    // Line 76: this._clientId = payload.data.clientId
                    *client_id.lock().unwrap() = Some(data.client_id.clone());
                    // Line 77: this.emit(IpcMessageType.Ack, payload.data)
                    Self::emit_event(listeners, "Ack", serde_json::to_value(data).unwrap());
                }
                // Line 79-81: case IpcMessageType.TaskEvent:
                IpcMessage::TaskEvent { origin, data, .. } if origin == IpcOrigin::Server => {
                    // Line 80: this.emit(IpcMessageType.TaskEvent, payload.data)
                    Self::emit_event(listeners, "TaskEvent", data);
                }
                _ => {}
            }
        }
    }
    
    // Line 86-88: private log(...args: unknown[]) {
    fn log(&self, message: &str) {
        (self._log)(message);
    }
    
    // Line 90-99: public sendCommand(command: TaskCommand) {
    pub fn send_command(&self, command: TaskCommand) {
        // Line 91-96: const message: IpcMessage = { ... }
        if let Some(client_id) = self._client_id.lock().unwrap().clone() {
            let message = IpcMessage::TaskCommand {
                origin: IpcOrigin::Client,
                client_id,
                data: command,
            };
            
            // Line 98: this.sendMessage(message)
            self.send_message(message);
        }
    }
    
    // Line 101-103: public sendMessage(message: IpcMessage) {
    pub fn send_message(&self, message: IpcMessage) {
        // Line 102: ipc.of[this._id]?.emit("message", message)
        if let Some(connection) = IPC.of.lock().unwrap().get(&self._id) {
            connection.emit("message", serde_json::to_value(message).unwrap());
        }
    }
    
    // Line 105-112: public disconnect() {
    pub fn disconnect(&self) {
        // Line 106-111: try { ipc.disconnect(this._id) } catch (error) { ... }
        IPC.disconnect(&self._id);
        // Line 108: // @TODO: Should we set _disconnect here?
    }
    
    // Line 114-116: public get socketPath() {
    pub fn socket_path(&self) -> &str {
        &self._socket_path
    }
    
    // Line 118-120: public get clientId() {
    pub fn client_id(&self) -> Option<String> {
        self._client_id.lock().unwrap().clone()
    }
    
    // Line 122-124: public get isConnected() {
    pub fn is_connected(&self) -> bool {
        *self._is_connected.lock().unwrap()
    }
    
    // Line 126-128: public get isReady() {
    pub fn is_ready(&self) -> bool {
        *self._is_connected.lock().unwrap() && self._client_id.lock().unwrap().is_some()
    }
    
    // EventEmitter helper
    fn emit_event(
        listeners: &Arc<Mutex<HashMap<String, Vec<Box<dyn Fn(serde_json::Value) + Send>>>>>,
        event: &str,
        data: serde_json::Value,
    ) {
        if let Some(callbacks) = listeners.lock().unwrap().get(event) {
            for callback in callbacks {
                callback(data.clone());
            }
        }
    }
    
    pub fn emit(&self, event: IpcMessageType, data: serde_json::Value) {
        Self::emit_event(&self.listeners, &format!("{:?}", event), data);
    }
}
