// Periodic Index Compaction - SEM-009-B
use crate::error::Result;
use crate::search::semantic_search_engine::SemanticSearchEngine;
use crate::search::search_metrics::INDEX_OPERATIONS_TOTAL;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::interval;
use tracing::{info, warn, error, instrument};

/// Periodic index compaction service
pub struct IndexCompactionService {
    engine: Arc<SemanticSearchEngine>,
    interval_secs: u64,
    backpressure: Arc<Semaphore>,
}

impl IndexCompactionService {
    /// Create new compaction service
    pub fn new(engine: Arc<SemanticSearchEngine>) -> Self {
        let interval_secs = std::env::var("INDEX_COMPACTION_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600); // Default: 1 hour
        
        Self {
            engine,
            interval_secs,
            backpressure: Arc::new(Semaphore::new(1)), // Only one compaction at a time
        }
    }
    
    /// Start periodic compaction
    #[instrument(skip(self))]
    pub async fn start(self: Arc<Self>) {
        info!("Starting periodic index compaction service (interval: {}s)", self.interval_secs);
        let mut ticker = interval(Duration::from_secs(self.interval_secs));
        
        loop {
            ticker.tick().await;
            
            // Acquire semaphore for backpressure
            let permit = match self.backpressure.try_acquire() {
                Ok(permit) => permit,
                Err(_) => {
                    warn!("Skipping compaction - previous compaction still running");
                    INDEX_OPERATIONS_TOTAL.with_label_values(&["compaction_skipped"]).inc();
                    continue;
                }
            };
            
            info!("Starting periodic index compaction");
            let start = Instant::now();
            
            match self.engine.optimize_index().await {
                Ok(()) => {
                    let duration = start.elapsed();
                    info!("Index compaction completed successfully in {:?}", duration);
                    INDEX_OPERATIONS_TOTAL.with_label_values(&["compaction_success"]).inc();
                }
                Err(e) => {
                    error!("Index compaction failed: {}", e);
                    INDEX_OPERATIONS_TOTAL.with_label_values(&["compaction_error"]).inc();
                }
            }
            
            drop(permit);
        }
    }
    
    /// Trigger manual compaction
    pub async fn compact_now(&self) -> Result<()> {
        let _permit = self.backpressure.acquire().await
            .map_err(|e| crate::error::Error::Runtime {
                message: format!("Failed to acquire compaction semaphore: {}", e)
            })?;
        
        let start = Instant::now();
        let result = self.engine.optimize_index().await;
        
        if result.is_ok() {
            let duration = start.elapsed();
            info!("Manual compaction completed in {:?}", duration);
            INDEX_OPERATIONS_TOTAL.with_label_values(&["manual_compaction_success"]).inc();
        } else {
            INDEX_OPERATIONS_TOTAL.with_label_values(&["manual_compaction_error"]).inc();
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_compaction_service_creation() {
        std::env::set_var("INDEX_COMPACTION_INTERVAL", "60");
        
        let config = crate::search::semantic_search_engine::SearchConfig {
            db_path: "./test_compaction".to_string(),
            max_embedding_dim: Some(1536),
            ..Default::default()
        };
        
        // Create a mock embedder for testing
        use crate::embeddings::embedder_interface::{IEmbedder, EmbeddingResponse, EmbedderInfo, AvailableEmbedders};
        struct MockEmbedder;
        #[async_trait::async_trait]
        impl IEmbedder for MockEmbedder {
            async fn create_embeddings(
                &self,
                texts: Vec<String>,
                _model: Option<&str>
            ) -> crate::error::Result<EmbeddingResponse> {
                Ok(EmbeddingResponse {
                    embeddings: vec![vec![0.0; 1536]; texts.len()],
                    usage: None,
                })
            }
            async fn validate_configuration(&self) -> crate::error::Result<(bool, Option<String>)> {
                Ok((true, None))
            }
            fn embedder_info(&self) -> EmbedderInfo {
                EmbedderInfo {
                    name: AvailableEmbedders::OpenAi,
                }
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
        let embedder = Arc::new(MockEmbedder) as Arc<dyn IEmbedder>;
        
        let engine = Arc::new(
            crate::search::semantic_search_engine::SemanticSearchEngine::new(config, embedder)
                .await
                .unwrap()
        );
        
        let service = IndexCompactionService::new(engine);
        assert_eq!(service.interval_secs, 60);
    }
}
