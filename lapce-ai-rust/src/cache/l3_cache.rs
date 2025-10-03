/// L3Cache - EXACT implementation from docs lines 63-66
use std::sync::Arc;
use anyhow::Result;

use super::{
    cache_metrics::CacheMetrics,
    serializer::{Serializer as CacheSerializer, SerializationFormat},
    types::{CacheKey, CacheValue, CacheLevel},
};

use redis::AsyncCommands;

pub struct L3Cache {
    pub redis: redis::aio::MultiplexedConnection,
    pub serializer: Arc<CacheSerializer>,
    pub metrics: Arc<CacheMetrics>,
}

impl L3Cache {
    pub async fn new(redis_url: &str, metrics: Arc<CacheMetrics>) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis = client.get_multiplexed_async_connection().await?;
        
        Ok(Self {
            redis,
            serializer: Arc::new(CacheSerializer::new(SerializationFormat::Bincode)),
            metrics,
        })
    }
    
    pub async fn get(&mut self, key: &CacheKey) -> Result<Option<CacheValue>> {
        let key_str = format!("cache:{}", key.0);
        
        match self.redis.get::<_, Option<Vec<u8>>>(&key_str).await {
            Ok(Some(data)) => {
                let value: CacheValue = bincode::deserialize(&data)?;
                self.metrics.record_l3_hit();
                Ok(Some(value))
            }
            Ok(None) => {
                self.metrics.record_l3_miss();
                Ok(None)
            }
            Err(e) => {
                self.metrics.record_l3_miss();
                Err(e.into())
            }
        }
    }
    
    pub async fn put(&mut self, key: &CacheKey, value: CacheValue) -> Result<()> {
        let key_str = format!("cache:{}", key.0);
        let data = bincode::serialize(&value)?;
        let ttl = 3600; // 1 hour TTL
        self.redis.set_ex(&key_str, data, ttl).await?;
        Ok(())
    }
}

// Remove duplicate Serializer definition - it's imported from serializer module
