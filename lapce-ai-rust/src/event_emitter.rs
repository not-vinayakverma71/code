/// EventEmitter - EXACT 1:1 Translation of Node.js EventEmitter pattern
/// Consolidated from event_emitter.rs and events_exact_translation.rs
/// Used by ipc-server.ts and ipc-client.ts

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// Import event types from events_exact_translation.rs
pub use crate::events_exact_translation::*;

pub type EventHandler<T> = Arc<dyn Fn(T) + Send + Sync>;

pub struct EventEmitter<T> {
    handlers: Arc<RwLock<HashMap<String, Vec<EventHandler<T>>>>>,
}

impl<T: Clone> EventEmitter<T> {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register an event handler
    pub async fn on(&self, event: String, handler: EventHandler<T>) {
        let mut handlers = self.handlers.write().await;
        handlers.entry(event).or_insert_with(Vec::new).push(handler);
    }
    
    /// Emit an event to all registered handlers
    pub async fn emit(&self, event: String, data: T) {
        let handlers = self.handlers.read().await;
        if let Some(event_handlers) = handlers.get(&event) {
            for handler in event_handlers {
                handler(data.clone());
            }
        }
    }
    
    /// Remove all handlers for an event
    pub async fn remove_all_listeners(&self, event: String) {
        let mut handlers = self.handlers.write().await;
        handlers.remove(&event);
    }
}

/// IpcServerEventEmitter - for server-side events
pub struct IpcServerEventEmitter {
    connect_handlers: Arc<RwLock<Vec<Arc<dyn Fn(String) + Send + Sync>>>>,
    disconnect_handlers: Arc<RwLock<Vec<Arc<dyn Fn(String) + Send + Sync>>>>,
    task_command_handlers: Arc<RwLock<Vec<Arc<dyn Fn(String, crate::types_ipc::TaskCommand) + Send + Sync>>>>,
}

impl IpcServerEventEmitter {
    pub fn new() -> Self {
        Self {
            connect_handlers: Arc::new(RwLock::new(Vec::new())),
            disconnect_handlers: Arc::new(RwLock::new(Vec::new())),
            task_command_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn on_connect<F>(&self, handler: F) 
    where
        F: Fn(String) + Send + Sync + 'static
    {
        self.connect_handlers.write().await.push(Arc::new(handler));
    }
    
    pub async fn on_disconnect<F>(&self, handler: F)
    where
        F: Fn(String) + Send + Sync + 'static
    {
        self.disconnect_handlers.write().await.push(Arc::new(handler));
    }
    
    pub async fn on_task_command<F>(&self, handler: F)
    where
        F: Fn(String, crate::types_ipc::TaskCommand) + Send + Sync + 'static
    {
        self.task_command_handlers.write().await.push(Arc::new(handler));
    }
    
    pub async fn emit_connect(&self, client_id: String) {
        let handlers = self.connect_handlers.read().await;
        for handler in handlers.iter() {
            handler(client_id.clone());
        }
    }
    
    pub async fn emit_disconnect(&self, client_id: String) {
        let handlers = self.disconnect_handlers.read().await;
        for handler in handlers.iter() {
            handler(client_id.clone());
        }
    }
    
    pub async fn emit_task_command(&self, client_id: String, command: crate::types_ipc::TaskCommand) {
        let handlers = self.task_command_handlers.read().await;
        for handler in handlers.iter() {
            handler(client_id.clone(), command.clone());
        }
    }
}

/// IpcClientEventEmitter - for client-side events
pub struct IpcClientEventEmitter {
    connect_handlers: Arc<RwLock<Vec<Arc<dyn Fn() + Send + Sync>>>>,
    disconnect_handlers: Arc<RwLock<Vec<Arc<dyn Fn() + Send + Sync>>>>,
    ack_handlers: Arc<RwLock<Vec<Arc<dyn Fn(crate::types_ipc::Ack) + Send + Sync>>>>,
    task_event_handlers: Arc<RwLock<Vec<Arc<dyn Fn(crate::types_events::TaskEvent) + Send + Sync>>>>,
}

impl IpcClientEventEmitter {
    pub fn new() -> Self {
        Self {
            connect_handlers: Arc::new(RwLock::new(Vec::new())),
            disconnect_handlers: Arc::new(RwLock::new(Vec::new())),
            ack_handlers: Arc::new(RwLock::new(Vec::new())),
            task_event_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn on_connect<F>(&self, handler: F)
    where
        F: Fn() + Send + Sync + 'static
    {
        self.connect_handlers.write().await.push(Arc::new(handler));
    }
    
    pub async fn on_disconnect<F>(&self, handler: F)
    where
        F: Fn() + Send + Sync + 'static
    {
        self.disconnect_handlers.write().await.push(Arc::new(handler));
    }
    
    pub async fn on_ack<F>(&self, handler: F)
    where
        F: Fn(crate::types_ipc::Ack) + Send + Sync + 'static
    {
        self.ack_handlers.write().await.push(Arc::new(handler));
    }
    
    pub async fn on_task_event<F>(&self, handler: F)
    where
        F: Fn(crate::types_events::TaskEvent) + Send + Sync + 'static
    {
        self.task_event_handlers.write().await.push(Arc::new(handler));
    }
    
    pub async fn emit_connect(&self) {
        let handlers = self.connect_handlers.read().await;
        for handler in handlers.iter() {
            handler();
        }
    }
    
    pub async fn emit_disconnect(&self) {
        let handlers = self.disconnect_handlers.read().await;
        for handler in handlers.iter() {
            handler();
        }
    }
    
    pub async fn emit_ack(&self, ack: crate::types_ipc::Ack) {
        let handlers = self.ack_handlers.read().await;
        for handler in handlers.iter() {
            handler(ack.clone());
        }
    }
    
    pub async fn emit_task_event(&self, event: crate::types_events::TaskEvent) {
        let handlers = self.task_event_handlers.read().await;
        for handler in handlers.iter() {
            handler(event.clone());
        }
    }
}
