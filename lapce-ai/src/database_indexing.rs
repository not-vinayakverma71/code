/// Database Indexing Strategy - Day 43 AM
use std::collections::{BTreeMap, HashMap};
use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct IndexManager {
    indexes: HashMap<String, Index>,
    statistics: IndexStatistics,
}

#[derive(Debug, Clone)]
pub struct Index {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub unique: bool,
    pub btree: BTreeMap<Vec<u8>, Vec<u64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
    FullText,
    GiST,
    GIN,
}

#[derive(Debug, Clone, Default)]
pub struct IndexStatistics {
    pub total_indexes: usize,
    pub index_hits: u64,
    pub index_misses: u64,
    pub avg_lookup_time_us: f64,
    pub space_used_bytes: usize,
}

impl IndexManager {
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
            statistics: IndexStatistics::default(),
        }
    }
    
    pub fn create_index(&mut self, name: String, table: String, columns: Vec<String>, index_type: IndexType) -> Result<()> {
        let index = Index {
            name: name.clone(),
            table,
            columns,
            index_type,
            unique: false,
            btree: BTreeMap::new(),
        };
        
        self.indexes.insert(name, index);
        self.statistics.total_indexes += 1;
        Ok(())
    }
    
    pub fn lookup(&mut self, index_name: &str, key: &[u8]) -> Option<Vec<u64>> {
        let start = std::time::Instant::now();
        
        let result = self.indexes.get(index_name)
            .and_then(|index| index.btree.get(key))
            .cloned();
        
        if result.is_some() {
            self.statistics.index_hits += 1;
        } else {
            self.statistics.index_misses += 1;
        }
        
        let elapsed = start.elapsed().as_micros() as f64;
        let total = self.statistics.index_hits + self.statistics.index_misses;
        self.statistics.avg_lookup_time_us = 
            (self.statistics.avg_lookup_time_us * (total - 1) as f64 + elapsed) / total as f64;
        
        result
    }
    
    pub fn insert(&mut self, index_name: &str, key: Vec<u8>, value: u64) -> Result<()> {
        let index = self.indexes.get_mut(index_name)
            .ok_or_else(|| anyhow::anyhow!("Index not found"))?;
        
        index.btree.entry(key)
            .or_insert_with(Vec::new)
            .push(value);
        
        self.statistics.space_used_bytes += 16;
        Ok(())
    }
    
    pub fn optimize(&mut self, index_name: &str) -> Result<()> {
        let index = self.indexes.get_mut(index_name)
            .ok_or_else(|| anyhow::anyhow!("Index not found"))?;
        
        // Rebuild B-tree for better balance
        let entries: Vec<_> = index.btree.clone().into_iter().collect();
        index.btree.clear();
        
        for (key, values) in entries {
            index.btree.insert(key, values);
        }
        
        Ok(())
    }
}
