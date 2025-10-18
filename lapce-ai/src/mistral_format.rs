/// Exact 1:1 Translation of TypeScript mistral-format from codex-reference/api/transform/mistral-format.ts
/// DAY 12 AFTERNOON: Translate mistral-format.ts

use serde::{Deserialize, Serialize};

/// MistralMessage enum - exact translation lines 7-11
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum MistralMessage {
    #[serde(rename = "system")]
    System {
        content: String,
    },
    #[serde(rename = "user")]
    User {
        content: MistralContent,
    },
    #[serde(rename = "assistant")]
    Assistant {
        content: Option<String>,
    },
    #[serde(rename = "tool")]
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MistralContent {
    Text(String),
    Parts(Vec<MistralContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MistralContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl {
        #[serde(rename = "imageUrl")]
        image_url: ImageUrl,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
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
        content: String,
    },
}

#[derive(Debug, Clone)]
pub struct ImageSource {
    pub source_type: String, // "url" or "base64"
    pub url: Option<String>,
    pub data: Option<String>,
    pub media_type: Option<String>,
}

/// convertToMistralMessages - exact translation lines 13-97
pub fn convert_to_mistral_messages(anthropic_messages: Vec<AnthropicMessage>) -> Vec<MistralMessage> {
    let mut mistral_messages = Vec::new();
    
    for anthropic_message in anthropic_messages {
        match anthropic_message.content {
            AnthropicContent::Text(text) => {
                // Simple string content - lines 17-21
                match anthropic_message.role.as_str() {
                    "system" => mistral_messages.push(MistralMessage::System {
                        content: text,
                    }),
                    "user" => mistral_messages.push(MistralMessage::User {
                        content: MistralContent::Text(text),
                    }),
                    "assistant" => mistral_messages.push(MistralMessage::Assistant {
                        content: Some(text),
                    }),
                    _ => {}
                }
            }
            AnthropicContent::Blocks(blocks) => {
                // Complex block content - lines 22-93
                if anthropic_message.role == "user" {
                    // Process user messages - lines 23-59
                    let mut non_tool_messages = Vec::new();
                    let mut _tool_messages = Vec::new();
                    
                    for block in blocks {
                        match block {
                            AnthropicContentBlock::ToolResult { .. } => {
                                _tool_messages.push(block);
                            }
                            AnthropicContentBlock::Text { .. } |
                            AnthropicContentBlock::Image { .. } => {
                                non_tool_messages.push(block);
                            }
                            _ => {} // user cannot send tool_use messages
                        }
                    }
                    
                    // Process non-tool messages - lines 39-59
                    if !non_tool_messages.is_empty() {
                        let content_parts: Vec<MistralContentPart> = non_tool_messages
                            .into_iter()
                            .map(|block| match block {
                                AnthropicContentBlock::Image { source } => {
                                    // Lines 47-52: kilocode_change - support url type
                                    let url = if source.source_type == "url" {
                                        source.url.unwrap_or_default()
                                    } else {
                                        format!("data:{};base64,{}",
                                            source.media_type.unwrap_or_else(|| "image/png".to_string()),
                                            source.data.unwrap_or_default())
                                    };
                                    
                                    MistralContentPart::ImageUrl {
                                        image_url: ImageUrl { url },
                                    }
                                }
                                AnthropicContentBlock::Text { text } => {
                                    MistralContentPart::Text { text }
                                }
                                _ => MistralContentPart::Text { text: String::new() },
                            })
                            .collect();
                        
                        mistral_messages.push(MistralMessage::User {
                            content: MistralContent::Parts(content_parts),
                        });
                    }
                } else if anthropic_message.role == "assistant" {
                    // Process assistant messages - lines 60-92
                    let mut non_tool_messages = Vec::new();
                    let mut _tool_messages = Vec::new();
                    
                    for block in blocks {
                        match block {
                            AnthropicContentBlock::ToolUse { .. } => {
                                _tool_messages.push(block);
                            }
                            AnthropicContentBlock::Text { .. } |
                            AnthropicContentBlock::Image { .. } => {
                                non_tool_messages.push(block);
                            }
                            _ => {} // assistant cannot send tool_result messages
                        }
                    }
                    
                    // Extract text content - lines 76-86
                    let content = if !non_tool_messages.is_empty() {
                        Some(non_tool_messages
                            .into_iter()
                            .filter_map(|block| match block {
                                AnthropicContentBlock::Text { text } => Some(text),
                                AnthropicContentBlock::Image { .. } => None, // assistant cannot send images
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n"))
                    } else {
                        None
                    };
                    
                    mistral_messages.push(MistralMessage::Assistant { content });
                }
            }
        }
    }
    
    mistral_messages
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
        
        let result = convert_to_mistral_messages(messages);
        assert_eq!(result.len(), 2);
        
        match &result[0] {
            MistralMessage::User { content } => {
                match content {
                    MistralContent::Text(text) => assert_eq!(text, "Hello"),
                    _ => panic!("Wrong content type"),
                }
            }
            _ => panic!("Wrong message type"),
        }
        
        match &result[1] {
            MistralMessage::Assistant { content } => {
                assert_eq!(content.as_deref(), Some("Hi there"));
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
        
        let result = convert_to_mistral_messages(messages);
        assert_eq!(result.len(), 1);
        
        match &result[0] {
            MistralMessage::User { content } => {
                match content {
                    MistralContent::Parts(parts) => {
                        assert_eq!(parts.len(), 2);
                        match &parts[0] {
                            MistralContentPart::Text { text } => {
                                assert_eq!(text, "Look at this:");
                            }
                            _ => panic!("Wrong part type"),
                        }
                        match &parts[1] {
                            MistralContentPart::ImageUrl { image_url } => {
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
