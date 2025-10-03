/// Main production server binary for lapce-ai-rust

use anyhow::{Result, Context};
use clap::Parser;
use tokio::signal;
use tracing::{info, warn, error};
use std::sync::Arc;
use std::path::Path;

use lapce_ai_rust::{
    ipc_server_complete::IpcServerComplete,
    provider_pool::{ProviderPool, ProviderPoolConfig},
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
    info!("Rust version: {}", env!("CARGO_PKG_RUST_VERSION"));
    info!("Build target: {}", env!("TARGET"));
    
    // Load configuration
    let config_path = Path::new(&args.config);
    if !config_path.exists() {
        warn!("Config file not found at {}, using defaults", args.config);
    let mut config = if std::path::Path::new(&args.config).exists() {
        info!("Loading config from: {}", args.config);
        IpcConfig::from_file(&args.config)?
    } else {
        info!("Using default configuration");
        IpcConfig::default()
    };
    
    // Apply command-line overrides
    if let Some(socket) = args.socket {
        config.socket_path = socket;
    }
    if let Some(port) = args.metrics_port {
        config.metrics_port = port;
    }
    
    // Validate configuration
    config.validate()?;
    info!("Configuration validated successfully");
    
    if args.validate {
        println!("Configuration is valid");
        return Ok(());
    }
    
    // Load provider configuration
    let provider_config = load_provider_config(&args.config).await?;
    
    // Initialize provider pool
    let provider_pool = Arc::new(ProviderPool::new(provider_config).await?);
    info!("Provider pool initialized with {} providers", provider_pool.get_stats().await.len());
    
    // Create IPC server
    let mut server = IpcServerComplete::new(config.clone()).await?;
    server.set_provider_pool(provider_pool.clone());
    
    info!("Server configuration:");
    info!("  Socket path: {}", config.socket_path);
    info!("  Max connections: {}", config.max_connections);
    info!("  Metrics port: {}", config.metrics_port);
    info!("  Compression: {}", config.enable_compression);
    info!("  TLS: {}", config.enable_tls);
    
    // Setup signal handlers
    let server_clone = server.clone();
    tokio::spawn(async move {
        use tokio::signal;
        
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
        
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down gracefully");
            }
        }
        
        server_clone.shutdown.send(()).ok();
        Ok::<_, anyhow::Error>(())
    });
    
    // Notify systemd that we're ready
    #[cfg(target_os = "linux")]
    {
        if let Ok(true) = sd_notify::booted() {
            sd_notify::notify(true, &[sd_notify::NotifyState::Ready])?;
            info!("Notified systemd: ready");
        }
    }
    
    // Start server
    info!("Server starting on {}", config.socket_path);
    info!("Metrics available at http://0.0.0.0:{}/metrics", config.metrics_port);
    info!("Health check at http://0.0.0.0:{}/health", config.metrics_port);
    
    match server.serve().await {
        Ok(_) => {
            info!("Server shut down cleanly");
        }
        Err(e) => {
            error!("Server error: {}", e);
            return Err(e.into());
        }
    }
    
    // Cleanup
    provider_pool.shutdown().await;
    info!("Provider pool shut down");
    
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
                            .unwrap_or("")
                            .to_string(),
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
