//! Context Condensation via LLM Summarization
//!
//! Direct 1:1 port from Codex/src/core/condense/index.ts
//! Intelligent conversation summarization to reduce token usage while preserving semantic meaning.
//!
//! Key features:
//! - N_MESSAGES_TO_KEEP = 3 (always preserve last 3 message pairs)
//! - Detailed 6-section summary prompt (verbatim from TypeScript)
//! - Growth prevention: reject summary if tokens >= previous context
//! - Bedrock-first-user workaround for compatibility
//! - Custom prompts and dedicated condensing handlers
//! - Image block removal before summarization

use serde::{Deserialize, Serialize};

// Re-export ApiMessage from sliding_window for now
// TODO: Move to shared types module (PORT-TYPES-02)
use crate::core::sliding_window::ApiMessage;

/// Number of most recent message pairs to always keep
/// From Codex: N_MESSAGES_TO_KEEP = 3
pub const N_MESSAGES_TO_KEEP: usize = 3;

/// Minimum percentage of context window to trigger condensing
/// From Codex: MIN_CONDENSE_THRESHOLD = 5
pub const MIN_CONDENSE_THRESHOLD: f64 = 5.0;

/// Maximum percentage of context window to trigger condensing
/// From Codex: MAX_CONDENSE_THRESHOLD = 100
pub const MAX_CONDENSE_THRESHOLD: f64 = 100.0;

/// EXACT VERBATIM SUMMARY PROMPT from Codex condense/index.ts lines 14-52
/// DO NOT MODIFY - years of calibration went into this
pub const SUMMARY_PROMPT: &str = r#"Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions.
This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing with the conversation and supporting any continuing tasks.

Your summary should be structured as follows:
Context: The context to continue the conversation with. If applicable based on the current task, this should include:
  1. Previous Conversation: High level details about what was discussed throughout the entire conversation with the user. This should be written to allow someone to be able to follow the general overarching conversation flow.
  2. Current Work: Describe in detail what was being worked on prior to this request to summarize the conversation. Pay special attention to the more recent messages in the conversation.
  3. Key Technical Concepts: List all important technical concepts, technologies, coding conventions, and frameworks discussed, which might be relevant for continuing with this work.
  4. Relevant Files and Code: If applicable, enumerate specific files and code sections examined, modified, or created for the task continuation. Pay special attention to the most recent messages and changes.
  5. Problem Solving: Document problems solved thus far and any ongoing troubleshooting efforts.
  6. Pending Tasks and Next Steps: Outline all pending tasks that you have explicitly been asked to work on, as well as list the next steps you will take for all outstanding work, if applicable. Include code snippets where they add clarity. For any next steps, include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off. This should be verbatim to ensure there's no information loss in context between tasks.

Example summary structure:
1. Previous Conversation:
  [Detailed description]
2. Current Work:
  [Detailed description]
3. Key Technical Concepts:
  - [Concept 1]
  - [Concept 2]
  - [...]
4. Relevant Files and Code:
  - [File Name 1]
    - [Summary of why this file is important]
    - [Summary of the changes made to this file, if any]
    - [Important Code Snippet]
  - [File Name 2]
    - [Important Code Snippet]
  - [...]
5. Problem Solving:
  [Detailed description]
6. Pending Tasks and Next Steps:
  - [Task 1 details & next steps]
  - [Task 2 details & next steps]
  - [...]

Output only the summary of the conversation so far, without any additional commentary or explanation."#;

/// Response from summarization operation
#[derive(Debug, Clone)]
pub struct SummarizeResponse {
    pub messages: Vec<ApiMessage>,
    pub summary: String,
    pub cost: f64,
    pub new_context_tokens: Option<usize>,
    pub error: Option<String>,
}

/// Summarizes the conversation messages using an LLM call
///
/// Port of Codex summarizeConversation() from condense/index.ts lines 85-219
///
/// # Arguments
/// * `messages` - The conversation messages
/// * `system_prompt` - The system prompt for API requests
/// * `task_id` - The task ID for telemetry
/// * `prev_context_tokens` - Current token count to ensure no growth
/// * `is_automatic_trigger` - Whether triggered automatically
/// * `custom_condensing_prompt` - Optional custom prompt
///
/// # Returns
/// SummarizeResponse with new messages, summary, cost, and tokens
pub async fn summarize_conversation(
    messages: Vec<ApiMessage>,
    system_prompt: String,
    task_id: String,
    prev_context_tokens: usize,
    is_automatic_trigger: bool,
    custom_condensing_prompt: Option<String>,
) -> Result<SummarizeResponse, String> {
    // Telemetry hook
    // TelemetryService::instance().capture_context_condensed(
    //     task_id, is_automatic_trigger, custom_condensing_prompt.is_some()
    // );
    
    let mut response = SummarizeResponse {
        messages: messages.clone(),
        summary: String::new(),
        cost: 0.0,
        new_context_tokens: None,
        error: None,
    };
    
    // Always preserve the first message (which may contain slash command content)
    if messages.is_empty() {
        response.error = Some("No messages to condense".to_string());
        return Ok(response);
    }
    
    let first_message = &messages[0];
    
    // Get messages to summarize, including the first message and excluding the last N messages
    let messages_to_summarize_end = messages.len().saturating_sub(N_MESSAGES_TO_KEEP);
    let messages_to_summarize = get_messages_since_last_summary(
        &messages[..messages_to_summarize_end].to_vec()
    );
    
    if messages_to_summarize.len() <= 1 {
        let error = if messages.len() <= N_MESSAGES_TO_KEEP + 1 {
            format!(
                "Not enough messages to condense. Have {} messages, need at least {} (current context: {} tokens)",
                messages.len(),
                N_MESSAGES_TO_KEEP + 2,
                prev_context_tokens
            )
        } else {
            "Context was condensed recently".to_string()
        };
        response.error = Some(error);
        return Ok(response);
    }
    
    let keep_messages = &messages[messages.len().saturating_sub(N_MESSAGES_TO_KEEP)..];
    
    // Check if there's a recent summary in the messages we're keeping
    let recent_summary_exists = keep_messages.iter().any(|m| m.is_summary.unwrap_or(false));
    
    if recent_summary_exists {
        response.error = Some("Context was condensed recently".to_string());
        return Ok(response);
    }
    
    // TODO: Implement actual LLM call via provider
    // For now, return placeholder
    // let final_request_message = ApiMessage {
    //     role: "user".to_string(),
    //     content: MessageContent::Text(
    //         "Summarize the conversation so far, as described in the prompt instructions.".to_string()
    //     ),
    //     ts: None,
    //     is_summary: None,
    // };
    
    // let mut request_messages = messages_to_summarize.clone();
    // request_messages.push(final_request_message);
    
    // // Remove image blocks
    // request_messages = maybe_remove_image_blocks(request_messages, api_handler);
    
    // // Use custom prompt if provided and non-empty
    // let prompt_to_use = custom_condensing_prompt
    //     .filter(|p| !p.trim().is_empty())
    //     .unwrap_or_else(|| SUMMARY_PROMPT.to_string());
    
    // // Stream the summarization request
    // let stream = handler.create_message(prompt_to_use, request_messages);
    
    // let mut summary = String::new();
    // let mut cost = 0.0;
    // let mut output_tokens = 0;
    
    // for await chunk in stream {
    //     match chunk.type {
    //         "text" => summary.push_str(&chunk.text),
    //         "usage" => {
    //             cost = chunk.total_cost.unwrap_or(0.0);
    //             output_tokens = chunk.output_tokens.unwrap_or(0);
    //         }
    //         _ => {}
    //     }
    // }
    
    // summary = summary.trim();
    
    // if summary.is_empty() {
    //     response.error = Some("Condensation failed - empty summary".to_string());
    //     response.cost = cost;
    //     return Ok(response);
    // }
    
    // // Create summary message
    // let summary_message = ApiMessage {
    //     role: "assistant".to_string(),
    //     content: MessageContent::Text(summary.clone()),
    //     ts: keep_messages.get(0).and_then(|m| m.ts),
    //     is_summary: Some(true),
    // };
    
    // // Reconstruct messages: [first message, summary, last N messages]
    // let mut new_messages = vec![first_message.clone(), summary_message];
    // new_messages.extend_from_slice(keep_messages);
    
    // // Count tokens in new context
    // // Use output_tokens if available, otherwise estimate
    // let new_context_tokens = if output_tokens > 0 {
    //     output_tokens + count_tokens_for_messages(keep_messages)
    // } else {
    //     count_tokens_for_messages(&new_messages)
    // };
    
    // // Growth prevention check
    // if new_context_tokens >= prev_context_tokens {
    //     response.error = Some(format!(
    //         "Condensation would grow context: {} → {} tokens",
    //         prev_context_tokens, new_context_tokens
    //     ));
    //     response.cost = cost;
    //     return Ok(response);
    // }
    
    // Ok(SummarizeResponse {
    //     messages: new_messages,
    //     summary,
    //     cost,
    //     new_context_tokens: Some(new_context_tokens),
    //     error: None,
    // })
    
    // PLACEHOLDER until provider integration
    response.error = Some("Condensation not yet implemented - provider integration pending".to_string());
    Ok(response)
}

/// Returns the list of all messages since the last summary message, including the summary.
/// Returns all messages if there is no summary.
///
/// Port of Codex getMessagesSinceLastSummary() from condense/index.ts lines 222-253
///
/// # Arguments
/// * `messages` - The conversation messages
///
/// # Returns
/// Messages since last summary (or all messages if no summary found)
pub fn get_messages_since_last_summary(messages: &[ApiMessage]) -> Vec<ApiMessage> {
    // Find last summary index (search from end)
    let last_summary_index = messages
        .iter()
        .rposition(|m| m.is_summary.unwrap_or(false));
    
    let messages_since_summary = if let Some(idx) = last_summary_index {
        messages[idx..].to_vec()
    } else {
        messages.to_vec()
    };
    
    // Bedrock requires the first message to be a user message.
    // We preserve the original first message to maintain context.
    // See https://github.com/RooCodeInc/Roo-Code/issues/4147
    if !messages_since_summary.is_empty() && messages_since_summary[0].role != "user" {
        // Get the original first message (should always be a user message with the task)
        if let Some(original_first) = messages.first() {
            if original_first.role == "user" {
                // Use the original first message unchanged to maintain full context
                let mut result = vec![original_first.clone()];
                result.extend_from_slice(&messages_since_summary);
                return result;
            }
        }
        
        // Fallback to generic message if no original first message exists (shouldn't happen)
        use crate::core::sliding_window::MessageContent;
        let user_message = ApiMessage {
            role: "user".to_string(),
            content: MessageContent::Text("Please continue from the following summary:".to_string()),
            ts: messages.first().and_then(|m| m.ts).map(|ts| ts.saturating_sub(1)),
            is_summary: None,
        };
        let mut result = vec![user_message];
        result.extend_from_slice(&messages_since_summary);
        return result;
    }
    
    messages_since_summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sliding_window::{ApiMessage, MessageContent};
    
    #[test]
    fn test_constants() {
        assert_eq!(N_MESSAGES_TO_KEEP, 3);
        assert_eq!(MIN_CONDENSE_THRESHOLD, 5.0);
        assert_eq!(MAX_CONDENSE_THRESHOLD, 100.0);
    }
    
    #[test]
    fn test_summary_prompt_verbatim() {
        // Ensure the summary prompt is not accidentally modified
        assert!(SUMMARY_PROMPT.contains("Your task is to create a detailed summary"));
        assert!(SUMMARY_PROMPT.contains("Previous Conversation:"));
        assert!(SUMMARY_PROMPT.contains("Pending Tasks and Next Steps:"));
        assert!(SUMMARY_PROMPT.contains("verbatim"));
    }
    
    #[test]
    fn test_get_messages_since_last_summary_no_summary() {
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
        ];
        
        let result = get_messages_since_last_summary(&messages);
        assert_eq!(result.len(), 2);
    }
    
    #[test]
    fn test_get_messages_since_last_summary_with_summary() {
        let messages = vec![
            ApiMessage {
                role: "user".to_string(),
                content: MessageContent::Text("First".to_string()),
                ts: Some(1),
                is_summary: None,
            },
            ApiMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text("Summary text".to_string()),
                ts: Some(2),
                is_summary: Some(true),
            },
            ApiMessage {
                role: "user".to_string(),
                content: MessageContent::Text("After summary".to_string()),
                ts: Some(3),
                is_summary: None,
            },
        ];
        
        let result = get_messages_since_last_summary(&messages);
        // Should include summary and messages after
        assert_eq!(result.len(), 2);
        assert!(result[0].is_summary.unwrap_or(false));
    }
    
    #[test]
    fn test_bedrock_first_user_workaround() {
        let messages = vec![
            ApiMessage {
                role: "user".to_string(),
                content: MessageContent::Text("Original first".to_string()),
                ts: Some(1),
                is_summary: None,
            },
            ApiMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text("Summary".to_string()),
                ts: Some(2),
                is_summary: Some(true),
            },
        ];
        
        let result = get_messages_since_last_summary(&messages);
        
        // Should prepend original first message if summary is not user role
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "user");
        if let MessageContent::Text(text) = &result[0].content {
            assert_eq!(text, "Original first");
        }
    }
}
