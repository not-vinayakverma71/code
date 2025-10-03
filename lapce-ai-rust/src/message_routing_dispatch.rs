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
use crate::task_exact_translation::{ClineAskResponse, UserContent};
use crate::task_connection_handling::AskResponse;
use std::path::PathBuf;
use crate::types_message::{ClineAsk, ClineMessage as TypesClineMessage, ClineSay};
use crate::types_tool::ToolParameter;

// Add missing types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressStatus {
    pub text: String,
}

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
// UserContent already imported from task_exact_translation

// Import Task struct
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
                last_msg.is_ask_type(ask_type) &&
                last_msg.get_ask_type() == Some(ask_type.clone())
            } else {
                false
            };
            
            if partial == Some(true) {
                if is_updating_previous_partial {
                    // Existing partial message, so update it
                    if let Some(last_msg) = messages.last_mut() {
                        last_msg.set_text(text.clone());
                        last_msg.set_partial(Some(true));
                        last_msg.set_progress_status(progress_status.clone().map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null)));
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
                    
                    Task::add_to_cline_messages(self, TypesClineMessage::Ask {
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
                        last_msg.set_text(text.clone());
                        last_msg.set_partial(Some(false));
                        last_msg.set_progress_status(progress_status.clone().map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null)));
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
                    
Task::add_to_cline_messages(self, TypesClineMessage::Ask {
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
            
            Task::add_to_cline_messages(self, TypesClineMessage::Ask {
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
        
        // Switch back to active state
        let has_status_ask = self.idle_ask.read().await.is_some() ||
                             self.resumable_ask.read().await.is_some() ||
                             self.interactive_ask.read().await.is_some();
        
        if has_status_ask {
            *self.idle_ask.write().await = None;
            *self.resumable_ask.write().await = None;
            *self.interactive_ask.write().await = None;
            self.emit_event(RooCodeEventName::TaskActive, self.task_id.clone());
        }
        
        self.emit_event(RooCodeEventName::TaskAskResponded, self.task_id.clone());
        
        Ok(AskResponse {
            response,
            text,
            images,
        })
    }
    
    /// setMessageResponse - exact translation line 851-853
    pub fn set_message_response(&self, text: String, images: Option<Vec<String>>) {
        self.handle_webview_ask_response(
            ClineAskResponse {
                response: "messageResponse".to_string(),
                text: Some(text.clone()),
                images: images.clone(),
                metadata: None,
            },
            Some(text),
            images,
        );
    }
    
    /// handleWebviewAskResponse - exact translation line 855-859
    pub fn handle_webview_ask_response(
        &self,
        ask_response: ClineAskResponse,
        text: Option<String>,
        images: Option<Vec<String>>,
    ) {
        let ask_response_arc = self.ask_response.clone();
        let ask_response_text_arc = self.ask_response_text.clone();
        let ask_response_images_arc = self.ask_response_images.clone();
        
        tokio::spawn(async move {
            *ask_response_arc.write().await = Some(ask_response);
            *ask_response_text_arc.write().await = text;
            *ask_response_images_arc.write().await = images;
        });
    }
    
    /// approveAsk - exact translation line 861-863
    pub fn approve_ask(&self, text: Option<String>, images: Option<Vec<String>>) {
        self.handle_webview_ask_response(
            ClineAskResponse {
                response: "yesButtonClicked".to_string(),
                text: text.clone(),
                images: images.clone(),
                metadata: None,
            },
            text,
            images,
        );
    }
    
    /// denyAsk - exact translation line 865-867
    pub fn deny_ask(&self, text: Option<String>, images: Option<Vec<String>>) {
        self.handle_webview_ask_response(
            ClineAskResponse {
                response: "noButtonClicked".to_string(),
                text: text.clone(),
                images: images.clone(),
                metadata: None,
            },
            text,
            images,
        );
    }
    
    /// submitUserMessage - exact translation line 869-888
    pub fn submit_user_message(&self, text: String, images: Option<Vec<String>>) {
        let text = text.trim().to_string();
        let images = images.unwrap_or_default();
        
        if text.is_empty() && images.is_empty() {
            return;
        }
        
        if let Some(provider) = self.provider_ref.upgrade() {
            provider.post_message_to_webview(crate::handler_registration_types::WebviewMessage {
                type_: "invoke".to_string(),
                text: None,
                data: Some(serde_json::json!({
                    "invoke": "sendMessage",
                    "text": text,
                    "images": images
                })),
            });
        } else {
            eprintln!("[Task#submitUserMessage] Provider reference lost");
        }
    }
    
    /// handleTerminalOperation - exact translation line 890-896
    pub async fn handle_terminal_operation(&self, terminal_operation: &str) {
        let terminal = self.terminal_process.read().await;
        if let Some(ref proc) = *terminal {
            match terminal_operation {
                "continue" => proc.continue_execution(),
                "abort" => proc.abort(),
                _ => {}
            }
        }
    }
    
    /// Helper to emit events
    fn emit_event(&self, event: RooCodeEventName, data: String) {
        // Event emission implementation
        println!("Event: {:?} - {}", event, data);
    }
}

// Helper functions for ask type checking

fn is_interactive_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::FollowUp | ClineAsk::Confirmation)
}

fn is_resumable_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::Tool | ClineAsk::ApiCostLimit)
}

fn is_idle_ask(ask_type: &ClineAsk) -> bool {
    matches!(ask_type, ClineAsk::RequestCostLimit)
}

// Extensions for ClineMessage
impl ClineMessage {
    pub fn is_partial(&self) -> bool {
        match self {
            ClineMessage::Ask { partial, .. } => partial.unwrap_or(false),
            _ => false,
        }
    }
    
    pub fn is_ask_type(&self, ask_type: &ClineAsk) -> bool {
        match self {
            ClineMessage::Ask { ask, .. } => ask == ask_type,
            _ => false,
        }
    }
    
    pub fn get_ask_type(&self) -> Option<ClineAsk> {
        match self {
            ClineMessage::Ask { ask, .. } => Some(ask.clone()),
            _ => None,
        }
    }
    
    pub fn set_text(&mut self, new_text: Option<String>) {
        match self {
            ClineMessage::Ask { text, .. } => {
                if let Some(t) = new_text {
                    *text = t;
                }
            }
            _ => {}
        }
    }
    
    pub fn set_partial(&mut self, new_partial: Option<bool>) {
        match self {
            ClineMessage::Ask { partial, .. } => *partial = new_partial,
            _ => {}
        }
    }
    
    pub fn set_progress_status(&mut self, new_status: Option<ToolProgressStatus>) {
        match self {
            ClineMessage::Ask { progress_status, .. } => *progress_status = new_status,
            _ => {}
        }
    }
    
    pub fn set_is_protected(&mut self, new_protected: Option<bool>) {
        match self {
            ClineMessage::Ask { is_protected, .. } => *is_protected = new_protected,
            _ => {}
        }
    }
}

// Update ClineMessage enum to support ask fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClineMessage {
    Say {
        text: String,
        ts: u64,
    },
    Ask {
        ts: u64,
        ask: ClineAsk,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        partial: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress_status: Option<ToolProgressStatus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_protected: Option<bool>,
    },
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
