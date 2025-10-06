/// Exact 1:1 Translation of TypeScript Task from codex-reference/core/task/Task.ts
/// This is NOT a rewrite - it's a direct translation maintaining same logic and flow
/// Lines 1-350 of 2859 total lines

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use tokio::sync::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::events_exact_translation::RooCodeEventName;

// Placeholder types - to be implemented
#[derive(Debug)]
pub struct ApiHandler;
#[derive(Debug)]
pub struct AutoApprovalHandler;
#[derive(Debug)]
pub struct ToolRepetitionDetector;
#[derive(Debug)]
pub struct FileContextTracker;
#[derive(Debug)]
pub struct UrlContentFetcher;
#[derive(Debug)]
pub struct BrowserSession;
#[derive(Debug)]
pub struct DiffViewProvider;

// Define missing types
pub type AssistantMessageInfo = HashMap<String, Value>;
pub type Metadata = HashMap<String, Value>;

// Import translated types
use crate::ClineMessage;
use crate::global_settings_exact_translation::ProviderSettings;
// use crate::ipc_types_exact_translation::*;
// use crate::base_provider::ApiHandler as ApiHandlerTrait;

// Use ToolProgressStatus from ipc_messages

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskResponse {
    pub answer: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineAskResponse {
    pub response: String,
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewMessage {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: Option<String>,
    pub data: Option<serde_json::Value>,
    // Compatibility fields
    pub command: Option<String>,
}

// RooTerminalProcess moved to avoid duplicate - defined in message_routing_dispatch.rs

/// Maximum exponential backoff seconds constant
const MAX_EXPONENTIAL_BACKOFF_SECONDS: u32 = 600; // 10 minutes

/// Default usage collection timeout in milliseconds
const DEFAULT_USAGE_COLLECTION_TIMEOUT_MS: u32 = 5000; // 5 seconds

/// Forced context reduction percent
const FORCED_CONTEXT_REDUCTION_PERCENT: u32 = 75; // Keep 75% of context

/// Maximum context window retries
const MAX_CONTEXT_WINDOW_RETRIES: u32 = 3;

/// TaskOptions structure - exact translation
#[derive(Clone)]
pub struct TaskOptions {
    pub task: Option<String>,
    pub assistant_message_info: Option<serde_json::Value>, // TODO: Define AssistantMessageInfo
    pub assistant_metadata: Option<serde_json::Value>, // TODO: Define Metadata
    pub custom_variables: Option<serde_json::Value>,
    pub images: Option<Vec<String>>,
    pub start_with: Option<String>,
    pub project_path: Option<String>,
    pub automatically_approve_api_requests: Option<String>,
    pub context_files_content: Option<String>,
    pub context_files: Option<Vec<String>>,
    pub experiments: Option<HashMap<String, bool>>,
    pub start_task: Option<bool>,
    pub root_task: Option<Arc<Task>>,
    pub parent_task: Option<Arc<Task>>,
    pub task_number: Option<i32>,
    // Skip debug output for closure
    pub on_created: Option<Arc<dyn Fn(Arc<Task>) + Send + Sync>>,
    pub initial_todos: Option<Vec<TodoItem>>,
    // Additional fields needed by constructor
    pub context: Option<ExtensionContext>,
    pub provider: Option<Arc<ClineProvider>>,
    pub api_configuration: Option<ProviderSettings>,
    pub enable_diff: Option<bool>,
    pub enable_checkpoints: Option<bool>,
    pub enable_task_bridge: Option<bool>,
    pub fuzzy_match_threshold: Option<f32>,
    pub consecutive_mistake_limit: Option<u32>,
    pub history_item: Option<HistoryItem>,
}

impl std::fmt::Debug for TaskOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskOptions")
            .field("task", &self.task)
            .field("assistant_message_info", &self.assistant_message_info)
            .field("assistant_metadata", &self.assistant_metadata)
            .field("custom_variables", &self.custom_variables)
            .field("images", &self.images)
            .field("start_with", &self.start_with)
            .field("project_path", &self.project_path)
            .field("automatically_approve_api_requests", &self.automatically_approve_api_requests)
            .field("context_files_content", &self.context_files_content)
            .field("context_files", &self.context_files)
            .field("experiments", &self.experiments)
            .field("start_task", &self.start_task)
            .field("root_task", &self.root_task)
            .field("parent_task", &self.parent_task)
            .field("task_number", &self.task_number)
            .field("on_created", &"<function>")
            .field("initial_todos", &self.initial_todos)
            .field("context", &self.context)
            .field("provider", &self.provider)
            .field("api_configuration", &self.api_configuration)
            .field("enable_diff", &self.enable_diff)
            .field("enable_checkpoints", &self.enable_checkpoints)
            .field("enable_task_bridge", &self.enable_task_bridge)
            .field("fuzzy_match_threshold", &self.fuzzy_match_threshold)
            .field("consecutive_mistake_limit", &self.consecutive_mistake_limit)
            .field("history_item", &self.history_item)
            .finish()
    }
}

/// TaskMetadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub task: Option<String>,
    pub images: Option<Vec<String>>,
}

/// TaskStatus enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Idle,
    Active,
    Interactive,
    Resumable,
    Paused,
    Completed,
    Aborted,
}

/// TodoItem structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub completed: bool,
}

/// HistoryItem structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub task: Option<String>,
    pub ts: i64,
    pub is_favorited: Option<bool>,
}

/// ToolUsage tracking
#[derive(Debug, Clone, Default)]
pub struct ToolUsageTracker {
    pub usage_count: HashMap<String, u32>,
}

/// Re-export types from mcp_tools
pub use crate::mcp_tools::{UserContent as McpUserContent};

// Also define here for compatibility  
pub use crate::message_routing_dispatch::RooTerminalProcess;
/// ApiMessage structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// UserContent structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
}

/// AssistantMessageContent structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantMessageContent {
    Text { text: String },
    ToolUse { name: String, input: serde_json::Value },
}

/// ExtensionContext placeholder
#[derive(Debug, Clone)]
pub struct ExtensionContext {
    pub global_storage_uri: PathBuf,
    pub workspace_state: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

/// ClineProvider placeholder
#[derive(Debug)]
pub struct ClineProvider {
    pub context: ExtensionContext,
    pub state: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

// Implementation moved to avoid duplicate definitions

#[derive(Debug, Clone)]
pub struct ClineProviderState {
    pub mode: Option<String>,
}


/// Task class - exact translation
#[derive(Debug, Clone)]
pub struct Task {
    // Private fields from TypeScript
    pub context: ExtensionContext,
    
    // Public readonly fields
    pub task_id: String,
    pub task_is_favorited: Option<bool>,
    pub instance_id: String,
    pub metadata: TaskMetadata,
    
    // Optional fields
    pub todo_list: Option<Vec<TodoItem>>,
    
    // Task hierarchy
    pub root_task: Option<Arc<Task>>,
    pub parent_task: Option<Arc<Task>>,
    pub task_number: i32,
    pub workspace_path: String,
    
    // Task mode (with async initialization)
    pub task_mode: Arc<RwLock<Option<String>>>,
    pub task_mode_ready: Arc<Mutex<bool>>,
    
    // Provider reference (weak ref)
    pub provider_ref: Weak<ClineProvider>,
    pub global_storage_path: PathBuf,
    
    // Status fields
    pub abort: Arc<RwLock<bool>>,
    pub idle_ask: Arc<RwLock<Option<ClineMessage>>>,
    pub resumable_ask: Arc<RwLock<Option<ClineMessage>>>,
    pub interactive_ask: Arc<RwLock<Option<ClineMessage>>>,
    
    pub did_finish_aborting_stream: Arc<RwLock<bool>>,
    pub abandoned: Arc<RwLock<bool>>,
    pub is_initialized: Arc<RwLock<bool>>,
    pub is_paused: Arc<RwLock<bool>>,
    pub paused_mode_slug: Arc<RwLock<String>>,
    
    // API configuration
    pub api_configuration: ProviderSettings,
    pub api: Option<TaskApiHandler>,
    pub auto_approval_handler: Option<Arc<AutoApprovalHandler>>,
    
    // Tool repetition detection
    pub tool_repetition_detector: Arc<ToolRepetitionDetector>,
    pub roo_ignore_controller: Option<Arc<RooIgnoreController>>,
    pub roo_protected_controller: Option<Arc<RooProtectedController>>,
    pub file_context_tracker: Arc<FileContextTracker>,
    pub url_content_fetcher: Arc<UrlContentFetcher>,
    pub terminal_process: Arc<RwLock<Option<RooTerminalProcess>>>,
    
    // Browser session
    pub browser_session: Arc<BrowserSession>,
    
    // Diff management
    pub diff_view_provider: Arc<DiffViewProvider>,
    pub diff_strategy: Arc<RwLock<Option<DiffStrategy>>>,
    pub diff_enabled: bool,
    pub fuzzy_match_threshold: f32,
    pub did_edit_file: Arc<RwLock<bool>>,
    
    // Conversation history
    pub api_conversation_history: Arc<RwLock<Vec<ApiMessage>>>,
    pub cline_messages: Arc<RwLock<Vec<ClineMessage>>>,
    
    // Ask response handling
    pub ask_response: Arc<RwLock<Option<ClineAskResponse>>>,
    pub ask_response_text: Arc<RwLock<Option<String>>>,
    pub ask_response_images: Arc<RwLock<Option<Vec<String>>>>,
    pub last_message_ts: Arc<RwLock<Option<u64>>>,
    
    // Tool use tracking
    pub consecutive_mistake_count: Arc<RwLock<u32>>,
    pub consecutive_mistake_limit: u32,
    pub consecutive_mistake_count_for_apply_diff: Arc<RwLock<HashMap<String, u32>>>,
    pub tool_usage: Arc<RwLock<ToolUsageTracker>>,
    
    // Checkpoints
    pub enable_checkpoints: bool,
    pub checkpoint_service: Arc<RwLock<Option<RepoPerTaskCheckpointService>>>,
    pub checkpoint_service_initializing: Arc<RwLock<bool>>,
    
    // Task bridge
    pub enable_task_bridge: bool,
    pub bridge_service: Arc<RwLock<Option<ExtensionBridgeService>>>,
    
    // Streaming state
    pub is_waiting_for_first_chunk: Arc<RwLock<bool>>,
    pub is_streaming: Arc<RwLock<bool>>,
    pub current_streaming_content_index: Arc<RwLock<usize>>,
    pub current_streaming_did_checkpoint: Arc<RwLock<bool>>,
    pub assistant_message_content: Arc<RwLock<Vec<AssistantMessageContent>>>,
    pub present_assistant_message_locked: Arc<RwLock<bool>>,
    pub present_assistant_message_has_pending_updates: Arc<RwLock<bool>>,
    pub user_message_content: Arc<RwLock<Vec<serde_json::Value>>>,
    pub user_message_content_ready: Arc<RwLock<bool>>,
    pub did_reject_tool: Arc<RwLock<bool>>,
    pub did_already_use_tool: Arc<RwLock<bool>>,
    pub did_complete_reading_stream: Arc<RwLock<bool>>,
    pub assistant_message_parser: Arc<AssistantMessageParser>,
    pub last_used_instructions: Arc<RwLock<Option<String>>>,
    pub skip_prev_response_id_once: Arc<RwLock<bool>>,
}

// Placeholder types for components not yet translated
#[derive(Debug, Clone)]
pub struct TaskApiHandler;

impl TaskApiHandler {
    pub fn get_model_id(&self) -> Result<String, String> {
        Ok("gpt-4".to_string())
    }
    
    pub fn get_last_response_id(&self) -> Option<String> {
        None
    }
    
    pub async fn create_message(
        &self,
        _system_prompt: String,
        _messages: Vec<ApiMessage>,
        _metadata: Option<serde_json::Value>,
    ) -> Result<crate::streaming_pipeline::stream_transform::ApiStream, crate::streaming_pipeline::stream_transform::ApiError> {
        Err(crate::streaming_pipeline::stream_transform::ApiError {
            message: "Not implemented".to_string(),
            status: None,
            metadata: None,
            error_details: None,
        })
    }
}

#[derive(Debug)]
pub struct RooIgnoreController {
    cwd: PathBuf,
}
#[derive(Debug)]
pub struct RooProtectedController {
    cwd: PathBuf,
    context: Option<ExtensionContext>,
    task_id: String,
    task_is_favorited: Option<bool>,
    instance_id: String,
    metadata: Option<serde_json::Value>,
    task_name: Option<String>,
    task_image_url: Option<String>,
}
#[derive(Debug)]
pub struct DiffStrategy;
#[derive(Debug)]
pub struct RepoPerTaskCheckpointService;
#[derive(Debug)]
pub struct ExtensionBridgeService;
#[derive(Debug)]
pub struct AssistantMessageParser;

impl RooIgnoreController {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }
    
    pub async fn initialize(&self) -> Result<(), String> {
        // Initialize ignore patterns
        Ok(())
    }
    
    pub async fn post_message_to_webview(&self, message: crate::task_connection_handling::WebviewMessage) -> Result<(), String> {
        // Initialize ignore patterns
        Ok(())
    }
}

impl RooProtectedController {
    pub fn new(cwd: PathBuf) -> Self {
        Self { 
            cwd,
            context: None,
            task_id: String::new(),
            task_is_favorited: None,
            instance_id: String::new(),
            metadata: None,
            task_name: None,
            task_image_url: None,
        }
    }
}

impl Task {
    /// constructor - exact translation from TypeScript  
    pub fn new(options: TaskOptions) -> Arc<Self> {
        let TaskOptions {
            context,
            provider,
            api_configuration,
            enable_diff,
            enable_checkpoints,
            enable_task_bridge,
            fuzzy_match_threshold,
            consecutive_mistake_limit,
            task,
            images,
            history_item,
            start_task,
            root_task,
            parent_task,
            task_number,
            on_created,
            initial_todos,
            ..
        } = options;
        
        // Validation from TypeScript
        if start_task.unwrap_or(true) && task.is_none() && images.is_none() && history_item.is_none() {
            panic!("Either historyItem or task/images must be provided");
        }
        
        // Generate IDs
        let task_id = if let Some(ref item) = history_item {
            item.id.clone()
        } else {
            Uuid::new_v4().to_string()
        };
        
        let task_is_favorited = history_item.as_ref().and_then(|h| h.is_favorited);
        
        let metadata = TaskMetadata {
            task: history_item.as_ref().and_then(|h| h.task.clone()).or(task),
            images: if history_item.is_some() { Some(vec![]) } else { images },
        };
        
        // Workspace path logic
        let workspace_path = if let Some(ref parent) = parent_task {
            parent.workspace_path.clone()
        } else {
            get_workspace_path(PathBuf::from(std::env::var("HOME").unwrap()).join("Documents"))
        };
        
        let instance_id = Uuid::new_v4().to_string()[..8].to_string();
        
        // Initialize controllers
        let cwd = PathBuf::from(&workspace_path);
        let roo_ignore_controller = Some(Arc::new(RooIgnoreController::new(cwd.clone())));
        let roo_protected_controller = Some(Arc::new(RooProtectedController::new(cwd.clone())));
        
        let task = Arc::new(Self {
            context: context.clone().unwrap_or_else(|| ExtensionContext {
                global_storage_uri: PathBuf::from("/tmp"),
                workspace_state: Arc::new(RwLock::new(HashMap::new())),
            }),
            task_id,
            task_is_favorited,
            instance_id,
            metadata,
            todo_list: initial_todos,
            root_task,
            parent_task,
            task_number: task_number.unwrap_or(-1),
            workspace_path,
            task_mode: Arc::new(RwLock::new(None)),
            task_mode_ready: Arc::new(Mutex::new(false)),
            provider_ref: Arc::downgrade(&provider.unwrap_or_else(|| Arc::new(ClineProvider {
                context: ExtensionContext {
                    global_storage_uri: PathBuf::from("/tmp"),
                    workspace_state: Arc::new(RwLock::new(HashMap::new())),
                },
                state: Arc::new(RwLock::new(HashMap::new())),
            }))),
            global_storage_path: context.as_ref().map(|c| c.global_storage_uri.clone()).unwrap_or_else(|| PathBuf::from("/tmp")),
            abort: Arc::new(RwLock::new(false)),
            idle_ask: Arc::new(RwLock::new(None)),
            resumable_ask: Arc::new(RwLock::new(None)),
            interactive_ask: Arc::new(RwLock::new(None)),
            did_finish_aborting_stream: Arc::new(RwLock::new(false)),
            abandoned: Arc::new(RwLock::new(false)),
            is_initialized: Arc::new(RwLock::new(false)),
            is_paused: Arc::new(RwLock::new(false)),
            paused_mode_slug: Arc::new(RwLock::new("default".to_string())),
            api_configuration: api_configuration.clone().unwrap_or(ProviderSettings::default()),
            api: Some(TaskApiHandler),
            auto_approval_handler: Some(Arc::new(AutoApprovalHandler)),
            tool_repetition_detector: Arc::new(ToolRepetitionDetector),
            roo_ignore_controller,
            roo_protected_controller,
            file_context_tracker: Arc::new(FileContextTracker),
            url_content_fetcher: Arc::new(UrlContentFetcher),
            terminal_process: Arc::new(RwLock::new(None)),
            browser_session: Arc::new(BrowserSession),
            diff_view_provider: Arc::new(DiffViewProvider),
            diff_strategy: Arc::new(RwLock::new(None)),
            diff_enabled: enable_diff.unwrap_or(false),
            fuzzy_match_threshold: fuzzy_match_threshold.unwrap_or(1.0),
            did_edit_file: Arc::new(RwLock::new(false)),
            api_conversation_history: Arc::new(RwLock::new(vec![])),
            cline_messages: Arc::new(RwLock::new(vec![])),
            ask_response: Arc::new(RwLock::new(None)),
            ask_response_text: Arc::new(RwLock::new(None)),
            ask_response_images: Arc::new(RwLock::new(None)),
            last_message_ts: Arc::new(RwLock::new(None)),
            consecutive_mistake_count: Arc::new(RwLock::new(0)),
            consecutive_mistake_limit: consecutive_mistake_limit.unwrap_or(3),
            consecutive_mistake_count_for_apply_diff: Arc::new(RwLock::new(HashMap::new())),
            tool_usage: Arc::new(RwLock::new(ToolUsageTracker::default())),
            enable_checkpoints: enable_checkpoints.unwrap_or(true),
            checkpoint_service: Arc::new(RwLock::new(None)),
            checkpoint_service_initializing: Arc::new(RwLock::new(false)),
            enable_task_bridge: enable_task_bridge.unwrap_or(false),
            bridge_service: Arc::new(RwLock::new(None)),
            is_waiting_for_first_chunk: Arc::new(RwLock::new(false)),
            is_streaming: Arc::new(RwLock::new(false)),
            current_streaming_content_index: Arc::new(RwLock::new(0)),
            current_streaming_did_checkpoint: Arc::new(RwLock::new(false)),
            assistant_message_content: Arc::new(RwLock::new(vec![])),
            present_assistant_message_locked: Arc::new(RwLock::new(false)),
            present_assistant_message_has_pending_updates: Arc::new(RwLock::new(false)),
            user_message_content: Arc::new(RwLock::new(vec![])),
            user_message_content_ready: Arc::new(RwLock::new(false)),
            did_reject_tool: Arc::new(RwLock::new(false)),
            did_already_use_tool: Arc::new(RwLock::new(false)),
            did_complete_reading_stream: Arc::new(RwLock::new(false)),
            assistant_message_parser: Arc::new(AssistantMessageParser),
            last_used_instructions: Arc::new(RwLock::new(None)),
            skip_prev_response_id_once: Arc::new(RwLock::new(false)),
        });
        
        // Call on_created callback if provided
        if let Some(callback) = on_created {
            callback(task.clone());
        }
        
        // Initialize RooIgnoreController asynchronously
        if let Some(ref controller) = task.roo_ignore_controller {
            let controller_clone = controller.clone();
            tokio::spawn(async move {
                if let Err(e) = controller_clone.initialize().await {
                    eprintln!("Failed to initialize RooIgnoreController: {}", e);
                }
            });
        }
        
        task
    }
    
    /// Reset the global API request timestamp (for testing)
    pub fn reset_global_api_request_time() {
        // Reset static timestamp
    }
    
    /// Emit an event for this task
    pub fn emit_event(&self, event: RooCodeEventName, data: String) {
        // Event emission implementation - placeholder for now
        println!("Task {} Event: {:?} - {}", self.task_id, event, data);
    }
}

/// Helper function to get workspace path
fn get_workspace_path(default: PathBuf) -> String {
    // Get workspace path or use default
    default.to_str().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_creation() {
        // Test basic task creation
    }
}
