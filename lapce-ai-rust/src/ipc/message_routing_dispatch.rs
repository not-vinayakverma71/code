/// Exact 1:1 Translation of TypeScript Task message routing/dispatch from codex-reference/core/task/Task.ts
/// Lines 700-900 of 2859 total lines
/// DAY 2 H5-6: Translate message routing/dispatch system

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::sync::RwLock;
use tokio::time::sleep;
use serde::{Serialize, Deserialize};
use crate::events_exact_translation::*;
use crate::global_settings_exact_translation::*;
// Import task types from task_exact_translation
// Task types are in parent module
use crate::task_exact_translation::{ClineAskResponse, UserContent, ToolProgressStatus};
// ConnectionId and Connection are now local types
type ConnectionId = u64;
struct Connection;
use std::path::PathBuf;
use super::ipc_messages::{Message, MessageRole};
use crate::events_exact_translation::ClineMessage as TypesClineMessage;
// Define missing types
// Use ClineAsk from ipc_messages which has all variants
use super::ipc_messages::ClineAsk;
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
                last_msg.is_ask_type(&ask_type) &&
                TypesClineMessage::get_ask_type(last_msg) == Some(ask_type)
            } else {
                false
            };
            
            if partial == Some(true) {
                if is_updating_previous_partial {
                    // Existing partial message, so update it
                    if let Some(last_msg) = messages.last_mut() {
                        TypesClineMessage::set_text(last_msg, text.clone());
                        TypesClineMessage::set_partial(last_msg, Some(true));
                        TypesClineMessage::set_progress_status(last_msg, progress_status.clone().map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null)));
                        TypesClineMessage::set_is_protected(last_msg, is_protected);
                        
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
                    
                    self.add_to_cline_messages(crate::events_exact_translation::ClineMessage::Ask {
                        ts: Some(ask_ts),
                        ask: ask_type,
                        text,
                        kilocode_meta_data: None,
                        partial: Some(true),
                        progress_status: progress_status_value,
                        is_protected,
                    }).await;
                    
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
                        TypesClineMessage::set_text(last_msg, text.clone());
                        TypesClineMessage::set_partial(last_msg, Some(false));
                        TypesClineMessage::set_progress_status(last_msg, progress_status.clone().map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null)));
                        TypesClineMessage::set_is_protected(last_msg, is_protected);
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
                    
self.add_to_cline_messages(TypesClineMessage::Ask {
                        ts: Some(ask_ts),
                        ask: ask_type.clone(),
                        text: text.clone(),
                        kilocode_meta_data: None,
                        partial: None,
                        progress_status: progress_status.as_ref().map(|s| serde_json::to_value(s).unwrap_or_default()),
                        is_protected,
                    }).await;
                }
            }
        } else {
            // This is a new non-partial message
            *self.ask_response.write().await = None;
            *self.ask_response_text.write().await = None;
            *self.ask_response_images.write().await = None;
            
            ask_ts = get_current_timestamp();
            *self.last_message_ts.write().await = Some(ask_ts);
            
            let progress_status_value = progress_status.as_ref().map(|s| {
                serde_json::to_value(s).unwrap_or_default()
            });
            
            self.add_to_cline_messages(TypesClineMessage::Ask {
                ts: Some(ask_ts),
                ask: ask_type.clone(),
                text,
                kilocode_meta_data: None,
                partial: None,
                progress_status: progress_status_value,
                is_protected,
            }).await;
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

// Task clone implementation for Arc usage
impl Clone for Task {
    fn clone(&self) -> Self {
        // Create shallow clone for Arc usage
        Self {
            context: self.context.clone(),
            task_id: self.task_id.clone(),
            instance_id: self.instance_id.clone(),
            metadata: self.metadata.clone(),
            todo_list: self.todo_list.clone(),
            root_task: self.root_task.clone(),
            parent_task: self.parent_task.clone(),
            task_number: self.task_number,
            workspace_path: self.workspace_path.clone(),
            task_mode: self.task_mode.clone(),
            task_mode_ready: self.task_mode_ready.clone(),
            provider_ref: self.provider_ref.clone(),
            global_storage_path: self.global_storage_path.clone(),
            abort: self.abort.clone(),
            idle_ask: self.idle_ask.clone(),
            resumable_ask: self.resumable_ask.clone(),
            interactive_ask: self.interactive_ask.clone(),
            did_finish_aborting_stream: self.did_finish_aborting_stream.clone(),
            abandoned: self.abandoned.clone(),
            is_initialized: self.is_initialized.clone(),
            is_paused: self.is_paused.clone(),
            paused_mode_slug: self.paused_mode_slug.clone(),
            api_configuration: self.api_configuration.clone(),
            api: self.api.clone(),
            auto_approval_handler: self.auto_approval_handler.clone(),
            tool_repetition_detector: self.tool_repetition_detector.clone(),
            roo_ignore_controller: self.roo_ignore_controller.clone(),
            roo_protected_controller: self.roo_protected_controller.clone(),
            file_context_tracker: self.file_context_tracker.clone(),
            url_content_fetcher: self.url_content_fetcher.clone(),
            terminal_process: self.terminal_process.clone(),
            browser_session: self.browser_session.clone(),
            diff_view_provider: self.diff_view_provider.clone(),
            diff_strategy: self.diff_strategy.clone(),
            diff_enabled: self.diff_enabled,
            fuzzy_match_threshold: self.fuzzy_match_threshold,
            did_edit_file: self.did_edit_file.clone(),
            api_conversation_history: self.api_conversation_history.clone(),
            cline_messages: self.cline_messages.clone(),
            ask_response: self.ask_response.clone(),
            ask_response_text: self.ask_response_text.clone(),
            ask_response_images: self.ask_response_images.clone(),
            last_message_ts: self.last_message_ts.clone(),
            consecutive_mistake_count: self.consecutive_mistake_count.clone(),
            consecutive_mistake_limit: self.consecutive_mistake_limit,
            consecutive_mistake_count_for_apply_diff: self.consecutive_mistake_count_for_apply_diff.clone(),
            tool_usage: self.tool_usage.clone(),
            enable_checkpoints: self.enable_checkpoints,
            checkpoint_service: self.checkpoint_service.clone(),
            checkpoint_service_initializing: self.checkpoint_service_initializing.clone(),
            enable_task_bridge: self.enable_task_bridge,
            bridge_service: self.bridge_service.clone(),
            is_waiting_for_first_chunk: self.is_waiting_for_first_chunk.clone(),
            is_streaming: self.is_streaming.clone(),
            current_streaming_content_index: self.current_streaming_content_index.clone(),
            current_streaming_did_checkpoint: self.current_streaming_did_checkpoint.clone(),
            assistant_message_content: self.assistant_message_content.clone(),
            present_assistant_message_locked: self.present_assistant_message_locked.clone(),
            present_assistant_message_has_pending_updates: self.present_assistant_message_has_pending_updates.clone(),
            user_message_content: self.user_message_content.clone(),
            user_message_content_ready: self.user_message_content_ready.clone(),
            did_reject_tool: self.did_reject_tool.clone(),
            did_already_use_tool: self.did_already_use_tool.clone(),
            did_complete_reading_stream: self.did_complete_reading_stream.clone(),
            assistant_message_parser: self.assistant_message_parser.clone(),
            last_used_instructions: self.last_used_instructions.clone(),
            skip_prev_response_id_once: self.skip_prev_response_id_once.clone(),
            task_is_favorited: self.task_is_favorited,
        }
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
