/// Memory Management (LSP-029)
/// Per-document and global memory limits, idle document eviction, RSS monitoring

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::{Result, anyhow};

/// Memory budget configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum memory per document (10MB default)
    pub max_document_bytes: usize,
    /// Global memory limit (500MB default)
    pub global_limit_bytes: usize,
    /// Idle timeout before eviction (5 minutes default)
    pub idle_timeout_secs: u64,
    /// Enable automatic eviction
    pub enable_auto_eviction: bool,
    /// RSS monitoring interval (30 seconds default)
    pub rss_monitor_interval_secs: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_document_bytes: 10 * 1024 * 1024, // 10MB
            global_limit_bytes: 500 * 1024 * 1024, // 500MB
            idle_timeout_secs: 300, // 5 minutes
            enable_auto_eviction: true,
            rss_monitor_interval_secs: 30,
        }
    }
}

/// Document memory tracker
#[derive(Debug, Clone)]
struct DocumentMemory {
    uri: String,
    size_bytes: usize,
    last_access: Instant,
    access_count: u64,
}

impl DocumentMemory {
    fn new(uri: String, size_bytes: usize) -> Self {
        Self {
            uri,
            size_bytes,
            last_access: Instant::now(),
            access_count: 1,
        }
    }
    
    fn touch(&mut self) {
        self.last_access = Instant::now();
        self.access_count += 1;
    }
    
    fn idle_duration(&self) -> Duration {
        self.last_access.elapsed()
    }
}

/// Memory manager for LSP gateway
pub struct MemoryManager {
    config: MemoryConfig,
    documents: Arc<RwLock<HashMap<String, DocumentMemory>>>,
    total_bytes: Arc<parking_lot::Mutex<usize>>,
}

impl MemoryManager {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            documents: Arc::new(RwLock::new(HashMap::new())),
            total_bytes: Arc::new(parking_lot::Mutex::new(0)),
        }
    }
    
    /// Register document memory usage
    pub fn register_document(&self, uri: &str, size_bytes: usize) -> Result<()> {
        // Check per-document limit
        if size_bytes > self.config.max_document_bytes {
            return Err(anyhow!(
                "Document {} exceeds max size: {} > {}",
                uri,
                size_bytes,
                self.config.max_document_bytes
            ));
        }
        
        let mut docs = self.documents.write();
        let mut total = self.total_bytes.lock();
        
        // Check global limit
        let new_total = *total + size_bytes;
        if new_total > self.config.global_limit_bytes {
            if self.config.enable_auto_eviction {
                // Try to evict idle documents
                drop(docs);
                drop(total);
                self.evict_idle_documents()?;
                
                // Retry after eviction
                docs = self.documents.write();
                total = self.total_bytes.lock();
                let new_total = *total + size_bytes;
                if new_total > self.config.global_limit_bytes {
                    return Err(anyhow!("Global memory limit exceeded after eviction"));
                }
            } else {
                return Err(anyhow!("Global memory limit exceeded"));
            }
        }
        
        // Update or insert
        if let Some(existing) = docs.get_mut(uri) {
            *total = *total - existing.size_bytes + size_bytes;
            existing.size_bytes = size_bytes;
            existing.touch();
        } else {
            *total += size_bytes;
            docs.insert(uri.to_string(), DocumentMemory::new(uri.to_string(), size_bytes));
        }
        
        // Update metrics
        super::LspMetrics::set_memory_usage("documents", *total as i64);
        super::LspMetrics::set_document_count("total", docs.len() as i64);
        
        Ok(())
    }
    
    /// Update document access time
    pub fn touch_document(&self, uri: &str) {
        if let Some(doc) = self.documents.write().get_mut(uri) {
            doc.touch();
        }
    }
    
    /// Remove document from tracking
    pub fn unregister_document(&self, uri: &str) {
        let mut docs = self.documents.write();
        if let Some(doc) = docs.remove(uri) {
            let mut total = self.total_bytes.lock();
            *total = total.saturating_sub(doc.size_bytes);
            
            // Update metrics
            super::LspMetrics::set_memory_usage("documents", *total as i64);
            super::LspMetrics::set_document_count("total", docs.len() as i64);
            
            tracing::debug!(
                uri = %uri,
                size_bytes = doc.size_bytes,
                access_count = doc.access_count,
                "Document unregistered"
            );
        }
    }
    
    /// Evict idle documents
    pub fn evict_idle_documents(&self) -> Result<usize> {
        let idle_timeout = Duration::from_secs(self.config.idle_timeout_secs);
        let mut docs = self.documents.write();
        
        // Find idle documents
        let idle_uris: Vec<String> = docs
            .iter()
            .filter(|(_, doc)| doc.idle_duration() > idle_timeout)
            .map(|(uri, _)| uri.clone())
            .collect();
        
        if idle_uris.is_empty() {
            return Ok(0);
        }
        
        // Evict idle documents
        let mut total = self.total_bytes.lock();
        let mut evicted_count = 0;
        let mut evicted_bytes = 0;
        
        for uri in idle_uris {
            if let Some(doc) = docs.remove(&uri) {
                *total = total.saturating_sub(doc.size_bytes);
                evicted_bytes += doc.size_bytes;
                evicted_count += 1;
                
                tracing::info!(
                    uri = %uri,
                    size_bytes = doc.size_bytes,
                    idle_secs = doc.idle_duration().as_secs(),
                    "Document evicted (idle)"
                );
            }
        }
        
        // Update metrics
        super::LspMetrics::set_memory_usage("documents", *total as i64);
        super::LspMetrics::set_document_count("total", docs.len() as i64);
        
        tracing::info!(
            evicted_count = evicted_count,
            evicted_bytes = evicted_bytes,
            remaining_documents = docs.len(),
            "Idle document eviction complete"
        );
        
        Ok(evicted_count)
    }
    
    /// Get current memory usage
    pub fn current_usage(&self) -> MemoryUsage {
        let docs = self.documents.read();
        let total_bytes = *self.total_bytes.lock();
        
        MemoryUsage {
            total_bytes,
            document_count: docs.len(),
            global_limit_bytes: self.config.global_limit_bytes,
            utilization: (total_bytes as f64 / self.config.global_limit_bytes as f64) * 100.0,
        }
    }
    
    /// Get RSS (Resident Set Size) in bytes
    pub fn get_rss_bytes() -> Result<usize> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let status = fs::read_to_string("/proc/self/status")?;
            
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let kb: usize = parts[1].parse()?;
                        return Ok(kb * 1024);
                    }
                }
            }
            
            Err(anyhow!("Could not find VmRSS in /proc/self/status"))
        }
        
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let output = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()?;
            
            let rss_str = String::from_utf8_lossy(&output.stdout);
            let kb: usize = rss_str.trim().parse()?;
            Ok(kb * 1024)
        }
        
        #[cfg(target_os = "windows")]
        {
            // On Windows, we'd use GetProcessMemoryInfo
            // For now, return an estimate
            Err(anyhow!("RSS monitoring not implemented for Windows"))
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(anyhow!("RSS monitoring not supported on this platform"))
        }
    }
    
    /// Monitor RSS and update metrics
    pub fn monitor_rss(&self) {
        if let Ok(rss_bytes) = Self::get_rss_bytes() {
            super::LspMetrics::set_memory_usage("rss", rss_bytes as i64);
            
            let usage = self.current_usage();
            super::LspMetrics::set_memory_usage("documents", usage.total_bytes as i64);
            
            tracing::debug!(
                rss_mb = rss_bytes / 1024 / 1024,
                documents_mb = usage.total_bytes / 1024 / 1024,
                document_count = usage.document_count,
                utilization = format!("{:.1}%", usage.utilization),
                "Memory status"
            );
        }
    }
    
    /// Start background RSS monitoring task
    pub fn start_rss_monitor(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval_secs = self.config.rss_monitor_interval_secs;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            
            loop {
                interval.tick().await;
                self.monitor_rss();
                
                // Auto-evict if needed
                if self.config.enable_auto_eviction {
                    let usage = self.current_usage();
                    if usage.utilization > 80.0 {
                        tracing::warn!(
                            utilization = format!("{:.1}%", usage.utilization),
                            "High memory utilization, attempting eviction"
                        );
                        
                        if let Err(e) = self.evict_idle_documents() {
                            tracing::error!(error = %e, "Failed to evict idle documents");
                        }
                    }
                }
            }
        })
    }
}

/// Memory usage snapshot
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub total_bytes: usize,
    pub document_count: usize,
    pub global_limit_bytes: usize,
    pub utilization: f64, // Percentage
}

impl MemoryUsage {
    pub fn total_mb(&self) -> f64 {
        self.total_bytes as f64 / 1024.0 / 1024.0
    }
    
    pub fn limit_mb(&self) -> f64 {
        self.global_limit_bytes as f64 / 1024.0 / 1024.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_registration() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        
        manager.register_document("file:///test1.rs", 1000).unwrap();
        manager.register_document("file:///test2.rs", 2000).unwrap();
        
        let usage = manager.current_usage();
        assert_eq!(usage.total_bytes, 3000);
        assert_eq!(usage.document_count, 2);
    }
    
    #[test]
    fn test_document_size_limit() {
        let config = MemoryConfig {
            max_document_bytes: 1000,
            ..Default::default()
        };
        let manager = MemoryManager::new(config);
        
        let result = manager.register_document("file:///large.rs", 2000);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_global_limit() {
        let config = MemoryConfig {
            global_limit_bytes: 5000,
            enable_auto_eviction: false,
            ..Default::default()
        };
        let manager = MemoryManager::new(config);
        
        manager.register_document("file:///test1.rs", 2000).unwrap();
        manager.register_document("file:///test2.rs", 2000).unwrap();
        
        let result = manager.register_document("file:///test3.rs", 2000);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_document_update() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        
        manager.register_document("file:///test.rs", 1000).unwrap();
        manager.register_document("file:///test.rs", 2000).unwrap(); // Update
        
        let usage = manager.current_usage();
        assert_eq!(usage.total_bytes, 2000);
        assert_eq!(usage.document_count, 1);
    }
    
    #[test]
    fn test_document_unregister() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        
        manager.register_document("file:///test.rs", 1000).unwrap();
        manager.unregister_document("file:///test.rs");
        
        let usage = manager.current_usage();
        assert_eq!(usage.total_bytes, 0);
        assert_eq!(usage.document_count, 0);
    }
    
    #[test]
    fn test_idle_eviction() {
        let config = MemoryConfig {
            idle_timeout_secs: 0, // Immediate eviction
            ..Default::default()
        };
        let manager = MemoryManager::new(config);
        
        manager.register_document("file:///test1.rs", 1000).unwrap();
        manager.register_document("file:///test2.rs", 2000).unwrap();
        
        // Sleep to make documents idle
        std::thread::sleep(Duration::from_millis(10));
        
        let evicted = manager.evict_idle_documents().unwrap();
        assert_eq!(evicted, 2);
        
        let usage = manager.current_usage();
        assert_eq!(usage.document_count, 0);
    }
    
    #[test]
    fn test_document_touch() {
        let config = MemoryConfig {
            idle_timeout_secs: 1,
            ..Default::default()
        };
        let manager = MemoryManager::new(config);
        
        manager.register_document("file:///test.rs", 1000).unwrap();
        
        std::thread::sleep(Duration::from_millis(500));
        manager.touch_document("file:///test.rs"); // Refresh access time
        std::thread::sleep(Duration::from_millis(600));
        
        // Should not be evicted because we touched it
        let evicted = manager.evict_idle_documents().unwrap();
        assert_eq!(evicted, 0);
    }
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn test_rss_measurement() {
        let rss = MemoryManager::get_rss_bytes();
        assert!(rss.is_ok(), "RSS measurement should work on Linux/macOS");
        
        let rss_bytes = rss.unwrap();
        assert!(rss_bytes > 0, "RSS should be positive");
        assert!(rss_bytes < 10 * 1024 * 1024 * 1024, "RSS should be reasonable (<10GB)");
    }
}
