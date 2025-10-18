/// Exact 1:1 Translation of TypeScript humanRelay from codex-reference/activate/humanRelay.ts
/// DAY 9 H3-4: Translate humanRelay.ts

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Human relay response structure
#[derive(Debug, Clone)]
pub struct HumanRelayResponse {
    pub request_id: String,
    pub text: Option<String>,
    pub cancelled: Option<bool>,
}

/// Callback type for human relay responses
pub type HumanRelayCallback = Arc<dyn Fn(Option<String>) + Send + Sync>;

/// Global callbacks mapping - exact translation line 2
use once_cell::sync::Lazy;

static HUMAN_RELAY_CALLBACKS: Lazy<Arc<RwLock<HashMap<String, HumanRelayCallback>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Register a callback function for human relay response - exact translation lines 9-10
pub async fn register_human_relay_callback(request_id: String, callback: HumanRelayCallback) {
    HUMAN_RELAY_CALLBACKS.write().await.insert(request_id, callback);
}

/// Unregister callback - exact translation line 12
pub async fn unregister_human_relay_callback(request_id: &str) -> bool {
    HUMAN_RELAY_CALLBACKS.write().await.remove(request_id).is_some()
}

/// Handle human relay response - exact translation lines 14-26
pub async fn handle_human_relay_response(response: HumanRelayResponse) {
    let callbacks = HUMAN_RELAY_CALLBACKS.read().await;
    
    if let Some(callback) = callbacks.get(&response.request_id) {
        // Clone callback to avoid holding lock during execution
        let callback = callback.clone();
        
        // Drop read lock before calling callback
        drop(callbacks);
        
        if response.cancelled.unwrap_or(false) {
            callback(None);
        } else {
            callback(response.text);
        }
        
        // Remove callback after use
        HUMAN_RELAY_CALLBACKS.write().await.remove(&response.request_id);
    }
}

/// Get current callback count (for testing)
pub async fn get_callback_count() -> usize {
    HUMAN_RELAY_CALLBACKS.read().await.len()
}

/// Clear all callbacks (for testing/cleanup)
pub async fn clear_all_callbacks() {
    HUMAN_RELAY_CALLBACKS.write().await.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    #[tokio::test]
    async fn test_register_and_handle_callback() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        
        let callback: HumanRelayCallback = Arc::new(move |response| {
            assert_eq!(response, Some("test response".to_string()));
            called_clone.store(true, Ordering::SeqCst);
        });
        
        register_human_relay_callback("test-id".to_string(), callback).await;
        assert_eq!(get_callback_count().await, 1);
        
        let response = HumanRelayResponse {
            request_id: "test-id".to_string(),
            text: Some("test response".to_string()),
            cancelled: Some(false),
        };
        
        handle_human_relay_response(response).await;
        
        // Wait a bit for async operations
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(get_callback_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_cancelled_response() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();
        
        let callback: HumanRelayCallback = Arc::new(move |response| {
            assert_eq!(response, None);
            called_clone.store(true, Ordering::SeqCst);
        });
        
        register_human_relay_callback("cancel-id".to_string(), callback).await;
        
        let response = HumanRelayResponse {
            request_id: "cancel-id".to_string(),
            text: Some("ignored text".to_string()),
            cancelled: Some(true),
        };
        
        handle_human_relay_response(response).await;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(get_callback_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_unregister_callback() {
        let callback: HumanRelayCallback = Arc::new(|_| {});
        
        register_human_relay_callback("unreg-id".to_string(), callback).await;
        assert_eq!(get_callback_count().await, 1);
        
        let removed = unregister_human_relay_callback("unreg-id").await;
        assert!(removed);
        assert_eq!(get_callback_count().await, 0);
        
        let removed_again = unregister_human_relay_callback("unreg-id").await;
        assert!(!removed_again);
    }
}
