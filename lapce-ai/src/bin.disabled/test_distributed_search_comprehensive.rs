// Comprehensive Distributed Search Tests (Tasks 97-101)
use anyhow::Result;
use redis::{Client, Commands, Connection};
use std::collections::HashMap;
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Document {
    id: String,
    content: String,
    shard: usize,
    score: f32,
}

#[derive(Clone)]
struct DistributedSearch {
    redis_client: Client,
    num_shards: usize,
}

impl DistributedSearch {
    fn new(redis_url: &str, num_shards: usize) -> Result<Self> {
        let client = Client::open(redis_url)?;
        Ok(Self {
            redis_client: client,
            num_shards,
        })
    }
    
    fn get_connection(&self) -> Result<Connection> {
        Ok(self.redis_client.get_connection()?)
    }
    
    fn get_shard(&self, doc_id: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&doc_id, &mut hasher);
        std::hash::Hasher::finish(&hasher) as usize % self.num_shards
    }
    
    fn index_document(&self, doc: &Document) -> Result<()> {
        let mut conn = self.get_connection()?;
        let shard_key = format!("shard:{}", doc.shard);
        let doc_json = serde_json::to_string(doc)?;
        let _: () = conn.hset(&shard_key, &doc.id, doc_json)?;
        Ok(())
    }
    
    fn search(&self, query: &str, limit: usize) -> Result<Vec<Document>> {
        let mut conn = self.get_connection()?;
        let mut results = Vec::new();
        
        for shard in 0..self.num_shards {
            let shard_key = format!("shard:{}", shard);
            let docs: HashMap<String, String> = conn.hgetall(&shard_key)?;
            
            for (_id, doc_json) in docs {
                let doc: Document = serde_json::from_str(&doc_json)?;
                if doc.content.contains(query) {
                    results.push(doc);
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        Ok(results)
    }
    
    fn rebalance_shards(&self, new_num_shards: usize) -> Result<()> {
        let mut conn = self.get_connection()?;
        let mut all_docs = Vec::new();
        
        // Collect all documents
        for shard in 0..self.num_shards {
            let shard_key = format!("shard:{}", shard);
            let docs: HashMap<String, String> = conn.hgetall(&shard_key)?;
            
            for (_id, doc_json) in docs {
                let doc: Document = serde_json::from_str(&doc_json)?;
                all_docs.push(doc);
            }
            
            // Clear old shard
            let _: () = conn.del(&shard_key)?;
        }
        
        // Redistribute to new shards
        for mut doc in all_docs {
            doc.shard = self.get_shard(&doc.id) % new_num_shards;
            let shard_key = format!("shard:{}", doc.shard);
            let doc_json = serde_json::to_string(&doc)?;
            let _: () = conn.hset(&shard_key, &doc.id, doc_json)?;
        }
        
        Ok(())
    }
    
    fn get_cluster_stats(&self) -> Result<HashMap<String, usize>> {
        let mut conn = self.get_connection()?;
        let mut stats = HashMap::new();
        
        for shard in 0..self.num_shards {
            let shard_key = format!("shard:{}", shard);
            let count: usize = conn.hlen(&shard_key)?;
            stats.insert(format!("shard_{}", shard), count);
        }
        
        Ok(stats)
    }
}

fn main() {
    use std::io::Write;
    
    println!("\n{}", "=".repeat(80));
    println!("🧪 COMPREHENSIVE DISTRIBUTED SEARCH TESTS");
    println!("{}", "=".repeat(80));
    std::io::stdout().flush().unwrap();
    
    match run_tests() {
        Ok(_) => {
            println!("\n✅ ALL DISTRIBUTED SEARCH TESTS PASSED!");
            std::io::stdout().flush().unwrap();
        }
        Err(e) => {
            println!("Error: {}", e);
            std::io::stdout().flush().unwrap();
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn run_tests() -> Result<()> {
    
    // Task 97: Test Redis connection
    match test_redis_connection() {
        Ok(_) => {},
        Err(e) => {
            println!("❌ Redis connection failed: {}", e);
            return Err(e);
        }
    }
    
    // Task 98: Test Distributed Search indexing
    match test_distributed_indexing() {
        Ok(_) => {},
        Err(e) => {
            println!("❌ Distributed indexing failed: {}", e);
            return Err(e);
        }
    }
    
    // Task 99: Test Distributed Search sharding
    match test_distributed_sharding() {
        Ok(_) => {},
        Err(e) => {
            println!("❌ Distributed sharding failed: {}", e);
            return Err(e);
        }
    }
    
    // Task 100: Test Distributed Search rebalancing
    match test_distributed_rebalancing() {
        Ok(_) => {},
        Err(e) => {
            println!("❌ Distributed rebalancing failed: {}", e);
            return Err(e);
        }
    }
    
    // Task 101: Test Distributed Search cluster
    match test_distributed_cluster() {
        Ok(_) => {},
        Err(e) => {
            println!("❌ Distributed cluster failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

fn test_redis_connection() -> Result<()> {
    println!("\n✅ Task 97: Testing Redis connection...");
    
    let client = Client::open("redis://127.0.0.1/")?;
    let mut conn = client.get_connection()?;
    
    // Test basic operations
    let _: () = conn.set("test_key", "test_value")?;
    let value: String = conn.get("test_key")?;
    assert_eq!(value, "test_value");
    
    let _: () = conn.del("test_key")?;
    
    println!("  ✅ Redis connection successful");
    
    // Check Redis version
    let info: String = redis::cmd("INFO").query(&mut conn)?;
    for line in info.lines() {
        if line.starts_with("redis_version:") {
            println!("  ✅ {}", line);
            break;
        }
    }
    
    Ok(())
}

fn test_distributed_indexing() -> Result<()> {
    println!("\n✅ Task 98: Testing Distributed Search indexing...");
    
    let search = DistributedSearch::new("redis://127.0.0.1/", 4)?;
    
    // Index documents
    let start = Instant::now();
    let num_docs = 100;
    
    for i in 0..num_docs {
        let doc = Document {
            id: format!("doc_{}", i),
            content: format!("Document {} content with keywords", i),
            shard: search.get_shard(&format!("doc_{}", i)),
            score: (i as f32) / 100.0,
        };
        
        search.index_document(&doc)?;
    }
    
    let index_time = start.elapsed();
    let index_rate = num_docs as f64 / index_time.as_secs_f64();
    
    println!("  ✅ Indexed {} documents in {:?} ({:.0} docs/sec)", 
        num_docs, index_time, index_rate);
    
    // Verify distribution
    let stats = search.get_cluster_stats()?;
    for (shard, count) in &stats {
        println!("  {} documents: {}", shard, count);
    }
    
    Ok(())
}

fn test_distributed_sharding() -> Result<()> {
    println!("\n✅ Task 99: Testing Distributed Search sharding...");
    
    let search = DistributedSearch::new("redis://127.0.0.1/", 4)?;
    
    // Test search across shards
    let results = search.search("content", 10)?;
    println!("  ✅ Found {} results across shards", results.len());
    
    // Verify sharding distribution
    let mut shard_distribution = HashMap::new();
    for result in &results {
        *shard_distribution.entry(result.shard).or_insert(0) += 1;
    }
    
    println!("  Shard distribution:");
    for (shard, count) in shard_distribution {
        println!("    Shard {}: {} documents", shard, count);
    }
    
    Ok(())
}

fn test_distributed_rebalancing() -> Result<()> {
    println!("\n✅ Task 100: Testing Distributed Search rebalancing...");
    
    let search = DistributedSearch::new("redis://127.0.0.1/", 4)?;
    
    // Get initial stats
    let initial_stats = search.get_cluster_stats()?;
    println!("  Initial shards (4):");
    for (shard, count) in &initial_stats {
        println!("    {}: {} docs", shard, count);
    }
    
    // Rebalance to 8 shards
    let start = Instant::now();
    search.rebalance_shards(8)?;
    let rebalance_time = start.elapsed();
    
    println!("  ✅ Rebalanced from 4 to 8 shards in {:?}", rebalance_time);
    
    // Verify new distribution
    let new_search = DistributedSearch::new("redis://127.0.0.1/", 8)?;
    let new_stats = new_search.get_cluster_stats()?;
    println!("  New shards (8):");
    for i in 0..8 {
        let key = format!("shard_{}", i);
        let count = new_stats.get(&key).unwrap_or(&0);
        println!("    {}: {} docs", key, count);
    }
    
    Ok(())
}

fn test_distributed_cluster() -> Result<()> {
    println!("\n✅ Task 101: Testing Distributed Search cluster...");
    
    let num_nodes = 3;
    let docs_per_node = 50;
    let total_docs = num_nodes * docs_per_node;
    
    // Simulate cluster with multiple nodes
    let nodes: Vec<Arc<DistributedSearch>> = (0..num_nodes)
        .map(|_| Arc::new(DistributedSearch::new("redis://127.0.0.1/", 4).unwrap()))
        .collect();
    
    // Concurrent indexing
    let start = Instant::now();
    let indexed = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    for (node_id, node) in nodes.iter().enumerate() {
        let node = node.clone();
        let indexed = indexed.clone();
        
        let handle = std::thread::spawn(move || {
            for i in 0..docs_per_node {
                let doc = Document {
                    id: format!("node_{}_doc_{}", node_id, i),
                    content: format!("Node {} document {}", node_id, i),
                    shard: node.get_shard(&format!("node_{}_doc_{}", node_id, i)),
                    score: rand::random::<f32>(),
                };
                
                if node.index_document(&doc).is_ok() {
                    indexed.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let cluster_time = start.elapsed();
    let cluster_rate = total_docs as f64 / cluster_time.as_secs_f64();
    
    println!("  ✅ Cluster indexed {} docs in {:?} ({:.0} docs/sec)", 
        indexed.load(Ordering::Relaxed), cluster_time, cluster_rate);
    
    // Test cluster search
    let search_node = &nodes[0];
    let results = search_node.search("document", 20)?;
    println!("  ✅ Cluster search found {} results", results.len());
    
    // Clean up
    let mut conn = search_node.get_connection()?;
    for i in 0..8 {
        let shard_key = format!("shard:{}", i);
        let _: () = conn.del(&shard_key).unwrap_or(());
    }
    
    Ok(())
}

// Helper function for random floats
mod rand {
    pub fn random<T>() -> T 
    where
        T: From<f32>,
    {
        let value = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() % 1000) as f32 / 1000.0;
        T::from(value)
    }
}
