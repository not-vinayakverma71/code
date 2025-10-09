// Periodic Index Compaction - SEM-009-B
use crate::error::Result;
use crate::search::semantic_search_engine::SemanticSearchEngine;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::interval;
use tracing::{info, warn, instrument};

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
        let mut ticker = interval(Duration::from_secs(self.interval_secs));
        
        loop {
            ticker.tick().await;
            
            // Acquire semaphore for backpressure
            let permit = match self.backpressure.try_acquire() {
                Ok(permit) => permit,
                Err(_) => {
                    warn!("Skipping compaction - previous compaction still running");
                    continue;
                }
            };
            
            info!("Starting periodic index compaction");
            
            match self.engine.optimize_index().await {
                Ok(()) => info!("Index compaction completed successfully"),
                Err(e) => warn!("Index compaction failed: {}", e),
            }
            
            drop(permit);
        }
    }
    
    /// Trigger manual compaction
    pub async fn compact_now(&self) -> Result<()> {
        let _permit = self.backpressure.acquire().await?;
        self.engine.optimize_index().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_compaction_service_creation() {
        std::env::set_var("INDEX_COMPACTION_INTERVAL", "60");
        
        let config = crate::search::semantic_search_engine::SearchConfig {
            db_path: std::path::PathBuf::from("./test_compaction"),
            max_embedding_dim: Some(1536),
            index_params: Default::default(),
        };
        
        let engine = Arc::new(
            crate::search::semantic_search_engine::SemanticSearchEngine::new(config)
                .await
                .unwrap()
        );
        
        let service = IndexCompactionService::new(engine);
        assert_eq!(service.interval_secs, 60);
    }
}
