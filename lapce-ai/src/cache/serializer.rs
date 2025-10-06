/// Serializer - EXACT implementation from docs line 65
use anyhow::Result;
use super::types::{CacheKey, CacheValue};

/// Serializer for L3 cache
pub struct Serializer {
    /// Format type
    format: SerializationFormat,
}

#[derive(Clone)]
pub enum SerializationFormat {
    Bincode,
    Json,
    MessagePack,
}

impl Serializer {
    pub fn new(format: SerializationFormat) -> Self {
        Self { format }
    }
    
    /// Serialize a cache value
    pub fn serialize(&self, value: &CacheValue) -> Result<Vec<u8>> {
        match self.format {
            SerializationFormat::Bincode => {
                Ok(bincode::serialize(value)?)
            }
            SerializationFormat::Json => {
                Ok(serde_json::to_vec(value)?)
            }
            SerializationFormat::MessagePack => {
                Ok(rmp_serde::to_vec(value)?)
            }
        }
    }
    
    /// Deserialize a cache value
    pub fn deserialize(&self, data: &[u8]) -> Result<CacheValue> {
        match self.format {
            SerializationFormat::Bincode => {
                Ok(bincode::deserialize(data)?)
            }
            SerializationFormat::Json => {
                Ok(serde_json::from_slice(data)?)
            }
            SerializationFormat::MessagePack => {
                Ok(rmp_serde::from_slice(data)?)
            }
        }
    }
    
    /// Serialize a cache key
    pub fn serialize_key(&self, key: &CacheKey) -> Result<Vec<u8>> {
        Ok(key.0.as_bytes().to_vec())
    }
    
    /// Deserialize a cache key
    pub fn deserialize_key(&self, data: &[u8]) -> Result<CacheKey> {
        Ok(CacheKey(String::from_utf8(data.to_vec())?))
    }
}
