// Missing types for streaming_response.rs

use serde::{Deserialize, Serialize};

// OpenAI handler types
pub struct OpenAiHandler;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageParam {
    pub role: String,
    pub content: ChatMessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentPart {
    Text { text: String },
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
pub fn convert_to_r1_format(_msg: &MessageParam) -> MessageParam {
    // TODO: Implement R1 format conversion
    MessageParam {
        role: "user".to_string(),
        content: ChatMessageContent::Text("".to_string()),
    }
}

pub fn convert_to_simple_messages(_msgs: &[MessageParam]) -> Vec<MessageParam> {
    // TODO: Implement simple messages conversion
    vec![]
}

pub fn convert_to_openai_messages(_msgs: &[MessageParam]) -> Vec<ChatCompletionMessageParam> {
    // TODO: Implement OpenAI messages conversion
    vec![]
}
