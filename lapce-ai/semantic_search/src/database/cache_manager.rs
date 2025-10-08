// Concrete CacheManager implementation with persistent storage
use super::cache_interface::ICacheManager;
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CacheData {
    hashes: HashMap<String, String>,
}

pub struct CacheManager {
    cache_file: PathBuf,
    data: RwLock<CacheData>,
}

impl CacheManager {
    pub fn new(_context: Arc<dyn std::any::Any + Send + Sync>, workspace: PathBuf) -> Self {
        let cache_file = workspace.join(".cache").join("file_hashes.json");
        Self::new_with_path(cache_file)
    }
    
    pub async fn initialize(&self) -> Result<()> {
        Ok(())
    }
    
    pub async fn clear_cache_file(&self) -> Result<()> {
        let mut data = self.data.write().unwrap();
        data.hashes.clear();
        drop(data);
        self.save_to_disk()?;
        Ok(())
    }
    
    fn save_cache(&self) -> Result<()> {
        self.save_to_disk()
    }
    
    fn save_to_disk(&self) -> Result<()> {
        let data = self.data.read().unwrap();
        let json = serde_json::to_string_pretty(&data.hashes)
            .map_err(|e| Error::Runtime { message: e.to_string() })?;
        fs::write(&self.cache_file, json)
            .map_err(|e| Error::Runtime { message: e.to_string() })?;
        Ok(())
    }
    
    pub fn new_with_path(cache_file: PathBuf) -> Self {
        
        // Create cache directory if needed
        if let Some(parent) = cache_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        // Load existing cache or create new
        let data = if cache_file.exists() {
            fs::read_to_string(&cache_file)
                .ok()
                .and_then(|content| serde_json::from_str::<CacheData>(&content).ok())
                .unwrap_or_default()
        } else {
            CacheData::default()
        };
        
        Self {
            cache_file,
            data: RwLock::new(data),
        }
    }
    
    fn persist(&self) {
        if let Ok(data) = self.data.read() {
            if let Ok(json) = serde_json::to_string_pretty(&*data) {
                let _ = fs::write(&self.cache_file, json);
            }
        }
    }
}

impl ICacheManager for CacheManager {
    fn get_hash(&self, file_path: &str) -> Option<String> {
        self.data.read().ok()?.hashes.get(file_path).cloned()
    }
    
    fn update_hash(&self, file_path: &str, hash: String) {
        if let Ok(mut data) = self.data.write() {
            data.hashes.insert(file_path.to_string(), hash);
        }
        self.persist();
    }
    
    fn delete_hash(&self, file_path: &str) {
        if let Ok(mut data) = self.data.write() {
            data.hashes.remove(file_path);
        }
        self.persist();
    }
    
    fn get_all_hashes(&self) -> HashMap<String, String> {
        self.data.read().map(|d| d.hashes.clone()).unwrap_or_default()
    }
}

