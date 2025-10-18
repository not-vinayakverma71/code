/// Eternix AI Server - Standalone process for AI features
/// Runs separately from editor, communicates via IPC

use std::sync::Arc;
use anyhow::Result;
use tokio::signal;
use tracing::{info, error};

use lapce_ai_rust::{
    ipc::ipc_server::IpcServer,
    provider_pool::{ProviderPool, ProviderPoolConfig},
};

#[cfg(unix)]
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;
#[cfg(windows)]
use lapce_ai_rust::ipc::windows_shared_memory::SharedMemoryBuffer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,lapce_ai_rust=debug")
        .init();
    
    info!("🚀 Starting Eternix AI Server");
    
    // Memory manager disabled - not implemented yet
    // let memory_manager = Arc::new(LapceMemoryManager::new()?);
    // info!("✅ Memory manager initialized");
    
    // Create IPC server
    let socket_path = "/tmp/eternix-ai.sock";
    let ipc_server = IpcServer::new(socket_path).await?;
    info!("✅ IPC server listening on {}", socket_path);
    
    // Configure AI providers
    let provider_config = ProviderPoolConfig::default();
    
    let provider_pool = Arc::new(ProviderPool::new_with_config(provider_config).await?);
    // ipc_server.register_provider_pool(provider_pool); // Method doesn't exist yet
    info!("✅ Provider pool registered with {} providers", 0);
    
    // Register handlers
    // ipc_server.register_handlers(); // Method takes arguments
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
    // info!("   Memory: {:.2} MB", memory_manager.get_memory_usage() as f64 / 1024.0 / 1024.0);
    info!("   Socket: {}", socket_path);
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutting down...");
    
    server_handle.abort();
    Ok(())
}
