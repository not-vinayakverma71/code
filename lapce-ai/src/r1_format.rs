/// Exact 1:1 Translation of TypeScript r1-format from codex-reference/api/transform/r1-format.ts
/// DAY 12 MORNING: Translate r1-format.ts

use serde::{Deserialize, Serialize};

/// ContentPartText - exact translation line 4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPartText {
    #[serde(rename = "type")]
    pub content_type: String, // "text"
    pub text: String,
}

/// ContentPartImage - exact translation line 5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPartImage {
    #[serde(rename = "type")]
    pub content_type: String, // "image_url"
    pub image_url: ImageUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// UserMessage - exact translation line 6
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub role: String, // "user"
    pub content: MessageContent,
}

/// AssistantMessage - exact translation line 7
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub role: String, // "assistant"
    pub content: MessageContent,
}

/// Message enum - exact translation line 8
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
}

/// MessageContent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentPart {
    Text(ContentPartText),
    Image(ContentPartImage),
}

/// AnthropicMessage - exact translation line 9
#[derive(Debug, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicContent,
}

#[derive(Debug, Clone)]
pub enum AnthropicContent {
    Text(String),
    Parts(Vec<AnthropicContentPart>),
}

#[derive(Debug, Clone)]
pub struct AnthropicContentPart {
    pub part_type: String,
    pub text: Option<String>,
    pub source: Option<ImageSource>,
}

#[derive(Debug, Clone)]
pub struct ImageSource {
    pub source_type: String, // "url" or "base64"
    pub url: Option<String>,
    pub data: Option<String>,
    pub media_type: Option<String>,
}

/// convertToR1Format - exact translation lines 18-106
/// Converts Anthropic messages to OpenAI format while merging consecutive messages with the same role.
/// This is required for DeepSeek Reasoner which does not support successive messages with the same role.
pub fn convert_to_r1_format(messages: Vec<AnthropicMessage>) -> Vec<Message> {
    let mut merged: Vec<Message> = Vec::new();
    
    for message in messages {
        let last_message_idx = if merged.is_empty() {
            None
        } else {
            Some(merged.len() - 1)
        };
        
        let mut message_content: MessageContent;
        let mut has_images = false;
        
        // Convert content to appropriate format - lines 25-61
        match &message.content {
            AnthropicContent::Parts(parts) => {
                let mut text_parts: Vec<String> = Vec::new();
                let mut image_parts: Vec<ContentPartImage> = Vec::new();
                
                for part in parts {
                    if part.part_type == "text" {
                        if let Some(text) = &part.text {
                            text_parts.push(text.clone());
                        }
                    }
                    if part.part_type == "image" {
                        has_images = true;
                        if let Some(source) = &part.source {
                            // Lines 39-43: Support both url and base64 sources
                            let url = if source.source_type == "url" {
                                source.url.clone().unwrap_or_default()
                            } else {
                                format!("data:{};base64,{}", 
                                    source.media_type.as_ref().unwrap_or(&"image/png".to_string()),
                                    source.data.as_ref().unwrap_or(&String::new()))
                            };
                            
                            image_parts.push(ContentPartImage {
                                content_type: "image_url".to_string(),
                                image_url: ImageUrl { url },
                            });
                        }
                    }
                }
                
                if has_images {
                    let mut parts: Vec<ContentPart> = Vec::new();
                    if !text_parts.is_empty() {
                        parts.push(ContentPart::Text(ContentPartText {
                            content_type: "text".to_string(),
                            text: text_parts.join("\n"),
                        }));
                    }
                    for img in image_parts {
                        parts.push(ContentPart::Image(img));
                    }
                    message_content = MessageContent::Parts(parts);
                } else {
                    message_content = MessageContent::Text(text_parts.join("\n"));
                }
            }
            AnthropicContent::Text(text) => {
                message_content = MessageContent::Text(text.clone());
            }
        }
        
        // If last message has same role, merge the content - lines 64-85
        if let Some(idx) = last_message_idx {
            let should_merge = match &merged[idx] {
                Message::User(u) => u.role == message.role,
                Message::Assistant(a) => a.role == message.role,
            };
            
            if should_merge {
                // Merge content based on types
                match &mut merged[idx] {
                    Message::User(ref mut user_msg) => {
                        user_msg.content = merge_content(user_msg.content.clone(), message_content);
                    }
                    Message::Assistant(ref mut asst_msg) => {
                        asst_msg.content = merge_content(asst_msg.content.clone(), message_content);
                    }
                }
            } else {
                // Add as new message - lines 86-100
                add_new_message(&mut merged, message.role, message_content);
            }
        } else {
            // First message, just add it
            add_new_message(&mut merged, message.role, message_content);
        }
    }
    
    merged
}

fn merge_content(existing: MessageContent, new: MessageContent) -> MessageContent {
    match (existing, new) {
        (MessageContent::Text(t1), MessageContent::Text(t2)) => {
            MessageContent::Text(format!("{}\n{}", t1, t2))
        }
        (existing, new) => {
            // Convert to parts format for merging
            let mut parts = Vec::new();
            
            // Add existing content
            match existing {
                MessageContent::Text(text) => {
                    parts.push(ContentPart::Text(ContentPartText {
                        content_type: "text".to_string(),
                        text,
                    }));
                }
                MessageContent::Parts(mut p) => {
                    parts.append(&mut p);
                }
            }
            
            // Add new content
            match new {
                MessageContent::Text(text) => {
                    parts.push(ContentPart::Text(ContentPartText {
                        content_type: "text".to_string(),
                        text,
                    }));
                }
                MessageContent::Parts(mut p) => {
                    parts.append(&mut p);
                }
            }
            
            MessageContent::Parts(parts)
        }
    }
}

fn add_new_message(merged: &mut Vec<Message>, role: String, content: MessageContent) {
    if role == "assistant" {
        merged.push(Message::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            content,
        }));
    } else {
        merged.push(Message::User(UserMessage {
            role: "user".to_string(),
            content,
        }));
    }
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
        
        let result = convert_to_r1_format(messages);
        assert_eq!(result.len(), 2);
        
        match &result[0] {
            Message::User(u) => {
                assert_eq!(u.role, "user");
                match &u.content {
                    MessageContent::Text(t) => assert_eq!(t, "Hello"),
                    _ => panic!("Wrong content type"),
                }
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_merge_consecutive_same_role() {
        let messages = vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::Text("First".to_string()),
            },
            AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::Text("Second".to_string()),
            },
            AnthropicMessage {
                role: "assistant".to_string(),
                content: AnthropicContent::Text("Response".to_string()),
            },
        ];
        
        let result = convert_to_r1_format(messages);
        assert_eq!(result.len(), 2); // Should merge the two user messages
        
        match &result[0] {
            Message::User(u) => {
                match &u.content {
                    MessageContent::Text(t) => assert_eq!(t, "First\nSecond"),
                    _ => panic!("Wrong content type"),
                }
            }
            _ => panic!("Wrong message type"),
        }
    }
}
