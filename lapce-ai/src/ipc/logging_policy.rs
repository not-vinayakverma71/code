/// Structured Logging Policy Implementation
/// PII redaction, sampling on hot paths, rotation compatibility

use tracing::{debug, error, info, trace, warn, Level};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use regex::Regex;
use once_cell::sync::Lazy;

/// PII patterns to redact
static PII_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        (Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(), "[EMAIL]"),
        (Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").unwrap(), "[CARD]"),
        (Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(), "[SSN]"),
        (Regex::new(r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap(), "[PHONE]"),
        (Regex::new(r"\b(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b").unwrap(), "[IP]"),
        (Regex::new(r"Bearer\s+[A-Za-z0-9\-._~+/]+=*").unwrap(), "Bearer [TOKEN]"),
        (Regex::new(r"(?i)(api[_-]?key|apikey|auth[_-]?token|password|passwd|pwd|secret|private[_-]?key)[\s]*[:=][\s]*[^\s]+").unwrap(), "[CREDENTIAL]"),
    ]
});

/// Redact PII from log messages
pub fn redact_pii(message: &str) -> String {
    let mut redacted = message.to_string();
    for (pattern, replacement) in PII_PATTERNS.iter() {
        redacted = pattern.replace_all(&redacted, *replacement).to_string();
    }
    redacted
}

/// Sampling configuration for hot paths
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    /// Sample rate for trace level (1 in N messages)
    pub trace_sample_rate: u64,
    /// Sample rate for debug level
    pub debug_sample_rate: u64,
    /// Sample rate for hot path logs
    pub hot_path_sample_rate: u64,
    /// Maximum logs per second
    pub rate_limit: u64,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            trace_sample_rate: 100,    // Log 1 in 100 trace messages
            debug_sample_rate: 10,     // Log 1 in 10 debug messages
            hot_path_sample_rate: 1000, // Log 1 in 1000 hot path messages
            rate_limit: 10000,          // Max 10k logs/second
        }
    }
}

/// Sampling logger for hot paths
pub struct SamplingLogger {
    config: SamplingConfig,
    trace_counter: AtomicU64,
    debug_counter: AtomicU64,
    hot_path_counter: AtomicU64,
    rate_limiter: Arc<RateLimiter>,
}

impl SamplingLogger {
    pub fn new(config: SamplingConfig) -> Self {
        Self {
            rate_limiter: Arc::new(RateLimiter::new(config.rate_limit)),
            config,
            trace_counter: AtomicU64::new(0),
            debug_counter: AtomicU64::new(0),
            hot_path_counter: AtomicU64::new(0),
        }
    }
    
    /// Log trace with sampling
    pub fn trace_sampled(&self, message: &str) {
        let count = self.trace_counter.fetch_add(1, Ordering::Relaxed);
        if count % self.config.trace_sample_rate == 0 && self.rate_limiter.allow() {
            trace!("{} (sampled 1/{})", redact_pii(message), self.config.trace_sample_rate);
        }
    }
    
    /// Log debug with sampling
    pub fn debug_sampled(&self, message: &str) {
        let count = self.debug_counter.fetch_add(1, Ordering::Relaxed);
        if count % self.config.debug_sample_rate == 0 && self.rate_limiter.allow() {
            debug!("{} (sampled 1/{})", redact_pii(message), self.config.debug_sample_rate);
        }
    }
    
    /// Log hot path with aggressive sampling
    pub fn hot_path(&self, message: &str) {
        let count = self.hot_path_counter.fetch_add(1, Ordering::Relaxed);
        if count % self.config.hot_path_sample_rate == 0 && self.rate_limiter.allow() {
            trace!("HOT_PATH: {} (sampled 1/{})", redact_pii(message), self.config.hot_path_sample_rate);
        }
    }
}

/// Rate limiter for log throttling
struct RateLimiter {
    limit: u64,
    counter: AtomicU64,
    last_reset: AtomicU64,
}

impl RateLimiter {
    fn new(limit: u64) -> Self {
        Self {
            limit,
            counter: AtomicU64::new(0),
            last_reset: AtomicU64::new(0),
        }
    }
    
    fn allow(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let last = self.last_reset.load(Ordering::Relaxed);
        
        // Reset counter every second
        if now > last {
            self.counter.store(0, Ordering::Relaxed);
            self.last_reset.store(now, Ordering::Relaxed);
        }
        
        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        count < self.limit
    }
}

/// Structured log event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: u64,
    pub level: String,
    pub message: String,
    pub module: String,
    pub fields: HashMap<String, serde_json::Value>,
}

/// Initialize structured logging with all policies
pub fn init_logging(config: LogConfig) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));
    
    let fmt_layer = fmt::layer()
        .with_target(config.show_target)
        .with_thread_ids(config.show_thread_ids)
        .with_thread_names(config.show_thread_names)
        .with_file(config.show_file)
        .with_line_number(config.show_line_number)
        .with_span_events(FmtSpan::CLOSE);
    
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);
    
    if config.json_output {
        let json_layer = fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true);
        subscriber.with(json_layer).init();
    } else {
        subscriber.init();
    }
    
    Ok(())
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: String,
    pub json_output: bool,
    pub show_target: bool,
    pub show_thread_ids: bool,
    pub show_thread_names: bool,
    pub show_file: bool,
    pub show_line_number: bool,
    pub rotation_size_mb: u64,
    pub rotation_count: u32,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_output: false,
            show_target: true,
            show_thread_ids: false,
            show_thread_names: false,
            show_file: false,
            show_line_number: false,
            rotation_size_mb: 100,
            rotation_count: 10,
        }
    }
}

/// Structured logging macros with PII redaction
#[macro_export]
macro_rules! log_structured {
    ($level:expr, $msg:expr, $($key:expr => $value:expr),*) => {
        match $level {
            tracing::Level::ERROR => {
                error!(
                    message = $crate::ipc::logging_policy::redact_pii($msg).as_str(),
                    $($key = ?$value,)*
                );
            },
            tracing::Level::WARN => {
                warn!(
                    message = $crate::ipc::logging_policy::redact_pii($msg).as_str(),
                    $($key = ?$value,)*
                );
            },
            tracing::Level::INFO => {
                info!(
                    message = $crate::ipc::logging_policy::redact_pii($msg).as_str(),
                    $($key = ?$value,)*
                );
            },
            tracing::Level::DEBUG => {
                debug!(
                    message = $crate::ipc::logging_policy::redact_pii($msg).as_str(),
                    $($key = ?$value,)*
                );
            },
            tracing::Level::TRACE => {
                trace!(
                    message = $crate::ipc::logging_policy::redact_pii($msg).as_str(),
                    $($key = ?$value,)*
                );
            },
        }
    };
}

/// Log rotation support for file outputs
pub struct RotatingFileLogger {
    base_path: String,
    max_size: u64,
    max_files: u32,
    current_size: AtomicU64,
    current_file: std::sync::Mutex<Option<std::fs::File>>,
}

impl RotatingFileLogger {
    pub fn new(base_path: String, max_size_mb: u64, max_files: u32) -> Self {
        Self {
            base_path,
            max_size: max_size_mb * 1024 * 1024,
            max_files,
            current_size: AtomicU64::new(0),
            current_file: std::sync::Mutex::new(None),
        }
    }
    
    pub fn write(&self, message: &str) -> anyhow::Result<()> {
        use std::io::Write;
        
        let message_size = message.len() as u64;
        let current = self.current_size.fetch_add(message_size, Ordering::Relaxed);
        
        // Check if rotation needed
        if current + message_size > self.max_size {
            self.rotate()?;
        }
        
        // Write message
        let mut file_guard = self.current_file.lock().unwrap();
        if file_guard.is_none() {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.base_path)?;
            *file_guard = Some(file);
        }
        
        if let Some(ref mut file) = *file_guard {
            writeln!(file, "{}", redact_pii(message))?;
            file.flush()?;
        }
        
        Ok(())
    }
    
    fn rotate(&self) -> anyhow::Result<()> {
        // Close current file
        {
            let mut file_guard = self.current_file.lock().unwrap();
            *file_guard = None;
        }
        
        // Rotate files
        for i in (1..self.max_files).rev() {
            let old_path = format!("{}.{}", self.base_path, i);
            let new_path = format!("{}.{}", self.base_path, i + 1);
            if std::path::Path::new(&old_path).exists() {
                std::fs::rename(&old_path, &new_path)?;
            }
        }
        
        // Move current to .1
        if std::path::Path::new(&self.base_path).exists() {
            std::fs::rename(&self.base_path, format!("{}.1", self.base_path))?;
        }
        
        // Reset size counter
        self.current_size.store(0, Ordering::Relaxed);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pii_redaction() {
        let tests = vec![
            ("user@example.com", "[EMAIL]"),
            ("Call me at 555-123-4567", "Call me at [PHONE]"),
            ("My SSN is 123-45-6789", "My SSN is [SSN]"),
            ("Card: 4111-1111-1111-1111", "Card: [CARD]"),
            ("IP: 192.168.1.1", "IP: [IP]"),
            ("Bearer abc123xyz", "Bearer [TOKEN]"),
            ("api_key=secret123", "[CREDENTIAL]"),
        ];
        
        for (input, expected_pattern) in tests {
            let redacted = redact_pii(input);
            assert!(
                redacted.contains(expected_pattern),
                "Failed to redact '{}': got '{}'",
                input,
                redacted
            );
        }
    }
    
    #[test]
    fn test_sampling() {
        let config = SamplingConfig {
            trace_sample_rate: 2,
            debug_sample_rate: 2,
            hot_path_sample_rate: 2,
            rate_limit: 1000,
        };
        
        let logger = SamplingLogger::new(config);
        
        // First call should not log (count = 0)
        // Second call should log (count = 1, 1 % 2 == 1)
        // Third call should not log (count = 2, 2 % 2 == 0)
        // Fourth call should log (count = 3, 3 % 2 == 1)
        
        for i in 0..10 {
            logger.trace_sampled(&format!("Test message {}", i));
        }
    }
    
    #[test]
    fn test_rate_limiting() {
        let limiter = RateLimiter::new(5);
        
        // Should allow first 5
        for _ in 0..5 {
            assert!(limiter.allow());
        }
        
        // Should deny after limit
        assert!(!limiter.allow());
    }
}
