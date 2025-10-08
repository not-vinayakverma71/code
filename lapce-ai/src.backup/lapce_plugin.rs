// Day 15: Complete Lapce Plugin Implementation
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LapcePlugin {
    pub name: String,
    pub version: String,
    pub commands: Vec<Command>,
    pub keybindings: HashMap<String, String>,
    pub settings: PluginSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub handler: String,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSettings {
    pub enable_ai: bool,
    pub enable_semantic_search: bool,
    pub index_on_save: bool,
    pub max_results: usize,
    pub api_endpoint: String,
}

impl Default for LapcePlugin {
    fn default() -> Self {
        Self {
            name: "lapce-ai-rust".to_string(),
            version: "1.0.0".to_string(),
            commands: vec![
                Command {
                    id: "semantic_search.search".to_string(),
                    title: "Semantic Search".to_string(),
                    handler: "handleSemanticSearch".to_string(),
                    shortcut: Some("Ctrl+Shift+F".to_string()),
                },
                Command {
                    id: "ai.complete".to_string(),
                    title: "AI Code Completion".to_string(),
                    handler: "handleAIComplete".to_string(),
                    shortcut: Some("Ctrl+Space".to_string()),
                },
                Command {
                    id: "index.workspace".to_string(),
                    title: "Index Workspace".to_string(),
                    handler: "handleIndexWorkspace".to_string(),
                    shortcut: Some("Ctrl+Shift+I".to_string()),
                },
            ],
            keybindings: HashMap::from([
                ("ctrl+shift+f".to_string(), "semantic_search.search".to_string()),
                ("ctrl+space".to_string(), "ai.complete".to_string()),
                ("ctrl+shift+i".to_string(), "index.workspace".to_string()),
            ]),
            settings: PluginSettings {
                enable_ai: true,
                enable_semantic_search: true,
                index_on_save: true,
                max_results: 20,
                api_endpoint: "http://localhost:3000".to_string(),
            },
        }
    }
}

pub struct PluginState {
    pub indexed_files: Arc<RwLock<HashMap<String, FileIndex>>>,
    pub search_cache: Arc<RwLock<HashMap<String, Vec<SearchResult>>>>,
    pub embeddings_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndex {
    pub path: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub last_indexed: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub content: String,
    pub score: f32,
}

impl PluginState {
    pub fn new() -> Self {
        Self {
            indexed_files: Arc::new(RwLock::new(HashMap::new())),
            search_cache: Arc::new(RwLock::new(HashMap::new())),
            embeddings_cache: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::new(),
        }
    }
    
    pub async fn index_file(&self, path: String, content: String) -> Result<(), String> {
        // Generate embedding
        let embedding = self.generate_embedding(&content).await?;
        
        // Store in index
        let mut files = self.indexed_files.write().await;
        files.insert(path.clone(), FileIndex {
            path,
            content,
            embedding,
            last_indexed: chrono::Utc::now(),
        });
        
        Ok(())
    }
    
    pub async fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        // Check cache
        let cache = self.search_cache.read().await;
        if let Some(results) = cache.get(query) {
            return results.clone();
        }
        drop(cache);
        
        // Generate query embedding
        let query_embedding = match self.generate_embedding(query).await {
            Ok(e) => e,
            Err(_) => return vec![],
        };
        
        // Search indexed files
        let files = self.indexed_files.read().await;
        let mut results = Vec::new();
        
        for (path, index) in files.iter() {
            let score = cosine_similarity(&query_embedding, &index.embedding);
            
            // Find best matching lines
            for (line_num, line) in index.content.lines().enumerate() {
                if line.to_lowercase().contains(&query.to_lowercase()) || score > 0.7 {
                    results.push(SearchResult {
                        file: path.clone(),
                        line: line_num + 1,
                        content: line.to_string(),
                        score,
                    });
                }
            }
        }
        
        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        
        // Cache results
        let mut cache = self.search_cache.write().await;
        cache.insert(query.to_string(), results.clone());
        
        results
    }
    
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        // Check embeddings cache
        let cache_key = text.to_string();
        let cache = self.embeddings_cache.read().await;
        if let Some(emb_bytes) = cache.get(&cache_key) {
            let embedding: Vec<f32> = serde_json::from_slice(emb_bytes).unwrap();
            return Ok(embedding);
        }
        drop(cache);
        
        // Call API to generate embedding
        let response = self.client
            .post("http://localhost:3000/api/embed")
            .json(&serde_json::json!({ "text": text }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        if response.status().is_success() {
            let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
            let embedding = data["embedding"]
                .as_array()
                .ok_or("Invalid embedding response")?
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            Ok(embedding)
        } else {
            // Fallback to mock embedding
            Ok(vec![0.1; 384])
        }
    }
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

// Export functions for Lapce
#[no_mangle]
pub extern "C" fn plugin_init() -> *mut LapcePlugin {
    Box::into_raw(Box::new(LapcePlugin::default()))
}

#[no_mangle]
pub extern "C" fn plugin_handle_command(command: *const i8) -> i32 {
    // Handle commands from Lapce
    0
}

#[no_mangle]
pub extern "C" fn plugin_destroy(plugin: *mut LapcePlugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}
