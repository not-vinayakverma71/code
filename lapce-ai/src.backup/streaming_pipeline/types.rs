// Missing types for streaming_response.rs

use serde::{Deserialize, Serialize};

// ModelInfo struct
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub supports_prompt_cache: bool,
    pub max_tokens: u32,
    pub context_window: u32,
}

// OpenAiHandler moved to streaming_response.rs to avoid duplication

pub use super::streaming_response::{OpenAiHandler, OpenAiHandlerOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageParam {
    pub role: String,
    pub content: ChatMessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
    MultiModal(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentPart {
    Text { 
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    Image { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiHandlerCreateMessageMetadata {
    pub user_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessageParam>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessageParam {
    pub role: String,
    pub content: String,
}

pub type ChatCompletionStream = futures::stream::BoxStream<'static, Result<String, anyhow::Error>>;

// Format conversion functions
pub fn convert_to_r1_format(messages: Vec<MessageParam>) -> Vec<MessageParam> {
    // Convert messages to R1 format
    messages.into_iter().map(|msg| {
        MessageParam {
            role: msg.role,
            content: msg.content,
        }
    }).collect()
}

pub fn convert_to_simple_messages(msgs: Vec<MessageParam>) -> Vec<MessageParam> {
    msgs
}

pub fn convert_to_openai_messages(msgs: Vec<MessageParam>) -> Vec<ChatCompletionMessageParam> {
    msgs.into_iter().map(|msg| {
        ChatCompletionMessageParam {
            role: msg.role,
            content: match msg.content {
                ChatMessageContent::Text(text) => text,
                ChatMessageContent::Parts(_) => "".to_string(),
                ChatMessageContent::MultiModal(_) => "".to_string(),
            }
        }
    }).collect()
}
