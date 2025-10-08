/// Main production server binary for lapce-ai-rust

use anyhow::{Result, Context};
use clap::Parser;
use tokio::signal;
use tracing::{info, warn, error};
use std::sync::Arc;
use std::path::Path;

use lapce_ai_rust::{
    ipc::ipc_server::IpcServer,
    ipc::ipc_config::IpcConfig,
    ai_providers::provider_manager::{ProviderManager, ProviderConfig},
};

#[derive(Parser, Debug)]
#[clap(
    name = "lapce-ai-server",
    version = env!("CARGO_PKG_VERSION"),
    author = "Lapce Team",
    about = "High-performance IPC server for Lapce AI integration"
)]
struct Args {
    /// Path to configuration file
    #[clap(short, long, default_value = "/etc/lapce-ai/config.toml")]
    config: String,
    
    /// Socket path for IPC (overrides config)
    #[clap(short, long)]
    socket: Option<String>,
    
    /// Enable debug logging
    #[clap(short, long)]
    debug: bool,
    
    /// Enable metrics endpoint
    #[clap(short, long)]
    metrics: bool,
    
    /// Metrics port
    #[clap(short = 'p', long, default_value = "9090")]
    metrics_port: u16,
    
    /// Dry run - validate config and exit
    #[clap(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.debug { 
        "debug,lapce_ai_rust=trace" 
    } else { 
        std::env::var("RUST_LOG")
            .as_deref()
            .unwrap_or("info")
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    info!("Starting Lapce AI Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Rust version: {}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config_path = Path::new(&args.config);
    let mut config = if config_path.exists() {
        info!("Loading config from: {}", args.config);
        IpcConfig::from_file(&args.config)?
    } else {
        info!("Using default configuration");
        IpcConfig::default()
    };
    
    // Apply command-line overrides
    if let Some(socket) = args.socket {
        config.ipc.socket_path = socket;
    }
    if let Some(port) = args.metrics_port {
        config.monitoring.prometheus_port = port;
    }
    
    info!("Configuration loaded successfully");
    
    if args.dry_run {
        info!("Dry run mode - exiting");
        return Ok(());
    }
    
    // Initialize provider manager
    let provider_manager = Arc::new(ProviderManager::new());
    info!("Provider manager initialized");
    
    // Create and configure server
    let server = IpcServer::new(config.clone());
    info!("IPC server created on socket: {}", config.ipc.socket_path);
    info!("  Max connections: {}", config.ipc.max_connections);
    
    // Setup signal handlers
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received SIGINT, shutting down...");
            }
            Err(e) => {
                error!("Failed to listen for SIGINT: {}", e);
            }
        }
    });
    
    // Start server
    info!("Starting IPC server...");
    match server.run().await {
        Ok(_) => {
            info!("Server shut down cleanly");
        }
        Err(e) => {
            error!("Server error: {}", e);
            return Err(e);
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(true) = sd_notify::booted() {
            sd_notify::notify(true, &[sd_notify::NotifyState::Stopping])?;
        }
    }
    
    Ok(())
}

async fn load_provider_config(config_path: &str) -> Result<ProviderPoolConfig> {
    if std::path::Path::new(config_path).exists() {
        let contents = std::fs::read_to_string(config_path)?;
        
        // Parse TOML and extract provider configuration
        let toml: toml::Value = toml::from_str(&contents)?;
        
        let mut provider_config = ProviderPoolConfig::default();
        
        // Parse providers array
        if let Some(providers) = toml.get("providers").and_then(|v| v.as_array()) {
            provider_config.providers.clear();
            
            for provider in providers {
                if let Some(table) = provider.as_table() {
                    let provider_cfg = lapce_ai_rust::provider_pool::ProviderConfig {
                        name: table.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        enabled: table.get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        api_key: table.get("api_key")
                            .and_then(|v| v.as_str())
                            .map(|s| expand_env_var(s)),
                        base_url: table.get("base_url")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        default_model: table.get("default_model")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        max_retries: table.get("max_retries")
                            .and_then(|v| v.as_integer())
                            .unwrap_or(3) as u32,
                        timeout_secs: table.get("timeout_secs")
                            .and_then(|v| v.as_integer())
                            .unwrap_or(30) as u64,
                        rate_limit_per_minute: table.get("rate_limit_per_minute")
                            .and_then(|v| v.as_integer())
                            .map(|v| v as u32),
                    };
                    
                    provider_config.providers.push(provider_cfg);
                }
            }
        }
        
        // Parse pool configuration
        if let Some(toml) = toml.as_table() {
            provider_config.fallback_enabled = toml.get("fallback_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            
            if let Some(order) = toml.get("fallback_order").and_then(|v| v.as_array()) {
                provider_config.fallback_order = order.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            
            provider_config.load_balance = toml.get("load_balance")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            provider_config.circuit_breaker_enabled = toml.get("circuit_breaker_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            
            provider_config.circuit_breaker_threshold = toml.get("circuit_breaker_threshold")
                .and_then(|v| v.as_integer())
                .unwrap_or(5) as u32;
        }
        
        Ok(provider_config)
    } else {
        Ok(ProviderPoolConfig::default())
    }
}

fn expand_env_var(value: &str) -> String {
    // Expand ${ENV_VAR} format
    if value.starts_with("${") && value.ends_with('}') {
        let var_name = &value[2..value.len()-1];
        std::env::var(var_name).unwrap_or_else(|_| value.to_string())
    } else {
        value.to_string()
    }
}
