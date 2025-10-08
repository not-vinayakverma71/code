use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use std::io;

/// Initialize tracing with JSON output for production use
pub fn init_json_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(io::stderr);
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Initialize tracing with pretty output for development
pub fn init_pretty_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("debug"));
    
    let fmt_layer = fmt::layer()
        .pretty()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Initialize logging based on environment
pub fn init() {
    if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        init_json_logging();
    } else if cfg!(debug_assertions) {
        init_pretty_logging();
    } else {
        init_json_logging();
    }
}

/// Log structured data with consistent fields
#[macro_export]
macro_rules! log_operation {
    ($level:expr, $op:expr, $($field:tt)*) => {
        match $level {
            "error" => tracing::error!(op = $op, $($field)*),
            "warn" => tracing::warn!(op = $op, $($field)*),
            "info" => tracing::info!(op = $op, $($field)*),
            "debug" => tracing::debug!(op = $op, $($field)*),
            "trace" => tracing::trace!(op = $op, $($field)*),
            _ => tracing::info!(op = $op, $($field)*),
        }
    };
}

/// Log cache operations with standard fields
#[macro_export]
macro_rules! log_cache {
    ($op:expr, hit:$hit:expr, lang:$lang:expr, bytes:$bytes:expr, time_ms:$time:expr) => {
        tracing::info!(
            op = $op,
            cache_hit = $hit,
            language = $lang,
            bytes = $bytes,
            time_ms = $time,
            "Cache operation"
        );
    };
}

/// Log parsing operations
#[macro_export]
macro_rules! log_parse {
    ($lang:expr, $bytes:expr, $time_ms:expr, $nodes:expr) => {
        tracing::info!(
            op = "parse",
            language = $lang,
            bytes = $bytes,
            time_ms = $time_ms,
            node_count = $nodes,
            "Parse operation"
        );
    };
}
