/// Eternix AI Server - Standalone process for AI features
/// Runs separately from editor, communicates via IPC

use std::sync::Arc;
use anyhow::Result;
use tokio::signal;
use tracing::{info, error};

use lapce_ai_rust::{
    ipc_server::IpcServer,
    provider_pool::{ProviderPool, ProviderPoolConfig, ProviderConfig},
    shared_memory_lapce::LapceMemoryManager,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,lapce_ai_rust=debug")
        .init();
    
    info!("🚀 Starting Eternix AI Server");
    
    // Initialize memory manager
    let memory_manager = Arc::new(LapceMemoryManager::new()?);
    info!("✅ Memory manager initialized");
    
    // Create IPC server
    let socket_path = "/tmp/eternix-ai.sock";
    let mut ipc_server = IpcServer::new(socket_path).await?;
    info!("✅ IPC server listening on {}", socket_path);
    
    // Configure AI providers
    let provider_config = ProviderPoolConfig {
        providers: vec![
            ProviderConfig {
                name: "gemini".to_string(),
                enabled: true,
                api_key: std::env::var("GEMINI_API_KEY").ok(),
                base_url: None,
                default_model: Some("gemini-1.5-flash".to_string()),
                max_retries: 3,
                timeout_secs: 30,
                rate_limit_per_minute: None,
            },
            ProviderConfig {
                name: "openai".to_string(),
                enabled: true,
                api_key: std::env::var("OPENAI_API_KEY").ok(),
                base_url: None,
                default_model: Some("gpt-4o-mini".to_string()),
                max_retries: 3,
                timeout_secs: 30,
                rate_limit_per_minute: None,
            }
        ],
        fallback_enabled: true,
        fallback_order: vec!["gemini".to_string(), "openai".to_string()],
        load_balance: false,
        circuit_breaker_enabled: true,
        circuit_breaker_threshold: 5,
    };
    
    let provider_pool = Arc::new(ProviderPool::new().await?);
    ipc_server.register_provider_pool(provider_pool);
    info!("✅ Provider pool registered with {} providers", 2);
    
    // Register handlers
    ipc_server.register_handlers();
    info!("✅ All handlers registered");
    
    // Start server in background
    let server_handle = {
        let server = Arc::new(ipc_server);
        tokio::spawn(async move {
            if let Err(e) = server.serve().await {
                error!("Server error: {}", e);
            }
        })
    };
    
    info!("🎯 Eternix AI Server ready!");
    info!("   Memory: {:.2} MB", memory_manager.get_memory_usage() as f64 / 1024.0 / 1024.0);
    info!("   Socket: {}", socket_path);
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutting down...");
    
    server_handle.abort();
    Ok(())
}
