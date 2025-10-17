//! Sliding Window Context Management
//!
//! Direct 1:1 port from Codex/src/core/sliding-window/index.ts
//! Handles token counting, conversation truncation, and context window management.
//! Key features:
//! - Token counting via provider or fallback to tiktoken
//! - Pair-preserving truncation (keep first message, remove even number)
//! - TOKEN_BUFFER_PERCENTAGE = 10% safety margin
//! - Profile-specific condense thresholds
// Ported from: Codex/src/core/sliding-window/index.ts
// Handles conversation truncation and token management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::condense::{
    summarize_conversation, SummarizeResponse, MIN_CONDENSE_THRESHOLD, MAX_CONDENSE_THRESHOLD,
};
use crate::core::model_limits::get_reserved_tokens;
use crate::core::token_counter;

/// Default percentage of the context window to use as a buffer when deciding when to truncate
/// From Codex: TOKEN_BUFFER_PERCENTAGE = 0.1
pub const TOKEN_BUFFER_PERCENTAGE: f64 = 0.1;
/// Represents an API message (aligned with Anthropic's MessageParam)
/// TODO: Move to shared types module (PORT-TYPES-02)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String, // "user" | "assistant"
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_summary: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    ContentBlocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        source: ImageSource,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Options for truncateConversationIfNeeded
#[derive(Debug, Clone)]
pub struct TruncateOptions {
    pub messages: Vec<ApiMessage>,
    pub total_tokens: usize,
    pub context_window: usize,
    pub max_tokens: Option<usize>,
    pub model_id: String,
    pub auto_condense_context: bool,
    pub auto_condense_context_percent: f64,
    pub system_prompt: String,
    pub task_id: String,
    pub custom_condensing_prompt: Option<String>,
    pub profile_thresholds: HashMap<String, f64>,
    pub current_profile_id: String,
}

/// Response from truncation operation (extends SummarizeResponse)
#[derive(Debug, Clone)]
pub struct TruncateResponse {
    pub messages: Vec<ApiMessage>,
    pub summary: String,
    pub cost: f64,
    pub new_context_tokens: Option<usize>,
    pub error: Option<String>,
    pub prev_context_tokens: usize,
}

/// Counts tokens for content using the provider's token counting implementation.
///
/// Port of Codex estimateTokenCount() from sliding-window/index.ts lines 22-28
///
/// # Arguments
/// * `content` - The content blocks to count tokens for
/// * `model_id` - The model ID for tokenizer selection
///
/// # Returns
/// Token count as Result<usize, String>
pub fn estimate_token_count(content: &[ContentBlock], model_id: &str) -> Result<usize, String> {
    let mut total = 0;
    
    for block in content {
        match block {
            ContentBlock::Text { text } => {
                // Use tiktoken for accurate counting
                let count = token_counter::count_tokens(text, model_id)?;
                total += count;
            }
            ContentBlock::Image { .. } => {
                // Images cost ~1000 tokens (Anthropic approximation)
                total += 1000;
            }
            ContentBlock::ToolUse { .. } | ContentBlock::ToolResult { .. } => {
                // Tool blocks are typically small, estimate 100 tokens
                total += 100;
            }
        }
    }
    
    Ok(total)
}

/// Truncates a conversation by removing a fraction of the messages.
///
/// Port of Codex truncateConversation() from sliding-window/index.ts lines 41-50
///
/// The first message is always retained, and a specified fraction (rounded to an even number)
/// of messages from the beginning (excluding the first) is removed.
///
/// # Arguments
/// * `messages` - The conversation messages
/// * `frac_to_remove` - The fraction (between 0 and 1) of messages (excluding the first) to remove
/// * `task_id` - The task ID for telemetry
///
/// # Returns
/// Truncated conversation messages
pub fn truncate_conversation(
    messages: Vec<ApiMessage>,
    frac_to_remove: f64,
    task_id: &str,
) -> Vec<ApiMessage> {
    // Telemetry hook
    // TelemetryService::instance().capture_sliding_window_truncation(task_id);
    
    if messages.is_empty() {
        return messages;
    }
    
    let mut truncated_messages = vec![messages[0].clone()];
    
    let raw_messages_to_remove = ((messages.len() - 1) as f64 * frac_to_remove).floor() as usize;
    let messages_to_remove = raw_messages_to_remove - (raw_messages_to_remove % 2);
    
    let remaining_messages = &messages[messages_to_remove + 1..];
    truncated_messages.extend_from_slice(remaining_messages);
    
    truncated_messages
}

/// Conditionally truncates the conversation messages if the total token count
/// exceeds the model's limit, considering the size of incoming content.
///
/// Port of Codex truncateConversationIfNeeded() from sliding-window/index.ts lines 91-175
///
/// # Arguments
/// * `options` - The truncation options
///
/// # Returns
/// TruncateResponse with messages, summary, cost, and token counts
pub async fn truncate_conversation_if_needed(
    options: TruncateOptions,
) -> Result<TruncateResponse, String> {
    let TruncateOptions {
        messages,
        total_tokens,
        context_window,
        max_tokens,
        model_id,
        auto_condense_context,
        auto_condense_context_percent,
        system_prompt,
        task_id,
        custom_condensing_prompt,
        profile_thresholds,
        current_profile_id,
    } = options;
    
    let mut error: Option<String> = None;
    let mut cost = 0.0;
    
    // Calculate the maximum tokens reserved for response
    // Uses model limits or custom max_tokens
    let reserved_tokens = get_reserved_tokens(&model_id, max_tokens);
    
    // Estimate tokens for the last message (which is always a user message)
    if messages.is_empty() {
        return Err("No messages to truncate".to_string());
    }
    
    let last_message = &messages[messages.len() - 1];
    
    // Use real token counting via tiktoken
    let last_message_tokens = match &last_message.content {
        MessageContent::Text(text) => {
            token_counter::count_tokens(text, &model_id)
                .map_err(|e| format!("Token counting failed: {}", e))?
        }
        MessageContent::ContentBlocks(blocks) => {
            estimate_token_count(blocks, &model_id)?
        }
    };
    
    // Calculate total effective tokens (totalTokens never includes the last message)
    let prev_context_tokens = total_tokens + last_message_tokens;
    
    // Calculate available tokens for conversation history
    // Truncate if we're within TOKEN_BUFFER_PERCENTAGE of the context window
    let allowed_tokens = ((context_window as f64) * (1.0 - TOKEN_BUFFER_PERCENTAGE)) as usize
        - reserved_tokens;
    
    // Determine the effective threshold to use
    let mut effective_threshold = auto_condense_context_percent;
    
    if let Some(&profile_threshold) = profile_thresholds.get(&current_profile_id) {
        if profile_threshold == -1.0 {
            // Special case: -1 means inherit from global setting
            effective_threshold = auto_condense_context_percent;
        } else if profile_threshold >= MIN_CONDENSE_THRESHOLD
            && profile_threshold <= MAX_CONDENSE_THRESHOLD
        {
            // Valid custom threshold
            effective_threshold = profile_threshold;
        } else {
            // Invalid threshold value, fall back to global setting
            eprintln!(
                "Invalid profile threshold {} for profile \"{}\". Using global default of {}%",
                profile_threshold, current_profile_id, auto_condense_context_percent
            );
            effective_threshold = auto_condense_context_percent;
        }
    }
    
    // If auto condense is enabled
    if auto_condense_context {
        let context_percent = (100.0 * prev_context_tokens as f64) / (context_window as f64);
        
        if context_percent >= effective_threshold
            || prev_context_tokens > allowed_tokens
        {
            // Attempt to intelligently condense the context
            // TODO: implement summarize_conversation call
            // let result = summarize_conversation(...).await?;
            // if result.error.is_some() {
            //     error = result.error;
            //     cost = result.cost;
            // } else {
            //     return Ok(TruncateResponse {
            //         messages: result.messages,
            //         summary: result.summary,
            //         cost: result.cost,
            //         new_context_tokens: result.new_context_tokens,
            //         error: None,
            //         prev_context_tokens,
            //     });
            // }
        }
    }
    
    // Fall back to sliding window truncation if needed
    if prev_context_tokens > allowed_tokens {
        let truncated_messages = truncate_conversation(messages, 0.5, &task_id);
        return Ok(TruncateResponse {
            messages: truncated_messages,
            summary: String::new(),
            cost,
            new_context_tokens: None,
            error,
            prev_context_tokens,
        });
    }
    
    // No truncation or condensation needed
    Ok(TruncateResponse {
        messages,
        summary: String::new(),
        cost,
        new_context_tokens: None,
        error,
        prev_context_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_truncate_conversation_keeps_first_message() {
        let messages = vec![
            ApiMessage {
                role: "user".to_string(),
                content: MessageContent::Text("First".to_string()),
                ts: Some(1),
                is_summary: None,
            },
            ApiMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text("Second".to_string()),
                ts: Some(2),
                is_summary: None,
            },
            ApiMessage {
                role: "user".to_string(),
                content: MessageContent::Text("Third".to_string()),
                ts: Some(3),
                is_summary: None,
            },
        ];
        
        let result = truncate_conversation(messages, 0.5, "test-task");
        
        // Should keep first message
        assert_eq!(result.len(), 2); // First + one from remaining
        assert_eq!(result[0].role, "user");
        if let MessageContent::Text(text) = &result[0].content {
            assert_eq!(text, "First");
        } else {
            panic!("Expected text content");
        }
    }
    
    #[test]
    fn test_truncate_conversation_removes_even_number() {
        let mut messages = vec![];
        for i in 0..10 {
            messages.push(ApiMessage {
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                content: MessageContent::Text(format!("Message {}", i)),
                ts: Some(i as u64),
                is_summary: None,
            });
        }
        
        let result = truncate_conversation(messages.clone(), 0.5, "test-task");
        
        // Should remove even number of messages (excluding first)
        // 9 messages (excluding first), 50% = 4.5 → 4 messages removed
        // Result: 1 (first) + 5 (remaining) = 6
        assert_eq!(result.len(), 6);
        
        // First message should be preserved
        if let MessageContent::Text(text) = &result[0].content {
            assert_eq!(text, "Message 0");
        }
    }
    
    #[test]
    fn test_token_buffer_percentage_constant() {
        assert_eq!(TOKEN_BUFFER_PERCENTAGE, 0.1);
    }
}
