// Day 20: Distributed Search with Redis
use redis::{Client, Connection, Commands, RedisResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedIndex {
    pub node_id: String,
    pub shard_count: usize,
    pub replicas: Vec<String>,
}

pub struct DistributedSearch {
    redis_client: Client,
    node_id: String,
    shards: Arc<RwLock<HashMap<usize, ShardInfo>>>,
}

#[derive(Debug, Clone)]
struct ShardInfo {
    id: usize,
    node: String,
    doc_count: usize,
    size_bytes: usize,
}

impl DistributedSearch {
    pub fn new(redis_url: &str, node_id: String) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        
        Ok(Self {
            redis_client: client,
            node_id,
            shards: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    pub async fn index_document(&self, doc_id: &str, content: &str, embedding: Vec<f32>) -> RedisResult<()> {
        let mut conn = self.redis_client.get_connection()?;
        
        // Calculate shard
        let shard_id = self.calculate_shard(doc_id).await;
        
        // Store document
        let doc_key = format!("doc:{}:{}", shard_id, doc_id);
        conn.set(&doc_key, content)?;
        
        // Store embedding
        let emb_key = format!("emb:{}:{}", shard_id, doc_id);
        let emb_bytes = bincode::serialize(&embedding).unwrap();
        conn.set(&emb_key, &emb_bytes[..])?;
        
        // Update index
        conn.sadd(format!("idx:{}", shard_id), doc_id)?;
        
        // Update stats
        conn.incr(format!("stats:{}:docs", self.node_id), 1)?;
        
        Ok(())
    }
    
    pub async fn search(&self, query_embedding: Vec<f32>, top_k: usize) -> Vec<SearchResult> {
        let mut conn = self.redis_client.get_connection().unwrap();
        let mut all_results = Vec::new();
        
        // Search all shards
        let shards = self.shards.read().await;
        for shard_id in 0..10 {  // Assume 10 shards
            // Get all docs in shard
            let docs: Vec<String> = conn.smembers(format!("idx:{}", shard_id)).unwrap_or_default();
            
            for doc_id in docs {
                // Get embedding
                let emb_key = format!("emb:{}:{}", shard_id, doc_id);
                if let Ok(emb_data) = conn.get::<_, Vec<u8>>(&emb_key) {
                    if let Ok(embedding) = bincode::deserialize::<Vec<f32>>(&emb_data) {
                        let score = cosine_similarity(&query_embedding, &embedding);
                        
                        // Get content
                        let doc_key = format!("doc:{}:{}", shard_id, doc_id);
                        if let Ok(doc_content) = conn.get::<_, String>(&doc_key) {
                            all_results.push(SearchResult {
                                doc_id: doc_id.clone(),
                                content: doc_content,
                                score,
                                shard_id,
                            });
                        }
                    }
                }
            }
        }
        
        // Sort and truncate
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        all_results.truncate(top_k);
        all_results
    }
    
    async fn calculate_shard(&self, doc_id: &str) -> usize {
        // Simple hash-based sharding
        let hash = doc_id.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        (hash % 10) as usize
    }
    
    pub async fn rebalance_shards(&self) -> RedisResult<()> {
        let mut conn = self.redis_client.get_connection()?;
        
        // Get all nodes
        let nodes: Vec<String> = conn.smembers("cluster:nodes")?;
        
        // Calculate new shard distribution
        let total_shards = 100;
        let shards_per_node = total_shards / nodes.len();
        
        // Assign shards to nodes
        for (i, node) in nodes.iter().enumerate() {
            let start = i * shards_per_node;
            let end = ((i + 1) * shards_per_node).min(total_shards);
            
            for shard in start..end {
                conn.hset("cluster:shards", shard, node)?;
            }
        }
        
        Ok(())
    }
    
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        let mut conn = self.redis_client.get_connection().unwrap();
        
        ClusterStats {
            total_docs: conn.get(format!("stats:{}:docs", self.node_id)).unwrap_or(0),
            total_shards: 10,
            node_count: conn.scard("cluster:nodes").unwrap_or(1),
            index_size_mb: 0.0, // Redis info parsing not available
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: String,
    pub content: String,
    pub score: f32,
    pub shard_id: usize,
}

#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub total_docs: usize,
    pub total_shards: usize,
    pub node_count: usize,
    pub index_size_mb: f64,
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
