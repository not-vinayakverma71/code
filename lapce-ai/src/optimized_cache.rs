/// Day 5: Optimized Cache with better write performance
use std::sync::Arc;
use dashmap::DashMap;
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::sync::RwLock;
use std::collections::VecDeque;

pub struct OptimizedCache {
    // L1: Hot data in memory
    l1: Arc<DashMap<String, CacheEntry>>,
    // Write buffer for batching
    write_buffer: Arc<RwLock<WriteBuffer>>,
    // Stats
    stats: Arc<RwLock<CacheStats>>,
}

struct CacheEntry {
    data: Vec<u8>,
    last_access: Instant,
    access_count: u32,
}

struct WriteBuffer {
    pending: VecDeque<(String, Vec<u8>)>,
    last_flush: Instant,
}

#[derive(Default)]
struct CacheStats {
    writes: u64,
    reads: u64,
    hits: u64,
    misses: u64,
}

impl OptimizedCache {
    pub fn new() -> Self {
        let cache = Self {
            l1: Arc::new(DashMap::with_capacity(10_000)),
            write_buffer: Arc::new(RwLock::new(WriteBuffer {
                pending: VecDeque::with_capacity(1000),
                last_flush: Instant::now(),
            })),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        };
        
        // Start background flusher
        let write_buffer = cache.write_buffer.clone();
        let l1 = cache.l1.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                let mut buffer = write_buffer.write().await;
                if buffer.pending.is_empty() {
                    continue;
                }
                
                // Batch flush
                let batch_size = buffer.pending.len().min(100);
                for _ in 0..batch_size {
                    if let Some((key, data)) = buffer.pending.pop_front() {
                        l1.insert(key, CacheEntry {
                            data,
                            last_access: Instant::now(),
                            access_count: 1,
                        });
                    }
                }
                buffer.last_flush = Instant::now();
            }
        });
        
        cache
    }
    
    pub async fn set(&self, key: String, value: Vec<u8>) -> Result<()> {
        // Fast path - add to write buffer
        let mut buffer = self.write_buffer.write().await;
        buffer.pending.push_back((key, value));
        
        self.stats.write().await.writes += 1;
        
        // Immediate flush if buffer is large
        if buffer.pending.len() > 1000 {
            drop(buffer); // Release lock
            self.flush_writes().await;
        }
        
        Ok(())
    }
    
    pub async fn set_immediate(&self, key: String, value: Vec<u8>) -> Result<()> {
        // Direct write for critical data
        self.l1.insert(key, CacheEntry {
            data: value,
            last_access: Instant::now(),
            access_count: 1,
        });
        
        self.stats.write().await.writes += 1;
        Ok(())
    }
    
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut stats = self.stats.write().await;
        stats.reads += 1;
        
        // Check L1
        if let Some(mut entry) = self.l1.get_mut(key) {
            entry.last_access = Instant::now();
            entry.access_count += 1;
            stats.hits += 1;
            return Some(entry.data.clone());
        }
        
        // Check write buffer
        let buffer = self.write_buffer.read().await;
        for (k, v) in buffer.pending.iter().rev() {
            if k == key {
                stats.hits += 1;
                return Some(v.clone());
            }
        }
        
        stats.misses += 1;
        None
    }
    
    async fn flush_writes(&self) {
        let mut buffer = self.write_buffer.write().await;
        
        while let Some((key, data)) = buffer.pending.pop_front() {
            self.l1.insert(key, CacheEntry {
                data,
                last_access: Instant::now(),
                access_count: 1,
            });
        }
        
        buffer.last_flush = Instant::now();
    }
    
    pub async fn stats(&self) -> (u64, u64, f64) {
        let stats = self.stats.read().await;
        let hit_rate = if stats.reads > 0 {
            (stats.hits as f64 / stats.reads as f64) * 100.0
        } else {
            0.0
        };
        (stats.writes, stats.reads, hit_rate)
    }
    
    pub fn clear(&self) {
        self.l1.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_write_performance() {
        let cache = OptimizedCache::new();
        
        let iterations = 100_000;
        let start = Instant::now();
        
        for i in 0..iterations {
            cache.set(format!("key_{}", i), vec![i as u8; 64]).await.unwrap();
        }
        
        let elapsed = start.elapsed();
        let writes_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("Optimized cache: {:.0} writes/sec", writes_per_sec);
        assert!(writes_per_sec > 500_000.0, "Write performance too low");
    }
    
    #[tokio::test]
    async fn test_read_after_write() {
        let cache = OptimizedCache::new();
        
        // Write some data
        for i in 0..100 {
            cache.set(format!("key_{}", i), vec![i as u8; 32]).await.unwrap();
        }
        
        // Allow buffer to flush
        tokio::time::sleep(Duration::from_millis(20)).await;
        
        // Read back
        for i in 0..100 {
            let value = cache.get(&format!("key_{}", i)).await;
            assert!(value.is_some());
            assert_eq!(value.unwrap()[0], i as u8);
        }
    }
}
