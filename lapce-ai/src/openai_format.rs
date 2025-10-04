/// Exact 1:1 Translation of TypeScript openai-format from codex-reference/api/transform/openai-format.ts
/// DAY 12 AFTERNOON: Translate openai-format.ts

use serde::{Deserialize, Serialize};

/// OpenAI message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum OpenAiMessage {
    #[serde(rename = "user")]
    User {
        content: OpenAiContent,
    },
    #[serde(rename = "assistant")]
    Assistant {
        content: Option<OpenAiContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    #[serde(rename = "tool")]
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAiContent {
    Text(String),
    Parts(Vec<OpenAiContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenAiContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: Function,
    #[serde(rename = "type")]
    pub tool_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub arguments: String,
}

/// Anthropic message types
#[derive(Debug, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicContent,
}

#[derive(Debug, Clone)]
pub enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Debug, Clone)]
pub enum AnthropicContentBlock {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: ToolResultContent,
    },
}

#[derive(Debug, Clone)]
pub struct ImageSource {
    pub source_type: String, // "url" or "base64"
    pub url: Option<String>,
    pub data: Option<String>,
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

/// convertToOpenAiMessages - exact translation lines 4-154
pub fn convert_to_openai_messages(anthropic_messages: Vec<AnthropicMessage>) -> Vec<OpenAiMessage> {
    let mut openai_messages = Vec::new();
    
    for anthropic_message in anthropic_messages {
        match anthropic_message.content {
            AnthropicContent::Text(text) => {
                // Simple string content - line 11
                match anthropic_message.role.as_str() {
                    "user" => openai_messages.push(OpenAiMessage::User {
                        content: OpenAiContent::Text(text),
                    }),
                    "assistant" => openai_messages.push(OpenAiMessage::Assistant {
                        content: Some(OpenAiContent::Text(text)),
                        tool_calls: None,
                    }),
                    _ => {}
                }
            }
            AnthropicContent::Blocks(blocks) => {
                // Complex block content - lines 12-154
                if anthropic_message.role == "user" {
                    // Separate tool results from other content - lines 22-35
                    let mut non_tool_messages = Vec::new();
                    let mut tool_messages = Vec::new();
                    
                    for block in blocks {
                        match block {
                            AnthropicContentBlock::ToolResult { .. } => {
                                tool_messages.push(block);
                            }
                            AnthropicContentBlock::Text { .. } | 
                            AnthropicContentBlock::Image { .. } => {
                                non_tool_messages.push(block);
                            }
                            _ => {} // user cannot send tool_use messages
                        }
                    }
                    
                    // Process tool result messages FIRST - lines 37-62
                    let mut tool_result_images = Vec::new();
                    
                    for tool_message in tool_messages {
                        if let AnthropicContentBlock::ToolResult { tool_use_id, content } = tool_message {
                            let content_str = match content {
                                ToolResultContent::Text(text) => text,
                                ToolResultContent::Parts(parts) => {
                                    parts.into_iter()
                                        .map(|part| match part {
                                            ToolResultPart::Text { text } => text,
                                            ToolResultPart::Image { source } => {
                                                tool_result_images.push(source);
                                                "(see following user message for image)".to_string()
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                }
                            };
                            
                            openai_messages.push(OpenAiMessage::Tool {
                                tool_call_id: tool_use_id,
                                content: content_str,
                            });
                        }
                    }
                    
                    // Process non-tool messages - lines 81-101
                    if !non_tool_messages.is_empty() {
                        let content_parts: Vec<OpenAiContentPart> = non_tool_messages
                            .into_iter()
                            .map(|block| match block {
                                AnthropicContentBlock::Image { source } => {
                                    // Lines 89-94: kilocode_change - support url type
                                    let url = if source.source_type == "url" {
                                        source.url.unwrap_or_default()
                                    } else {
                                        format!("data:{};base64,{}",
                                            source.media_type.unwrap_or_else(|| "image/png".to_string()),
                                            source.data.unwrap_or_default())
                                    };
                                    
                                    OpenAiContentPart::ImageUrl {
                                        image_url: ImageUrl { url },
                                    }
                                }
                                AnthropicContentBlock::Text { text } => {
                                    OpenAiContentPart::Text { text }
                                }
                                _ => OpenAiContentPart::Text { text: String::new() },
                            })
                            .collect();
                        
                        openai_messages.push(OpenAiMessage::User {
                            content: OpenAiContent::Parts(content_parts),
                        });
                    }
                } else if anthropic_message.role == "assistant" {
                    // Process assistant messages - lines 102-154
                    let mut tool_calls = Vec::new();
                    let mut text_parts = Vec::new();
                    
                    for block in blocks {
                        match block {
                            AnthropicContentBlock::ToolUse { id, name, input } => {
                                tool_calls.push(ToolCall {
                                    id,
                                    function: Function {
                                        name,
                                        arguments: input.to_string(),
                                    },
                                    tool_type: "function".to_string(),
                                });
                            }
                            AnthropicContentBlock::Text { text } => {
                                text_parts.push(text);
                            }
                            _ => {}
                        }
                    }
                    
                    let content = if text_parts.is_empty() {
                        None
                    } else {
                        Some(OpenAiContent::Text(text_parts.join("\n")))
                    };
                    
                    let tool_calls_opt = if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    };
                    
                    openai_messages.push(OpenAiMessage::Assistant {
                        content,
                        tool_calls: tool_calls_opt,
                    });
                }
            }
        }
    }
    
    openai_messages
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_convert_simple_messages() {
        let messages = vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::Text("Hello".to_string()),
            },
            AnthropicMessage {
                role: "assistant".to_string(),
                content: AnthropicContent::Text("Hi there".to_string()),
            },
        ];
        
        let result = convert_to_openai_messages(messages);
        assert_eq!(result.len(), 2);
        
        match &result[0] {
            OpenAiMessage::User { content } => {
                match content {
                    OpenAiContent::Text(text) => assert_eq!(text, "Hello"),
                    _ => panic!("Wrong content type"),
                }
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_convert_image_message() {
        let messages = vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::Blocks(vec![
                    AnthropicContentBlock::Text {
                        text: "Look at this:".to_string(),
                    },
                    AnthropicContentBlock::Image {
                        source: ImageSource {
                            source_type: "url".to_string(),
                            url: Some("https://example.com/image.png".to_string()),
                            data: None,
                            media_type: None,
                        },
                    },
                ]),
            },
        ];
        
        let result = convert_to_openai_messages(messages);
        assert_eq!(result.len(), 1);
        
        match &result[0] {
            OpenAiMessage::User { content } => {
                match content {
                    OpenAiContent::Parts(parts) => {
                        assert_eq!(parts.len(), 2);
                        match &parts[0] {
                            OpenAiContentPart::Text { text } => {
                                assert_eq!(text, "Look at this:");
                            }
                            _ => panic!("Wrong part type"),
                        }
                        match &parts[1] {
                            OpenAiContentPart::ImageUrl { image_url } => {
                                assert_eq!(image_url.url, "https://example.com/image.png");
                            }
                            _ => panic!("Wrong part type"),
                        }
                    }
                    _ => panic!("Wrong content type"),
                }
            }
            _ => panic!("Wrong message type"),
        }
    }
}
