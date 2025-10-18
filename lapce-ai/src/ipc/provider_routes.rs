// IPC route handlers for AI provider operations
// Implements provider routing with streaming support

use super::ipc_messages::{ProviderCommand, ProviderResponse};
use crate::ai_providers::provider_manager::ProviderManager;
use crate::ai_providers::core_trait::{CompletionRequest, ChatRequest, ChatMessage, StreamToken};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use futures::stream::StreamExt;

/// Provider route handler
pub struct ProviderRouteHandler {
    manager: Arc<RwLock<ProviderManager>>,
}

impl ProviderRouteHandler {
    /// Create new provider route handler
    pub fn new(manager: Arc<RwLock<ProviderManager>>) -> Self {
        Self { manager }
    }
    
    /// Handle provider command
    pub async fn handle_command(&self, command: ProviderCommand) -> ProviderResponse {
        match command {
            ProviderCommand::Complete {
                model,
                prompt,
                max_tokens,
                temperature,
                top_p,
                stop,
            } => {
                self.handle_complete(model, prompt, max_tokens, temperature, top_p, stop)
                    .await
            }
            
            ProviderCommand::Chat {
                model,
                messages,
                max_tokens,
                temperature,
                tools,
            } => {
                self.handle_chat(model, messages, max_tokens, temperature, tools)
                    .await
            }
            
            // Streaming variants handled separately (need async stream support in IPC)
            ProviderCommand::CompleteStream { .. } | ProviderCommand::ChatStream { .. } => {
                ProviderResponse::Error {
                    message: "Streaming not yet wired to IPC - use handle_stream methods".to_string(),
                }
            }
        }
    }
    
    /// Handle streaming completion (returns stream tokens)
    pub async fn handle_complete_stream(
        &self,
        model: String,
        prompt: String,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<impl futures::stream::Stream<Item = Result<StreamToken>>> {
        let request = CompletionRequest {
            model,
            prompt,
            max_tokens,
            temperature,
            top_p: None,
            stop: None,
            n: None,
            stream: Some(true),
            logprobs: None,
            echo: None,
            best_of: None,
            logit_bias: None,
            user: None,
            suffix: None,
            presence_penalty: None,
            frequency_penalty: None,
        };
        
        let manager = self.manager.read().await;
        let stream = manager.complete_stream(request).await?;
        Ok(stream)
    }
    
    /// Handle streaming chat (returns stream tokens)
    pub async fn handle_chat_stream(
        &self,
        model: String,
        messages: Vec<serde_json::Value>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<impl futures::stream::Stream<Item = Result<StreamToken>>> {
        // Convert JSON messages to ChatMessage
        let chat_messages: Vec<ChatMessage> = messages
            .into_iter()
            .filter_map(|msg| serde_json::from_value(msg).ok())
            .collect();
        
        let request = ChatRequest {
            model,
            messages: chat_messages,
            max_tokens,
            temperature,
            top_p: None,
            n: None,
            stream: Some(true),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };
        
        let manager = self.manager.read().await;
        let stream = manager.chat_stream(request).await?;
        Ok(stream)
    }
    
    /// Handle non-streaming completion
    async fn handle_complete(
        &self,
        model: String,
        prompt: String,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        top_p: Option<f32>,
        stop: Option<Vec<String>>,
    ) -> ProviderResponse {
        let request = CompletionRequest {
            model,
            prompt,
            max_tokens,
            temperature,
            top_p,
            stop,
            n: None,
            stream: Some(false),
            logprobs: None,
            echo: None,
            best_of: None,
            logit_bias: None,
            user: None,
            suffix: None,
            presence_penalty: None,
            frequency_penalty: None,
        };
        
        let manager = self.manager.read().await;
        match manager.complete(request).await {
            Ok(response) => {
                // Extract text from first choice
                let text = response
                    .choices
                    .first()
                    .and_then(|c| c.text.clone())
                    .unwrap_or_default();
                
                ProviderResponse::Complete {
                    id: response.id,
                    text,
                    usage: response.usage.and_then(|u| serde_json::to_value(u).ok()),
                }
            }
            Err(e) => ProviderResponse::Error {
                message: e.to_string(),
            },
        }
    }
    
    /// Handle non-streaming chat
    async fn handle_chat(
        &self,
        model: String,
        messages: Vec<serde_json::Value>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tools: Option<Vec<serde_json::Value>>,
    ) -> ProviderResponse {
        // Convert JSON messages to ChatMessage
        let chat_messages: Vec<ChatMessage> = messages
            .into_iter()
            .filter_map(|msg| serde_json::from_value(msg).ok())
            .collect();
        
        let request = ChatRequest {
            model,
            messages: chat_messages,
            max_tokens,
            temperature,
            top_p: None,
            n: None,
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            functions: None,
            function_call: None,
            tools,
            tool_choice: None,
            response_format: None,
        };
        
        let manager = self.manager.read().await;
        match manager.chat(request).await {
            Ok(response) => {
                // Extract content from first choice
                let content = response
                    .choices
                    .first()
                    .and_then(|c| c.message.content.clone())
                    .unwrap_or_default();
                
                // Extract tool calls if present
                let tool_calls = response
                    .choices
                    .first()
                    .and_then(|c| c.message.tool_calls.clone())
                    .and_then(|calls| serde_json::to_value(calls).ok());
                
                ProviderResponse::Chat {
                    id: response.id,
                    content,
                    usage: response.usage.and_then(|u| serde_json::to_value(u).ok()),
                    tool_calls,
                }
            }
            Err(e) => ProviderResponse::Error {
                message: e.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_providers::provider_manager::{ProviderManager, ProvidersConfig};
    
    #[tokio::test]
    async fn test_provider_route_handler_creation() {
        let config = ProvidersConfig::default();
        let manager = ProviderManager::new(config).await.expect("Failed to create manager");
        let handler = ProviderRouteHandler::new(Arc::new(RwLock::new(manager)));
        
        // Test that handler is created successfully
        assert!(true);
    }
}
