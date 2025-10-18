/// Lapce IPC Server - Production Binary
/// Main entry point for the IPC server with all integrations

use std::sync::Arc;
use anyhow::{Result, Context};
use tokio::signal;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use lapce_ai_rust::{
    IpcConfig,
    IpcError,
    StreamToken,
    AutoReconnectionManager, ReconnectionStrategy,
};
use lapce_ai_rust::ipc::ipc_server_volatile::IpcServerVolatile;
use lapce_ai_rust::ipc::provider_config::{load_provider_configs_from_env, validate_provider_configs};
use lapce_ai_rust::ipc::provider_routes::ProviderRouteHandler;
use lapce_ai_rust::ai_providers::provider_manager::{ProviderManager, ProvidersConfig, ProviderConfig};
use lapce_ai_rust::ai_providers::provider_registry::ProviderRegistry;
use std::collections::HashMap;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    if let Err(e) = init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        return Err(e);
    }
    
    info!("Starting Lapce IPC Server");
    
    // Load configuration
    let config_path = std::env::var("LAPCE_CONFIG_PATH")
        .unwrap_or_else(|_| "lapce-ipc.toml".to_string());
    
    let config = IpcConfig::create_default_if_missing(&config_path)
        .context("Failed to load configuration")?
        .apply_env_overrides();
    
    config.validate()
        .context("Invalid configuration")?;
    
    info!("Configuration loaded from: {}", config_path);
    
    // Validate and load provider configs from environment
    validate_provider_configs()
        .context("Failed to validate provider configuration")?;
    
    let provider_configs = load_provider_configs_from_env()
        .context("Failed to load provider configs")?;
    
    // Build ProviderManager config
    let mut providers_map = HashMap::new();
    for (name, init_config) in provider_configs {
        let provider_config = ProviderConfig {
            name: name.clone(),
            api_key: init_config.api_key.unwrap_or_default(),
            base_url: init_config.base_url,
            max_retries: 3,
            timeout: std::time::Duration::from_secs(60),
            rate_limit_override: None,
        };
        providers_map.insert(name, provider_config);
    }
    
    let providers_config = ProvidersConfig {
        providers: providers_map.clone(),
        default_provider: "openai".to_string(),
        health_check_interval: std::time::Duration::from_secs(30),
        circuit_breaker_threshold: 5,
        circuit_breaker_timeout: std::time::Duration::from_secs(60),
    };
    
    // Initialize ProviderManager
    let provider_manager = ProviderManager::new(providers_config).await
        .context("Failed to create ProviderManager")?;
    
    info!("Provider manager initialized with {} providers", providers_map.len());
    
    // Create IPC server (volatile version with control socket)
    let server = IpcServerVolatile::new(&config.server.socket_path).await
        .context("Failed to create IPC server")?;
    
    info!("IPC server created at: {}", config.server.socket_path);
    
    // Create provider route handler
    let provider_handler = Arc::new(ProviderRouteHandler::new(
        Arc::new(RwLock::new(provider_manager))
    ));
    
    // Register provider chat streaming handler
    let provider_handler_stream = provider_handler.clone();
    use lapce_ai_rust::ipc::binary_codec::MessageType;
    server.register_streaming_handler(
        MessageType::ChatMessage,  // Use ChatMessage for provider streaming
        move |data, tx| {
            let handler = provider_handler_stream.clone();
            async move {
                use lapce_ai_rust::ipc::ipc_messages::{ProviderChatStreamRequest, ProviderStreamChunk, ProviderStreamDone};
                use futures::StreamExt;
                
                // Deserialize request
                let request: ProviderChatStreamRequest = serde_json::from_slice(&data)
                    .map_err(|e| IpcError::Protocol {
                        message: format!("Failed to parse ProviderChatStreamRequest: {}", e)
                    })?;
                
                info!("[Provider] Streaming chat request: model={}, {} messages", 
                      request.model, request.messages.len());
                
                // Convert messages to JSON values for handler
                let json_messages: Vec<serde_json::Value> = request.messages.iter()
                    .map(|m| serde_json::json!({
                        "role": m.role,
                        "content": m.content,
                    }))
                    .collect();
                
                // Get streaming response
                let mut stream = handler.handle_chat_stream(
                    request.model,
                    json_messages,
                    request.max_tokens,
                    request.temperature,
                ).await
                    .map_err(|e| IpcError::Internal {
                        context: format!("Provider streaming failed: {}", e)
                    })?;
                
                // Stream chunks
                while let Some(token_result) = stream.next().await {
                    match token_result {
                        Ok(token) => {
                            // Use StreamToken from root export
                            match token {
                                StreamToken::Text(text) => {
                                    let chunk = ProviderStreamChunk { content: text };
                                    let chunk_bytes = serde_json::to_vec(&chunk)
                                        .map_err(|e| IpcError::Protocol {
                                            message: format!("Failed to serialize chunk: {}", e)
                                        })?;
                                    tx.send(bytes::Bytes::from(chunk_bytes)).await
                                        .map_err(|_| IpcError::Internal {
                                            context: "Channel closed".to_string()
                                        })?;
                                }
                                StreamToken::Done => {
                                    let done = ProviderStreamDone { usage: None };
                                    let done_bytes = serde_json::to_vec(&done)
                                        .map_err(|e| IpcError::Protocol {
                                            message: format!("Failed to serialize done: {}", e)
                                        })?;
                                    tx.send(bytes::Bytes::from(done_bytes)).await
                                        .map_err(|_| IpcError::Internal {
                                            context: "Channel closed".to_string()
                                        })?;
                                    break;
                                }
                                StreamToken::Error(err) => {
                                    error!("[Provider] Stream error: {}", err);
                                    return Err(IpcError::Internal {
                                        context: format!("Stream error: {}", err)
                                    });
                                }
                                _ => {} // Ignore other token types for now
                            }
                        }
                        Err(e) => {
                            error!("[Provider] Token error: {}", e);
                            return Err(IpcError::Internal {
                                context: format!("Token error: {}", e)
                            });
                        }
                    }
                }
                
                Ok(())
            }
        },
    );
    
    info!("Provider streaming handler registered");
    
    // Register LSP gateway handler if enabled
    #[cfg(feature = "lsp_gateway")]
    {
        use lapce_ai_rust::lsp_gateway::native::LspGateway;
        use lapce_ai_rust::ipc::binary_codec::LspRequestPayload;
        
        let lsp_gateway = Arc::new(LspGateway::new());
        info!("LSP Gateway initialized");
        
        let gateway_handler = lsp_gateway.clone();
        server.register_streaming_handler(
            MessageType::LspRequest,
            move |data, tx| {
                let gateway = gateway_handler.clone();
                async move {
                    info!("[LSP] Received request, {} bytes", data.len());
                    
                    // Handle request through gateway (expects Bytes, returns Bytes)
                    let response_bytes = gateway.handle_request(data).await
                        .map_err(|e| IpcError::Internal {
                            context: format!("LSP gateway error: {}", e)
                        })?;
                    
                    info!("[LSP] Sending response, {} bytes", response_bytes.len());
                    
                    // Send response
                    tx.send(response_bytes).await
                        .map_err(|_| IpcError::Internal {
                            context: "Channel closed".to_string()
                        })?;
                    
                    Ok(())
                }
            },
        );
        
        info!("LSP Gateway handler registered");
    }
    
    #[cfg(not(feature = "lsp_gateway"))]
    {
        info!("LSP Gateway disabled (lsp_gateway feature not enabled)");
    }
    
    // Server is already Arc from IpcServerVolatile::new()
    
    // Setup auto-reconnection if enabled
    let reconnection_manager = if config.server.enable_auto_reconnect {
        let manager = Arc::new(AutoReconnectionManager::new(
            ReconnectionStrategy::Fixed { delay_ms: 5000 }
        ));
        
        // Start reconnection manager
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.start().await;
        });
        
        info!("Auto-reconnection manager started");
        Some(manager)
    } else {
        None
    };
    
    // Setup metrics server if enabled
    // Note: IpcServerVolatile doesn't have full metrics yet
    if config.monitoring.enable_metrics {
        info!("Metrics server requested but not available with IpcServerVolatile");
        // TODO: Implement metrics for IpcServerVolatile
    }
    
    // Setup graceful shutdown
    let shutdown_server = server.clone();
    
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received shutdown signal");
                shutdown_server.shutdown();
                
                // Give connections time to close gracefully
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Err(err) => {
                error!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });
    
    // Start health check task
    let health_check_interval = tokio::time::Duration::from_secs(
        config.monitoring.health_check_interval_secs
    );
    let health_server = server.clone();
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(health_check_interval);
        loop {
            interval.tick().await;
            
            // Check server health
            let _metrics = health_server.metrics();
            
            info!("Health check - Server running");
            
            // TODO: Add more health checks
        }
    });
    
    // Start the server
    info!("Starting IPC server...");
    info!("Performance targets:");
    info!("  - Memory: < 3MB");
    info!("  - Latency: < 10μs");
    info!("  - Throughput: > 1M msg/sec");
    info!("  - Connections: 1000+");
    
    server.serve().await
        .context("Server error")?;
    
    info!("IPC server stopped");
    
    // Cleanup
    if let Some(manager) = reconnection_manager {
        manager.stop();
    }
    
    Ok(())
}

fn init_logging() -> Result<()> {
    let log_level = std::env::var("LAPCE_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string());
    
    let filter = match log_level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_thread_names(true)
        )
        .with(tracing_subscriber::filter::LevelFilter::from_level(filter))
        .init();
    
    Ok(())
}

async fn start_metrics_server(
    port: u16,
    endpoint: String,
    metrics: Arc<lapce_ai_rust::ipc::ipc_server::Metrics>,
) -> Result<()> {
    use warp::Filter;
    
    let endpoint = endpoint.trim_start_matches('/').to_string();
    
    let routes = warp::path(endpoint)
        .map(move || {
            // Return real metrics from IpcServer
            metrics.export_prometheus()
        });
    
    tokio::spawn(async move {
        warp::serve(routes)
            .run(([127, 0, 0, 1], port))
            .await;
    });
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_server_startup() {
        // Create temp config
        let config = IpcConfig {
            server: lapce_ai_rust::ipc_config::ServerConfig {
                socket_path: "/tmp/test_lapce_ipc.sock".to_string(),
                max_connections: 10,
                idle_timeout_secs: 10,
                max_message_size: 1024 * 1024,
                buffer_pool_size: 10,
                enable_auto_reconnect: false,
                reconnect_delay_ms: 100,
            },
            ..Default::default()
        };
        
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_str().unwrap();
        config.save(config_path).unwrap();
        
        // Test config loading
        let loaded = IpcConfig::from_file(config_path).unwrap();
        assert_eq!(loaded.server.socket_path, config.server.socket_path);
    }
}
