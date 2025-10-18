/// Streaming Updates (LSP-031)
/// LspProgress for long operations, chunked diagnostics for large changes

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::Mutex;
use anyhow::Result;
use tokio::sync::mpsc;

/// Progress token for tracking long operations
#[derive(Debug, Clone)]
pub struct ProgressToken {
    token: String,
    current: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
}

impl ProgressToken {
    pub fn new(token: String, total: u64) -> Self {
        Self {
            token,
            current: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(total)),
        }
    }
    
    pub fn token(&self) -> &str {
        &self.token
    }
    
    pub fn current(&self) -> u64 {
        self.current.load(Ordering::Relaxed)
    }
    
    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }
    
    pub fn percentage(&self) -> u8 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        let current = self.current();
        ((current as f64 / total as f64) * 100.0).min(100.0) as u8
    }
    
    pub fn increment(&self) {
        self.current.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn set(&self, current: u64) {
        self.current.store(current, Ordering::Relaxed);
    }
}

/// Progress kind for LSP progress notifications
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressKind {
    Begin,
    Report,
    End,
}

impl ProgressKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProgressKind::Begin => "begin",
            ProgressKind::Report => "report",
            ProgressKind::End => "end",
        }
    }
}

/// Progress notification builder
pub struct ProgressNotification {
    token: String,
    kind: ProgressKind,
    title: Option<String>,
    message: Option<String>,
    percentage: Option<u8>,
}

impl ProgressNotification {
    pub fn begin(token: String, title: String) -> Self {
        Self {
            token,
            kind: ProgressKind::Begin,
            title: Some(title),
            message: None,
            percentage: None,
        }
    }
    
    pub fn report(token: String) -> Self {
        Self {
            token,
            kind: ProgressKind::Report,
            title: None,
            message: None,
            percentage: None,
        }
    }
    
    pub fn end(token: String) -> Self {
        Self {
            token,
            kind: ProgressKind::End,
            title: None,
            message: None,
            percentage: None,
        }
    }
    
    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
    
    pub fn with_percentage(mut self, percentage: u8) -> Self {
        self.percentage = Some(percentage.min(100));
        self
    }
    
    /// Convert to JSON string for LspProgressPayload
    pub fn to_json(&self) -> String {
        let mut parts = vec![
            format!(r#""kind":"{}""#, self.kind.as_str()),
        ];
        
        if let Some(ref title) = self.title {
            parts.push(format!(r#""title":"{}""#, Self::escape_json(title)));
        }
        
        if let Some(ref message) = self.message {
            parts.push(format!(r#""message":"{}""#, Self::escape_json(message)));
        }
        
        if let Some(percentage) = self.percentage {
            parts.push(format!(r#""percentage":{}"#, percentage));
        }
        
        format!("{{{}}}", parts.join(","))
    }
    
    pub fn token(&self) -> &str {
        &self.token
    }
    
    fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
}

/// Progress reporter for long operations
pub struct ProgressReporter {
    token: ProgressToken,
    sender: mpsc::UnboundedSender<ProgressNotification>,
    report_interval: u64, // Report every N items
}

impl ProgressReporter {
    pub fn new(
        token: ProgressToken,
        sender: mpsc::UnboundedSender<ProgressNotification>,
        report_interval: u64,
    ) -> Self {
        Self {
            token,
            sender,
            report_interval: report_interval.max(1),
        }
    }
    
    /// Start progress tracking
    pub fn begin(&self, title: String) -> Result<()> {
        let notification = ProgressNotification::begin(self.token.token().to_string(), title);
        self.sender.send(notification)?;
        
        tracing::info!(
            token = %self.token.token(),
            total = self.token.total(),
            "Progress started"
        );
        
        Ok(())
    }
    
    /// Report progress increment
    pub fn increment(&self) -> Result<()> {
        self.token.increment();
        let current = self.token.current();
        
        // Report only at intervals to avoid flooding
        if current % self.report_interval == 0 || current == self.token.total() {
            self.report_current()?;
        }
        
        Ok(())
    }
    
    /// Report specific progress
    pub fn set(&self, current: u64) -> Result<()> {
        self.token.set(current);
        self.report_current()?;
        Ok(())
    }
    
    /// Report current progress
    fn report_current(&self) -> Result<()> {
        let percentage = self.token.percentage();
        let message = format!("{}/{}", self.token.current(), self.token.total());
        
        let notification = ProgressNotification::report(self.token.token().to_string())
            .with_message(message)
            .with_percentage(percentage);
        
        self.sender.send(notification)?;
        
        tracing::debug!(
            token = %self.token.token(),
            current = self.token.current(),
            total = self.token.total(),
            percentage = percentage,
            "Progress reported"
        );
        
        Ok(())
    }
    
    /// End progress tracking
    pub fn end(&self, message: Option<String>) -> Result<()> {
        let mut notification = ProgressNotification::end(self.token.token().to_string());
        
        if let Some(msg) = message {
            notification = notification.with_message(msg);
        }
        
        self.sender.send(notification)?;
        
        tracing::info!(
            token = %self.token.token(),
            total = self.token.total(),
            "Progress completed"
        );
        
        Ok(())
    }
}

/// Diagnostic chunk for streaming large diagnostic sets
#[derive(Debug, Clone)]
pub struct DiagnosticChunk {
    pub uri: String,
    pub version: Option<u32>,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub diagnostics_json: String,
}

impl DiagnosticChunk {
    /// Check if this is the final chunk
    pub fn is_final(&self) -> bool {
        self.chunk_index + 1 == self.total_chunks
    }
}

/// Chunked diagnostics builder
pub struct DiagnosticsChunker {
    max_chunk_size: usize, // Max diagnostics per chunk
}

impl DiagnosticsChunker {
    pub fn new(max_chunk_size: usize) -> Self {
        Self {
            max_chunk_size: max_chunk_size.max(1),
        }
    }
    
    /// Split diagnostics JSON array into chunks
    pub fn chunk_diagnostics(
        &self,
        uri: String,
        version: Option<u32>,
        diagnostics_json: &str,
    ) -> Result<Vec<DiagnosticChunk>> {
        // Parse the JSON array to count diagnostics
        let diagnostics: Vec<serde_json::Value> = serde_json::from_str(diagnostics_json)?;
        
        if diagnostics.is_empty() {
            // Empty diagnostics - single chunk
            return Ok(vec![DiagnosticChunk {
                uri,
                version,
                chunk_index: 0,
                total_chunks: 1,
                diagnostics_json: "[]".to_string(),
            }]);
        }
        
        // Calculate number of chunks
        let total_chunks = (diagnostics.len() + self.max_chunk_size - 1) / self.max_chunk_size;
        
        let mut chunks = Vec::with_capacity(total_chunks);
        
        for (chunk_index, chunk_diagnostics) in diagnostics.chunks(self.max_chunk_size).enumerate() {
            let chunk_json = serde_json::to_string(chunk_diagnostics)?;
            
            chunks.push(DiagnosticChunk {
                uri: uri.clone(),
                version,
                chunk_index,
                total_chunks,
                diagnostics_json: chunk_json,
            });
        }
        
        tracing::debug!(
            uri = %uri,
            total_diagnostics = diagnostics.len(),
            total_chunks = total_chunks,
            "Diagnostics chunked"
        );
        
        Ok(chunks)
    }
}

impl Default for DiagnosticsChunker {
    fn default() -> Self {
        Self::new(100) // 100 diagnostics per chunk by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_progress_token() {
        let token = ProgressToken::new("test-token".to_string(), 100);
        
        assert_eq!(token.current(), 0);
        assert_eq!(token.total(), 100);
        assert_eq!(token.percentage(), 0);
        
        token.increment();
        assert_eq!(token.current(), 1);
        assert_eq!(token.percentage(), 1);
        
        token.set(50);
        assert_eq!(token.current(), 50);
        assert_eq!(token.percentage(), 50);
        
        token.set(100);
        assert_eq!(token.percentage(), 100);
    }
    
    #[test]
    fn test_progress_notification_json() {
        let notification = ProgressNotification::begin(
            "token-1".to_string(),
            "Indexing workspace".to_string(),
        );
        
        let json = notification.to_json();
        assert!(json.contains(r#""kind":"begin""#));
        assert!(json.contains(r#""title":"Indexing workspace""#));
        
        let notification = ProgressNotification::report("token-1".to_string())
            .with_message("50/100".to_string())
            .with_percentage(50);
        
        let json = notification.to_json();
        assert!(json.contains(r#""kind":"report""#));
        assert!(json.contains(r#""message":"50/100""#));
        assert!(json.contains(r#""percentage":50"#));
    }
    
    #[tokio::test]
    async fn test_progress_reporter() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let token = ProgressToken::new("test".to_string(), 10);
        let reporter = ProgressReporter::new(token, tx, 2);
        
        reporter.begin("Test operation".to_string()).unwrap();
        
        // Should receive begin notification
        let notification = rx.recv().await.unwrap();
        assert_eq!(notification.kind, ProgressKind::Begin);
        
        // Increment - should report at interval (every 2)
        reporter.increment().unwrap();
        assert!(rx.try_recv().is_err()); // No report yet
        
        reporter.increment().unwrap();
        let notification = rx.recv().await.unwrap(); // Report at 2
        assert_eq!(notification.kind, ProgressKind::Report);
        
        reporter.end(Some("Completed".to_string())).unwrap();
        let notification = rx.recv().await.unwrap();
        assert_eq!(notification.kind, ProgressKind::End);
    }
    
    #[test]
    fn test_diagnostics_chunker_empty() {
        let chunker = DiagnosticsChunker::new(10);
        let chunks = chunker.chunk_diagnostics(
            "file:///test.rs".to_string(),
            Some(1),
            "[]",
        ).unwrap();
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].total_chunks, 1);
        assert_eq!(chunks[0].diagnostics_json, "[]");
    }
    
    #[test]
    fn test_diagnostics_chunker_single_chunk() {
        let chunker = DiagnosticsChunker::new(10);
        
        let diagnostics = vec![
            serde_json::json!({"message": "error 1"}),
            serde_json::json!({"message": "error 2"}),
        ];
        let diagnostics_json = serde_json::to_string(&diagnostics).unwrap();
        
        let chunks = chunker.chunk_diagnostics(
            "file:///test.rs".to_string(),
            Some(1),
            &diagnostics_json,
        ).unwrap();
        
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_final());
    }
    
    #[test]
    fn test_diagnostics_chunker_multiple_chunks() {
        let chunker = DiagnosticsChunker::new(3);
        
        let diagnostics: Vec<serde_json::Value> = (0..10)
            .map(|i| serde_json::json!({"message": format!("error {}", i)}))
            .collect();
        let diagnostics_json = serde_json::to_string(&diagnostics).unwrap();
        
        let chunks = chunker.chunk_diagnostics(
            "file:///test.rs".to_string(),
            Some(1),
            &diagnostics_json,
        ).unwrap();
        
        assert_eq!(chunks.len(), 4); // 10 diagnostics / 3 per chunk = 4 chunks
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].total_chunks, 4);
        assert!(!chunks[0].is_final());
        assert!(chunks[3].is_final());
    }
    
    #[test]
    fn test_json_escaping() {
        let notification = ProgressNotification::begin(
            "token".to_string(),
            "Test \"quotes\" and\nnewlines".to_string(),
        );
        
        let json = notification.to_json();
        assert!(json.contains(r#"\"quotes\""#));
        assert!(json.contains(r#"\n"#));
    }
}
