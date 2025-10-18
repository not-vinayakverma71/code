// AI Chat State Store
// Ported from Codex ExtensionStateContext.tsx
// Phase 0.3: Core UI state management

use std::sync::Arc;

use floem::reactive::{RwSignal, SignalUpdate, SignalGet, create_rw_signal};
use serde::{Deserialize, Serialize};

use crate::ai_bridge::{BridgeClient, ConnectionState, InboundMessage};

// ============================================================================
// Core State Structure
// ============================================================================

#[derive(Clone, Debug)]
pub struct AIChatState {
    // Connection
    pub bridge: Arc<BridgeClient>,
    pub connection_state: RwSignal<ConnectionState>,
    
    // Messages
    pub messages: RwSignal<Vec<ChatMessage>>,
    pub message_queue: RwSignal<Vec<QueuedMessage>>,
    pub streaming_text: RwSignal<String>,  // Live streaming response
    
    // Auto-approval toggles
    pub auto_approval_enabled: RwSignal<bool>,
    pub always_allow_read_only: RwSignal<bool>,
    pub always_allow_write: RwSignal<bool>,
    pub always_allow_execute: RwSignal<bool>,
    pub always_allow_browser: RwSignal<bool>,
    pub always_allow_mcp: RwSignal<bool>,
    pub always_allow_mode_switch: RwSignal<bool>,
    pub always_allow_subtasks: RwSignal<bool>,
    pub always_allow_followup: RwSignal<bool>,
    pub always_allow_update_todo: RwSignal<bool>,
    
    // Display preferences
    pub show_timestamps: RwSignal<bool>,
    pub show_task_timeline: RwSignal<bool>,
    pub history_preview_collapsed: RwSignal<bool>,
    pub reasoning_block_collapsed: RwSignal<bool>,
    pub hide_cost_below_threshold: RwSignal<f64>,
    
    // Sound & notifications
    pub sound_enabled: RwSignal<bool>,
    pub sound_volume: RwSignal<f64>,
    pub system_notifications_enabled: RwSignal<bool>,
    
    // Mode & workflow
    pub current_mode: RwSignal<String>,
    pub custom_instructions: RwSignal<Option<String>>,
    
    // Limits & thresholds
    pub allowed_max_requests: RwSignal<Option<usize>>,
    pub allowed_max_cost: RwSignal<Option<f64>>,
    pub max_concurrent_file_reads: RwSignal<usize>,
}

impl AIChatState {
    pub fn new(bridge: Arc<BridgeClient>) -> Self {
        Self {
            bridge,
            connection_state: create_rw_signal(ConnectionState::Disconnected),
            
            messages: create_rw_signal(Vec::new()),
            message_queue: create_rw_signal(Vec::new()),
            streaming_text: create_rw_signal(String::new()),
            
            // Defaults from ExtensionStateContext lines 244-245
            auto_approval_enabled: create_rw_signal(true),
            always_allow_read_only: create_rw_signal(true),
            always_allow_write: create_rw_signal(true),
            always_allow_execute: create_rw_signal(false),
            always_allow_browser: create_rw_signal(false),
            always_allow_mcp: create_rw_signal(false),
            always_allow_mode_switch: create_rw_signal(false),
            always_allow_subtasks: create_rw_signal(false),
            always_allow_followup: create_rw_signal(false),
            always_allow_update_todo: create_rw_signal(true),
            
            // Display defaults from lines 280-284
            show_timestamps: create_rw_signal(true),
            show_task_timeline: create_rw_signal(true),
            history_preview_collapsed: create_rw_signal(false),
            reasoning_block_collapsed: create_rw_signal(true),
            hide_cost_below_threshold: create_rw_signal(0.0),
            
            // Sound defaults from lines 224-227
            sound_enabled: create_rw_signal(false),
            sound_volume: create_rw_signal(0.5),
            system_notifications_enabled: create_rw_signal(false),
            
            // Mode defaults from line 249
            current_mode: create_rw_signal("code".to_string()),
            custom_instructions: create_rw_signal(None),
            
            // Limits from lines 275
            allowed_max_requests: create_rw_signal(None),
            allowed_max_cost: create_rw_signal(None),
            max_concurrent_file_reads: create_rw_signal(5),
        }
    }
    
    /// Poll for incoming messages (call from UI event loop)
    pub fn poll_messages(&self) {
        while let Some(msg) = self.bridge.try_receive() {
            self.handle_inbound_message(msg);
        }
    }
    
    /// Handle incoming message from backend
    fn handle_inbound_message(&self, msg: InboundMessage) {
        match msg {
            InboundMessage::ChatMessage { ts, message } => {
                self.messages.update(|msgs| {
                    msgs.push(ChatMessage {
                        ts,
                        content: message.text.unwrap_or_default(),
                        message_type: message.msg_type,
                        partial: message.partial,
                    });
                });
            }
            
            InboundMessage::PartialMessage { ts, partial } => {
                self.messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.ts == ts) {
                        msg.content = partial.text.unwrap_or_default();
                        msg.partial = partial.partial;
                    }
                });
            }
            
            InboundMessage::ConnectionStatus { status } => {
                self.connection_state.update(|s| {
                    *s = match status {
                        crate::ai_bridge::messages::ConnectionStatusType::Disconnected => {
                            ConnectionState::Disconnected
                        }
                        crate::ai_bridge::messages::ConnectionStatusType::Connecting => {
                            ConnectionState::Connecting
                        }
                        crate::ai_bridge::messages::ConnectionStatusType::Connected => {
                            ConnectionState::Connected
                        }
                        crate::ai_bridge::messages::ConnectionStatusType::Error => {
                            ConnectionState::Disconnected // Treat error as disconnected
                        }
                    };
                });
            }
            
            // Provider streaming responses
            InboundMessage::ProviderStreamChunk { content, .. } => {
                // Append chunk to streaming text signal
                self.streaming_text.update(|text| {
                    text.push_str(&content);
                });
            }
            
            InboundMessage::ProviderStreamDone { usage } => {
                // Move streaming text to messages
                let final_text = self.streaming_text.get();
                if !final_text.is_empty() {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    
                    self.messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            ts,
                            content: final_text.clone(),
                            message_type: crate::ai_bridge::messages::MessageType::Say,
                            partial: false,
                        });
                    });
                    
                    self.streaming_text.set(String::new());
                }
                
                if let Some(usage_info) = usage {
                    eprintln!("[AI Chat] Stream complete - tokens: {} prompt + {} completion = {} total",
                        usage_info.prompt_tokens, usage_info.completion_tokens, usage_info.total_tokens);
                }
            }
            
            InboundMessage::ProviderError { message } => {
                eprintln!("[AI Chat] Provider error: {}", message);
                // Clear streaming text on error
                self.streaming_text.set(String::new());
            }
            
            _ => {
                // Other message types handled in specific components
            }
        }
    }
}

// ============================================================================
// State Data Structures
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub ts: u64,
    pub content: String,
    pub message_type: crate::ai_bridge::messages::MessageType,
    pub partial: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub text: String,
    pub images: Vec<String>,
    pub timestamp: u64,
}

// ============================================================================
// Persistence (to Lapce settings)
// ============================================================================

/// Persisted settings that survive across sessions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AIChatSettings {
    // Auto-approval
    pub auto_approval_enabled: bool,
    pub always_allow_read_only: bool,
    pub always_allow_write: bool,
    pub always_allow_execute: bool,
    pub always_allow_browser: bool,
    pub always_allow_mcp: bool,
    pub always_allow_mode_switch: bool,
    pub always_allow_subtasks: bool,
    pub always_allow_followup: bool,
    pub always_allow_update_todo: bool,
    
    // Display
    pub show_timestamps: bool,
    pub show_task_timeline: bool,
    pub history_preview_collapsed: bool,
    pub reasoning_block_collapsed: bool,
    pub hide_cost_below_threshold: f64,
    
    // Sound
    pub sound_enabled: bool,
    pub sound_volume: f64,
    pub system_notifications_enabled: bool,
    
    // Mode
    pub current_mode: String,
    pub custom_instructions: Option<String>,
    
    // Limits
    pub allowed_max_requests: Option<usize>,
    pub allowed_max_cost: Option<f64>,
    pub max_concurrent_file_reads: usize,
}

impl Default for AIChatSettings {
    fn default() -> Self {
        Self {
            auto_approval_enabled: true,
            always_allow_read_only: true,
            always_allow_write: true,
            always_allow_execute: false,
            always_allow_browser: false,
            always_allow_mcp: false,
            always_allow_mode_switch: false,
            always_allow_subtasks: false,
            always_allow_followup: false,
            always_allow_update_todo: true,
            
            show_timestamps: true,
            show_task_timeline: true,
            history_preview_collapsed: false,
            reasoning_block_collapsed: true,
            hide_cost_below_threshold: 0.0,
            
            sound_enabled: false,
            sound_volume: 0.5,
            system_notifications_enabled: false,
            
            current_mode: "code".to_string(),
            custom_instructions: None,
            
            allowed_max_requests: None,
            allowed_max_cost: None,
            max_concurrent_file_reads: 5,
        }
    }
}

impl AIChatSettings {
    /// Load from state signals
    pub fn from_state(state: &AIChatState) -> Self {
        use floem::reactive::SignalGet;
        Self {
            auto_approval_enabled: state.auto_approval_enabled.get(),
            always_allow_read_only: state.always_allow_read_only.get(),
            always_allow_write: state.always_allow_write.get(),
            always_allow_execute: state.always_allow_execute.get(),
            always_allow_browser: state.always_allow_browser.get(),
            always_allow_mcp: state.always_allow_mcp.get(),
            always_allow_mode_switch: state.always_allow_mode_switch.get(),
            always_allow_subtasks: state.always_allow_subtasks.get(),
            always_allow_followup: state.always_allow_followup.get(),
            always_allow_update_todo: state.always_allow_update_todo.get(),
            
            show_timestamps: state.show_timestamps.get(),
            show_task_timeline: state.show_task_timeline.get(),
            history_preview_collapsed: state.history_preview_collapsed.get(),
            reasoning_block_collapsed: state.reasoning_block_collapsed.get(),
            hide_cost_below_threshold: state.hide_cost_below_threshold.get(),
            
            sound_enabled: state.sound_enabled.get(),
            sound_volume: state.sound_volume.get(),
            system_notifications_enabled: state.system_notifications_enabled.get(),
            
            current_mode: state.current_mode.get(),
            custom_instructions: state.custom_instructions.get(),
            
            allowed_max_requests: state.allowed_max_requests.get(),
            allowed_max_cost: state.allowed_max_cost.get(),
            max_concurrent_file_reads: state.max_concurrent_file_reads.get(),
        }
    }
    
    /// Apply to state signals
    pub fn apply_to_state(&self, state: &AIChatState) {
        state.auto_approval_enabled.update(|v| *v = self.auto_approval_enabled);
        state.always_allow_read_only.update(|v| *v = self.always_allow_read_only);
        state.always_allow_write.update(|v| *v = self.always_allow_write);
        state.always_allow_execute.update(|v| *v = self.always_allow_execute);
        state.always_allow_browser.update(|v| *v = self.always_allow_browser);
        state.always_allow_mcp.update(|v| *v = self.always_allow_mcp);
        state.always_allow_mode_switch.update(|v| *v = self.always_allow_mode_switch);
        state.always_allow_subtasks.update(|v| *v = self.always_allow_subtasks);
        state.always_allow_followup.update(|v| *v = self.always_allow_followup);
        state.always_allow_update_todo.update(|v| *v = self.always_allow_update_todo);
        
        state.show_timestamps.update(|v| *v = self.show_timestamps);
        state.show_task_timeline.update(|v| *v = self.show_task_timeline);
        state.history_preview_collapsed.update(|v| *v = self.history_preview_collapsed);
        state.reasoning_block_collapsed.update(|v| *v = self.reasoning_block_collapsed);
        state.hide_cost_below_threshold.update(|v| *v = self.hide_cost_below_threshold);
        
        state.sound_enabled.update(|v| *v = self.sound_enabled);
        state.sound_volume.update(|v| *v = self.sound_volume);
        state.system_notifications_enabled.update(|v| *v = self.system_notifications_enabled);
        
        state.current_mode.update(|v| *v = self.current_mode.clone());
        state.custom_instructions.update(|v| *v = self.custom_instructions.clone());
        
        state.allowed_max_requests.update(|v| *v = self.allowed_max_requests);
        state.allowed_max_cost.update(|v| *v = self.allowed_max_cost);
        state.max_concurrent_file_reads.update(|v| *v = self.max_concurrent_file_reads);
    }
}
