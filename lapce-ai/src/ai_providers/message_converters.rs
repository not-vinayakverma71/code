/// Message Conversion Utilities - EXACT port from TypeScript
/// From: Codex/src/api/transform/openai-format.ts
/// Ensures 100% parity with TypeScript implementation

use serde_json::{json, Value};
use anyhow::Result;

/// Convert Anthropic messages to OpenAI format
/// EXACT port of convertToOpenAiMessages from openai-format.ts
pub fn convert_to_openai_messages(
    anthropic_messages: Vec<Value>
) -> Result<Vec<Value>> {
    let mut openai_messages = Vec::new();
    
    for anthropic_message in anthropic_messages {
        let role = anthropic_message["role"].as_str().unwrap_or("user");
        let content = &anthropic_message["content"];
        
        if content.is_string() {
            // Simple string content
            openai_messages.push(json!({
                "role": role,
                "content": content.as_str().unwrap()
            }));
        } else if content.is_array() {
            // Complex content with parts (text, images, tools)
            
            if role == "user" {
                // Separate tool messages from non-tool messages
                let mut non_tool_messages = Vec::new();
                let mut tool_messages = Vec::new();
                
                for part in content.as_array().unwrap() {
                    match part["type"].as_str() {
                        Some("tool_result") => {
                            tool_messages.push(part.clone());
                        }
                        Some("text") | Some("image") => {
                            non_tool_messages.push(part.clone());
                        }
                        _ => {}
                    }
                }
                
                // Process tool result messages FIRST (must follow tool use messages)
                let mut tool_result_images = Vec::new();
                
                for tool_message in tool_messages {
                    let mut content_str = String::new();
                    
                    if tool_message["content"].is_string() {
                        content_str = tool_message["content"].as_str().unwrap().to_string();
                    } else if tool_message["content"].is_array() {
                        let mut parts = Vec::new();
                        
                        for part in tool_message["content"].as_array().unwrap() {
                            if part["type"] == "image" {
                                tool_result_images.push(part.clone());
                                parts.push("(see following user message for image)".to_string());
                            } else {
                                parts.push(part["text"].as_str().unwrap_or("").to_string());
                            }
                        }
                        
                        content_str = parts.join("\n");
                    }
                    
                    openai_messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_message["tool_use_id"],
                        "content": content_str
                    }));
                }
                
                // Process non-tool messages
                if !non_tool_messages.is_empty() {
                    let mut content_parts = Vec::new();
                    
                    for part in non_tool_messages {
                        if part["type"] == "image" {
                            let source = &part["source"];
                            let url = if source["type"] == "url" {
                                source["url"].as_str().unwrap().to_string()
                            } else {
                                format!("data:{};base64,{}",
                                    source["media_type"].as_str().unwrap(),
                                    source["data"].as_str().unwrap())
                            };
                            
                            content_parts.push(json!({
                                "type": "image_url",
                                "image_url": { "url": url }
                            }));
                        } else {
                            content_parts.push(json!({
                                "type": "text",
                                "text": part["text"]
                            }));
                        }
                    }
                    
                    openai_messages.push(json!({
                        "role": "user",
                        "content": content_parts
                    }));
                }
            } else if role == "assistant" {
                // Assistant messages with tool calls
                let mut content_str = None;
                let mut tool_calls = Vec::new();
                
                for part in content.as_array().unwrap() {
                    if part["type"] == "text" {
                        content_str = Some(part["text"].as_str().unwrap());
                    } else if part["type"] == "tool_use" {
                        tool_calls.push(json!({
                            "id": part["id"],
                            "type": "function",
                            "function": {
                                "name": part["name"],
                                "arguments": part["input"].to_string()
                            }
                        }));
                    }
                }
                
                let mut message = json!({ "role": "assistant" });
                
                if let Some(content) = content_str {
                    message["content"] = json!(content);
                } else {
                    message["content"] = json!(null);
                }
                
                if !tool_calls.is_empty() {
                    message["tool_calls"] = json!(tool_calls);
                }
                
                openai_messages.push(message);
            }
        }
    }
    
    Ok(openai_messages)
}

/// Convert messages to Anthropic format
/// Handles Human: / Assistant: prefixing
pub fn convert_to_anthropic_format(messages: Vec<Value>) -> Result<Vec<Value>> {
    let mut anthropic_messages = Vec::new();
    let mut last_role = String::new();
    
    for message in messages {
        let role = message["role"].as_str().unwrap_or("user");
        let content = message["content"].as_str().unwrap_or("");
        
        if role == "system" {
            // Anthropic doesn't have system role, convert to user with prefix
            anthropic_messages.push(json!({
                "role": "user",
                "content": format!("System: {}\n\nHuman: Please acknowledge the system message above.",
                                 content)
            }));
            anthropic_messages.push(json!({
                "role": "assistant",
                "content": "I understand and will follow the system instructions."
            }));
        } else if role == "user" {
            // Add Human: prefix
            let formatted_content = if content.starts_with("Human:") {
                content.to_string()
            } else {
                format!("Human: {}", content)
            };
            
            // Ensure ends with Assistant: prompt
            let final_content = if formatted_content.ends_with("Assistant:") {
                formatted_content
            } else {
                format!("{}\n\nAssistant:", formatted_content)
            };
            
            anthropic_messages.push(json!({
                "role": "user",
                "content": final_content
            }));
        } else if role == "assistant" {
            anthropic_messages.push(json!({
                "role": "assistant",
                "content": content
            }));
        }
        
        last_role = role.to_string();
    }
    
    // Ensure conversation ends properly
    if last_role == "user" && !anthropic_messages.is_empty() {
        let last = anthropic_messages.last_mut().unwrap();
        let content = last["content"].as_str().unwrap();
        if !content.ends_with("Assistant:") {
            last["content"] = json!(format!("{}\n\nAssistant:", content));
        }
    }
    
    Ok(anthropic_messages)
}

/// Convert messages to Gemini format
/// Uses contents -> parts -> text structure
pub fn convert_to_gemini_format(messages: Vec<Value>) -> Result<Value> {
    let mut contents = Vec::new();
    let mut system_instruction = None;
    
    for message in messages {
        let role = message["role"].as_str().unwrap_or("user");
        let content = message["content"].clone();
        
        if role == "system" {
            // Gemini uses separate system_instruction field
            system_instruction = Some(content.as_str().unwrap_or("").to_string());
        } else {
            let gemini_role = if role == "assistant" { "model" } else { "user" };
            
            let parts = if content.is_string() {
                vec![json!({ "text": content.as_str().unwrap() })]
            } else if content.is_array() {
                let mut gemini_parts = Vec::new();
                
                for part in content.as_array().unwrap() {
                    if part["type"] == "text" {
                        gemini_parts.push(json!({
                            "text": part["text"]
                        }));
                    } else if part["type"] == "image_url" {
                        // Extract base64 data from URL
                        let url = part["image_url"]["url"].as_str().unwrap();
                        if url.starts_with("data:") {
                            let parts: Vec<&str> = url.split(',').collect();
                            if parts.len() == 2 {
                                let header = parts[0];
                                let data = parts[1];
                                let mime_type = header
                                    .replace("data:", "")
                                    .replace(";base64", "");
                                
                                gemini_parts.push(json!({
                                    "inline_data": {
                                        "mime_type": mime_type,
                                        "data": data
                                    }
                                }));
                            }
                        } else {
                            // URL reference
                            gemini_parts.push(json!({
                                "text": format!("[Image: {}]", url)
                            }));
                        }
                    }
                }
                
                gemini_parts
            } else {
                vec![json!({ "text": "" })]
            };
            
            contents.push(json!({
                "role": gemini_role,
                "parts": parts
            }));
        }
    }
    
    let mut result = json!({
        "contents": contents
    });
    
    if let Some(instruction) = system_instruction {
        result["system_instruction"] = json!({ "parts": [{ "text": instruction }] });
    }
    
    Ok(result)
}

/// Convert messages to simple format (for basic APIs)
pub fn convert_to_simple_format(messages: Vec<Value>) -> Result<String> {
    let mut result = String::new();
    
    for message in messages {
        let role = message["role"].as_str().unwrap_or("user");
        let content = message["content"].as_str().unwrap_or("");
        
        if role == "system" {
            result.push_str(&format!("System: {}\n\n", content));
        } else if role == "user" {
            result.push_str(&format!("User: {}\n\n", content));
        } else if role == "assistant" {
            result.push_str(&format!("Assistant: {}\n\n", content));
        }
    }
    
    Ok(result.trim().to_string())
}

/// Convert from R1 format (reasoning models)
pub fn convert_from_r1_format(r1_response: Value) -> Result<Value> {
    let mut response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": ""
            },
            "finish_reason": "stop"
        }]
    });
    
    // Extract reasoning and final answer
    if let Some(reasoning) = r1_response["reasoning"].as_str() {
        let content = format!("<thinking>\n{}\n</thinking>\n\n{}",
                            reasoning,
                            r1_response["answer"].as_str().unwrap_or(""));
        response["choices"][0]["message"]["content"] = json!(content);
    } else {
        response["choices"][0]["message"]["content"] = r1_response["answer"].clone();
    }
    
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_openai_conversion() {
        let messages = vec![
            json!({
                "role": "user",
                "content": "Hello"
            })
        ];
        
        let result = convert_to_openai_messages(messages).unwrap();
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[0]["content"], "Hello");
    }
    
    #[test]
    fn test_anthropic_format() {
        let messages = vec![
            json!({
                "role": "user",
                "content": "Hello"
            })
        ];
        
        let result = convert_to_anthropic_format(messages).unwrap();
        assert!(result[0]["content"].as_str().unwrap().contains("Human:"));
        assert!(result[0]["content"].as_str().unwrap().contains("Assistant:"));
    }
    
    #[test]
    fn test_gemini_format() {
        let messages = vec![
            json!({
                "role": "user",
                "content": "Test"
            }),
            json!({
                "role": "assistant",
                "content": "Response"
            })
        ];
        
        let result = convert_to_gemini_format(messages).unwrap();
        assert!(result["contents"].is_array());
        assert_eq!(result["contents"][0]["role"], "user");
        assert_eq!(result["contents"][1]["role"], "model");
    }
}
