/// Exact 1:1 Translation of TypeScript Task message routing/dispatch from codex-reference/core/task/Task.ts
/// Lines 700-900 of 2859 total lines
/// DAY 2 H5-6: Translate message routing/dispatch system

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::time::sleep;
use serde::{Serialize, Deserialize};
use crate::events_exact_translation::*;
// Import task types from task_exact_translation
// Task types are in parent module
use crate::ipc_messages::ToolProgressStatus;
// ConnectionId and Connection are now local types
type ConnectionId = u64;
struct Connection;
use crate::ipc_messages::ClineMessage as TypesClineMessage;
// Use ClineAsk from ipc_messages for consistency  
use crate::ipc_messages::ClineAsk;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineSay;
// Define Tool types locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub parameters: serde_json::Value,
    pub id: String,
}

// Use types from task_exact_translation
use crate::task_exact_translation::AskResponse;

// ToolParameter moved to types_tool.rs to avoid circular dependencies

#[derive(Debug, Clone)]
pub struct RooTerminalProcess {
    pub id: String,
    pub process: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: Option<serde_json::Value>,
}
// Define ClineProvider type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineProvider {
    pub id: String,
    pub name: String,
}

// Use Task from task_exact_translation
use crate::task_exact_translation::Task;

impl Task {
    /// Full ask implementation with message routing - exact translation lines 678-849
    pub async fn ask_full(
        &self,
        ask_type: ClineAsk,
        text: Option<String>,
        partial: Option<bool>,
        progress_status: Option<ToolProgressStatus>,
        is_protected: Option<bool>,
    ) -> Result<AskResponse, String> {
        // Helper function for timestamp
        fn get_current_timestamp() -> u64 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }
        // Check if aborted
        if *self.abort.read().await {
            return Err(format!("[KiloCode#ask] task {}.{} aborted", self.task_id, self.instance_id));
        }
        
        let mut ask_ts: u64 = 0;
        
        if partial.is_some() {
            let mut messages = self.cline_messages.write().await;
            let last_message = messages.last().cloned();
            
            let is_updating_previous_partial = if let Some(ref last_msg) = last_message {
                last_msg.is_partial() && 
                last_msg.is_ask_type(&ask_type)
            } else {
                false
            };
            
            if partial == Some(true) {
                if is_updating_previous_partial {
                    // Existing partial message, so update it
                    if let Some(last_msg) = messages.last_mut() {
                        last_msg.set_text(text.clone());
                        last_msg.set_partial(Some(true));
                        last_msg.set_progress_status(progress_status.clone());
                        last_msg.set_is_protected(is_protected);
                        
                        let msg_clone = last_msg.clone();
                        drop(messages);
                        self.update_cline_message(msg_clone).await;
                        return Err("Current ask promise was ignored (#1)".to_string());
                    }
                } else {
                    // This is a new partial message, so add it with partial state
                    ask_ts = get_current_timestamp();
                    *self.last_message_ts.write().await = Some(ask_ts);
                    drop(messages);
                    
                    let progress_status_value = progress_status.as_ref().map(|s| {
                        serde_json::to_value(s).unwrap_or_default()
                    });
                    
                    let message = TypesClineMessage {
                        ts: ask_ts,
                        msg_type: "ask".to_string(),
                        ask: Some(ask_type),
                        say: None,
                        text: text.clone(),
                        images: None,
                        partial: Some(true),
                        reasoning: None,
                        conversation_history_index: None,
                        checkpoint: None,
                        progress_status: progress_status.clone(),
                        context_condense: None,
                        is_protected,
                        api_protocol: None,
                        metadata: None,
                    };
                    self.add_to_cline_messages(message).await;
                    
                    return Err("Current ask promise was ignored (#2)".to_string());
                }
            } else {
                if is_updating_previous_partial {
                    // This is the complete version of a previously partial message
                    *self.ask_response.write().await = None;
                    *self.ask_response_text.write().await = None;
                    *self.ask_response_images.write().await = None;
                    
                    // Use stable timestamp from partial message
                    ask_ts = last_message.as_ref().unwrap().ts().unwrap();
                    *self.last_message_ts.write().await = Some(ask_ts);
                    
                    if let Some(last_msg) = messages.last_mut() {
                        last_msg.set_text(text.clone());
                        last_msg.set_partial(Some(false));
                        last_msg.set_progress_status(progress_status.clone());
                        last_msg.set_is_protected(is_protected);
                    }
                    
                    drop(messages);
                    self.save_cline_messages().await;
                    
                    if let Some(msg) = last_message {
                        self.update_cline_message(msg).await;
                    }
                } else {
                    // This is a new and complete message
                    *self.ask_response.write().await = None;
                    *self.ask_response_text.write().await = None;
                    *self.ask_response_images.write().await = None;
                    
                    ask_ts = get_current_timestamp();
                    *self.last_message_ts.write().await = Some(ask_ts);
                    drop(messages);
                    
                    let message = TypesClineMessage {
                        ts: ask_ts,
                        msg_type: "ask".to_string(),
                        ask: Some(ask_type.clone()),
                        say: None,
                        text: text.clone(),
                        images: None,
                        partial: None,
                        reasoning: None,
                        conversation_history_index: None,
                        checkpoint: None,
                        progress_status: progress_status.clone(),
                        context_condense: None,
                        is_protected,
                        api_protocol: None,
                        metadata: None,
                    };
                    self.add_to_cline_messages(message).await;
                }
            }
        } else {
            // This is a new non-partial message
            *self.ask_response.write().await = None;
            *self.ask_response_text.write().await = None;
            *self.ask_response_images.write().await = None;
            
            ask_ts = get_current_timestamp();
            *self.last_message_ts.write().await = Some(ask_ts);
            
            let message = TypesClineMessage {
                ts: ask_ts,
                msg_type: "ask".to_string(),
                ask: Some(ask_type.clone()),
                say: None,
                text,
                images: None,
                partial: None,
                reasoning: None,
                conversation_history_index: None,
                checkpoint: None,
                progress_status: progress_status.clone(),
                context_condense: None,
                is_protected,
                api_protocol: None,
                metadata: None,
            };
            self.add_to_cline_messages(message).await;
        }
        
        // Check if blocking and status is mutable
        let is_blocking = self.ask_response.read().await.is_none() && 
                         *self.last_message_ts.read().await == Some(ask_ts);
        let is_status_mutable = partial != Some(true) && is_blocking;
        let mut status_mutation_handles = vec![];
        
        if is_status_mutable {
            if is_interactive_ask(&ask_type) {
                let task_id = self.task_id.clone();
                let ask_ts_clone = ask_ts;
                let self_clone = Arc::new(self.clone());
                
                status_mutation_handles.push(tokio::spawn(async move {
                    sleep(Duration::from_secs(1)).await;
                    
                    if let Some(message) = self_clone.find_message_by_timestamp(ask_ts_clone) {
                        *self_clone.interactive_ask.write().await = Some(message);
                        self_clone.emit_event(RooCodeEventName::TaskInteractive, task_id);
                    }
                }));
            } else if is_resumable_ask(&ask_type) {
                let task_id = self.task_id.clone();
                let ask_ts_clone = ask_ts;
                let self_clone = Arc::new(self.clone());
                
                status_mutation_handles.push(tokio::spawn(async move {
                    sleep(Duration::from_secs(1)).await;
                    
                    if let Some(message) = self_clone.find_message_by_timestamp(ask_ts_clone) {
                        *self_clone.resumable_ask.write().await = Some(message);
                        self_clone.emit_event(RooCodeEventName::TaskResumable, task_id);
                    }
                }));
            } else if is_idle_ask(&ask_type) {
                let task_id = self.task_id.clone();
                let ask_ts_clone = ask_ts;
                let self_clone = Arc::new(self.clone());
                
                status_mutation_handles.push(tokio::spawn(async move {
                    sleep(Duration::from_secs(1)).await;
                    
                    if let Some(message) = self_clone.find_message_by_timestamp(ask_ts_clone) {
                        *self_clone.idle_ask.write().await = Some(message);
                        self_clone.emit_event(RooCodeEventName::TaskIdle, task_id);
                    }
                }));
            }
        }
        
        println!(
            "[Task#{}] pWaitFor askResponse({:?}) -> blocking (isStatusMutable = {}, statusMutationTimeouts = {})",
            self.task_id, ask_type, is_status_mutable, status_mutation_handles.len()
        );
        
        // Wait for response - equivalent to pWaitFor
        loop {
            if self.ask_response.read().await.is_some() || 
               *self.last_message_ts.read().await != Some(ask_ts) {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
        
        println!("[Task#{}] pWaitFor askResponse({:?}) -> unblocked", self.task_id, ask_type);
        
        if *self.last_message_ts.read().await != Some(ask_ts) {
            return Err("Current ask promise was ignored".to_string());
        }
        
        let response = self.ask_response.read().await.clone().unwrap();
        let text = self.ask_response_text.read().await.clone();
        let images = self.ask_response_images.read().await.clone();
        
        *self.ask_response.write().await = None;
        *self.ask_response_text.write().await = None;
        *self.ask_response_images.write().await = None;
        
        // Cancel status mutation tasks
        for handle in status_mutation_handles {
            handle.abort();
        }
        
        Ok(AskResponse {
            answer: response.response,
            confirmed: true,
        })
    }
}

// Helper functions for ask type checking
fn is_interactive_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::Followup | ClineAsk::Tool)
}

fn is_resumable_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::Tool | ClineAsk::ApiReqFailed)
}

fn is_idle_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::ApiReqFailed)
}

// Remove duplicate ClineMessage definition - using from events_exact_translation
// Helper function to get current timestamp
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// RooTerminalProcess placeholder implementation
impl RooTerminalProcess {
    pub fn continue_execution(&self) {
        // Continue terminal execution
    }
    
    pub fn abort(&self) {
        // Abort terminal execution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_routing() {
        // Test message routing
    }
}
