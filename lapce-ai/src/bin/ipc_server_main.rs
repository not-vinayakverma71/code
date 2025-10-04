/// IPC Server Main Entry Point with Health Monitoring
use std::sync::Arc;
use lapce_ai_rust::ipc::{IpcServer, HealthServer};
use tokio::signal;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    tracing::info!("Starting Lapce IPC Server with Health Monitoring...");
    
    // Create IPC server
    let socket_path = "/tmp/lapce_ipc.sock";
    let ipc_server = Arc::new(IpcServer::new(socket_path).await?);
    
    // Get metrics handle for health server
    let metrics = ipc_server.metrics();
    
    // Start health server on port 9090
    let health_server = Arc::new(HealthServer::new(metrics));
    let health_handle = tokio::spawn(async move {
        if let Err(e) = health_server.serve().await {
            tracing::error!("Health server error: {}", e);
        }
    });
    
    tracing::info!("Health server started on http://0.0.0.0:9090");
    tracing::info!("  - Health check: http://localhost:9090/health");
    tracing::info!("  - Metrics: http://localhost:9090/metrics");
    
    // Start IPC server
    let ipc_handle = tokio::spawn(async move {
        if let Err(e) = ipc_server.serve().await {
            tracing::error!("IPC server error: {}", e);
        }
    });
    
    tracing::info!("IPC server started on {}", socket_path);
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    tracing::info!("Received shutdown signal, stopping servers...");
    
    // Graceful shutdown
    health_handle.abort();
    ipc_handle.abort();
    
    Ok(())
}
