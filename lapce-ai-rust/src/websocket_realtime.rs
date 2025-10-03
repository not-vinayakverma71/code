/// WebSocket Real-time Updates - Day 35 AM
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealtimeMessage {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

pub struct WebSocketManager {
    tx: broadcast::Sender<RealtimeMessage>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx }
    }
    
    pub fn broadcast(&self, msg: RealtimeMessage) {
        let _ = self.tx.send(msg);
    }
    
    pub async fn handle_socket(&self, socket: WebSocket) {
        let mut rx = self.tx.subscribe();
        let (mut sender, mut receiver) = socket.split();
        
        // Spawn task to forward messages
        let tx_clone = self.tx.clone();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    let _ = sender.send(axum::extract::ws::Message::Text(text)).await;
                }
            }
        });
        
        // Handle incoming messages
        while let Some(Ok(msg)) = receiver.next().await {
            if let axum::extract::ws::Message::Text(text) = msg {
                if let Ok(parsed) = serde_json::from_str::<RealtimeMessage>(&text) {
                    let _ = tx_clone.send(parsed);
                }
            }
        }
    }
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(manager): State<Arc<WebSocketManager>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| manager.handle_socket(socket))
}
