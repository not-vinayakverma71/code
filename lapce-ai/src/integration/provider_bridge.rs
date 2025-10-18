/// Provider Bridge - Connects AI providers (Claude, OpenAI) to dispatcher
/// Handles model routing, streaming, and response formatting

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use crate::ai_providers::provider_manager::ProviderManager;
use crate::ai_providers::core_trait::{ChatRequest, ChatMessage, CompletionRequest, StreamToken};
use futures::stream::StreamExt;

/// Bridge between dispatcher and AI provider system
pub struct ProviderBridge {
    provider_manager: Arc<RwLock<ProviderManager>>,
}

impl ProviderBridge {
    pub fn new(provider_manager: Arc<RwLock<ProviderManager>>) -> Self {
        Self {
            provider_manager,
        }
    }
    
    /// Send a completion request to AI provider
    /// Returns a channel that streams response chunks
    pub async fn complete_streaming(
        &self,
        model: &str,
        prompt: &str,
        system_prompt: Option<String>,
    ) -> Result<mpsc::Receiver<StreamChunk>> {
        eprintln!("[PROVIDER BRIDGE] Streaming completion: model={}, prompt_len={}", 
                  model, prompt.len());
        
        let (tx, rx) = mpsc::channel(100);
        
        // Build completion request with system prompt
        let full_prompt = if let Some(sys) = system_prompt {
            format!("{}\n\n{}", sys, prompt)
        } else {
            prompt.to_string()
        };
        
        let request = CompletionRequest {
            model: model.to_string(),
            prompt: full_prompt,
            max_tokens: Some(2048),
            temperature: Some(0.7),
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
        
        // Get streaming response from provider manager
        let manager = self.provider_manager.read().await;
        let mut stream = manager.complete_stream(request).await?;
        drop(manager);
        
        // Spawn task to convert StreamTokens to StreamChunks
        tokio::spawn(async move {
            let mut index = 0;
            while let Some(token_result) = stream.next().await {
                match token_result {
                    Ok(token) => {
                        match token {
                            StreamToken::Text(text) => {
                                if tx.send(StreamChunk {
                                    text,
                                    index,
                                    is_final: false,
                                }).await.is_err() {
                                    break;
                                }
                                index += 1;
                            }
                            StreamToken::Done => {
                                let _ = tx.send(StreamChunk {
                                    text: String::new(),
                                    index,
                                    is_final: true,
                                }).await;
                                break;
                            }
                            StreamToken::Error(err) => {
                                eprintln!("[PROVIDER BRIDGE] Stream error: {}", err);
                                break;
                            }
                            _ => {} // Ignore other token types
                        }
                    }
                    Err(e) => {
                        eprintln!("[PROVIDER BRIDGE] Stream error: {}", e);
                        break;
                    }
                }
            }
        });
        
        Ok(rx)
    }
    
    /// Non-streaming completion
    pub async fn complete(
        &self,
        model: &str,
        prompt: &str,
        system_prompt: Option<String>,
    ) -> Result<String> {
        eprintln!("[PROVIDER BRIDGE] Completion: model={}", model);
        
        // Build completion request
        let full_prompt = if let Some(sys) = system_prompt {
            format!("{}\n\n{}", sys, prompt)
        } else {
            prompt.to_string()
        };
        
        let request = CompletionRequest {
            model: model.to_string(),
            prompt: full_prompt,
            max_tokens: Some(2048),
            temperature: Some(0.7),
            top_p: None,
            stop: None,
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
        
        // Get response from provider manager
        let manager = self.provider_manager.read().await;
        let response = manager.complete(request).await?;
        
        // Extract text from first choice
        let text = response.choices
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();
        
        Ok(text)
    }
    
    /// List available models
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // TODO Phase C: Get from actual provider manager
        Ok(vec![
            ModelInfo {
                id: "claude-sonnet-4".to_string(),
                name: "Claude Sonnet 4".to_string(),
                provider: "Anthropic".to_string(),
                context_window: 200000,
            },
            ModelInfo {
                id: "gpt-4".to_string(),
                name: "GPT-4".to_string(),
                provider: "OpenAI".to_string(),
                context_window: 128000,
            },
        ])
    }
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub text: String,
    pub index: u64,
    pub is_final: bool,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_provider_bridge_streaming() {
        let bridge = ProviderBridge::new();
        
        let mut rx = bridge.complete_streaming(
            "claude-sonnet-4",
            "Hello",
            None,
        ).await.unwrap();
        
        let mut chunks = vec![];
        while let Some(chunk) = rx.recv().await {
            if chunk.is_final {
                break;
            }
            chunks.push(chunk);
        }
        
        assert!(!chunks.is_empty());
    }
    
    #[tokio::test]
    async fn test_provider_bridge_list_models() {
        let bridge = ProviderBridge::new();
        let models = bridge.list_models().await.unwrap();
        assert!(!models.is_empty());
    }
}
