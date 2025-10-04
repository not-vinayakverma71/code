use super::types::{CacheKey, CacheValue};
use super::l2_cache::L2Cache;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::collections::HashMap;

pub struct BatchWriter {
    tx: mpsc::UnboundedSender<(CacheKey, CacheValue)>,
}

impl BatchWriter {
    pub fn new(l2: Arc<L2Cache>) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<(CacheKey, CacheValue)>();
        
        // Spawn batch processing task
        tokio::spawn(async move {
            let mut batch = HashMap::new();
            let mut timer = tokio::time::interval(tokio::time::Duration::from_millis(100));
            
            loop {
                tokio::select! {
                    Some((key, value)) = rx.recv() => {
                        batch.insert(key, value);
                        
                        // Write batch if it gets too large
                        if batch.len() >= 100 {
                            for (k, v) in batch.drain() {
                                let _ = l2.put(k, v).await;
                            }
                        }
                    }
                    _ = timer.tick() => {
                        // Flush batch periodically
                        if !batch.is_empty() {
                            for (k, v) in batch.drain() {
                                let _ = l2.put(k, v).await;
                            }
                        }
                    }
                }
            }
        });
        
        Self { tx }
    }
    
    pub fn write(&self, key: CacheKey, value: CacheValue) {
        let _ = self.tx.send((key, value));
    }
}