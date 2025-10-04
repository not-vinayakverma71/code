/// Exact 1:1 Translation of TypeScript simple-format from codex-reference/api/transform/simple-format.ts
/// DAY 12 AFTERNOON: Translate simple-format.ts

use serde::{Deserialize, Serialize};

/// MessageParam content types
#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// ContentBlock types
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { name: String },
    ToolResult { content: ToolResultContent },
}

#[derive(Debug, Clone)]
pub struct ImageSource {
    pub source_type: String, // "url" or "base64"
    pub media_type: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ToolResultContent {
    Text(String),
    Parts(Vec<ToolResultPart>),
}

#[derive(Debug, Clone)]
pub enum ToolResultPart {
    Text { text: String },
    Image { source: ImageSource },
}

/// convertToSimpleContent - exact translation lines 6-50
/// Convert complex content blocks to simple string content
pub fn convert_to_simple_content(content: MessageContent) -> String {
    match content {
        MessageContent::Text(s) => s,
        MessageContent::Blocks(blocks) => {
            blocks
                .into_iter()
                .filter_map(|block| {
                    match block {
                        ContentBlock::Text { text } => Some(text),
                        ContentBlock::Image { source } => {
                            // Line 18: kilocode_change
                            if source.source_type == "url" {
                                Some("[Image: URL]".to_string())
                            } else {
                                Some(format!("[Image: {}]", 
                                    source.media_type.unwrap_or_else(|| "unknown".to_string())))
                            }
                        }
                        ContentBlock::ToolUse { name } => {
                            Some(format!("[Tool Use: {}]", name))
                        }
                        ContentBlock::ToolResult { content } => {
                            match content {
                                ToolResultContent::Text(s) => Some(s),
                                ToolResultContent::Parts(parts) => {
                                    let result = parts
                                        .into_iter()
                                        .filter_map(|part| {
                                            match part {
                                                ToolResultPart::Text { text } => Some(text),
                                                ToolResultPart::Image { source } => {
                                                    // Lines 35-37: kilocode_change
                                                    if source.source_type == "url" {
                                                        Some("[Image: URL]".to_string())
                                                    } else {
                                                        Some(format!("[Image: {}]",
                                                            source.media_type.unwrap_or_else(|| "unknown".to_string())))
                                                    }
                                                }
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    if result.is_empty() {
                                        None
                                    } else {
                                        Some(result)
                                    }
                                }
                            }
                        }
                    }
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

/// Message structure
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

/// Simple message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMessage {
    pub role: String,
    pub content: String,
}

/// convertToSimpleMessages - exact translation lines 55-62
/// Convert Anthropic messages to simple format with string content
pub fn convert_to_simple_messages(messages: Vec<Message>) -> Vec<SimpleMessage> {
    messages
        .into_iter()
        .map(|message| SimpleMessage {
            role: message.role,
            content: convert_to_simple_content(message.content),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_text_content() {
        let content = MessageContent::Text("Hello world".to_string());
        let result = convert_to_simple_content(content);
        assert_eq!(result, "Hello world");
    }
    
    #[test]
    fn test_complex_blocks() {
        let content = MessageContent::Blocks(vec![
            ContentBlock::Text { text: "Text part".to_string() },
            ContentBlock::Image { 
                source: ImageSource {
                    source_type: "url".to_string(),
                    media_type: None,
                }
            },
            ContentBlock::ToolUse { name: "calculator".to_string() },
        ]);
        
        let result = convert_to_simple_content(content);
        assert!(result.contains("Text part"));
        assert!(result.contains("[Image: URL]"));
        assert!(result.contains("[Tool Use: calculator]"));
    }
    
    #[test]
    fn test_convert_messages() {
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: MessageContent::Text("Question".to_string()),
            },
            Message {
                role: "assistant".to_string(),
                content: MessageContent::Text("Answer".to_string()),
            },
        ];
        
        let result = convert_to_simple_messages(messages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "user");
        assert_eq!(result[0].content, "Question");
        assert_eq!(result[1].role, "assistant");
        assert_eq!(result[1].content, "Answer");
    }
}
