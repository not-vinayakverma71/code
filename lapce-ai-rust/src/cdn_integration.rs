/// CDN Integration - Day 44 PM
use std::collections::HashMap;
use anyhow::Result;
use reqwest;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct CDNManager {
    provider: CDNProvider,
    edge_locations: Vec<EdgeLocation>,
    cache_rules: HashMap<String, CacheRule>,
}

#[derive(Debug, Clone)]
pub enum CDNProvider {
    CloudFlare,
    Fastly,
    CloudFront,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct EdgeLocation {
    pub region: String,
    pub endpoint: String,
    pub latency_ms: f64,
}

#[derive(Debug, Clone)]
pub struct CacheRule {
    pub path_pattern: String,
    pub ttl_seconds: u64,
    pub cache_control: String,
    pub bypass_conditions: Vec<String>,
}

impl CDNManager {
    pub fn new(provider: CDNProvider) -> Self {
        Self {
            provider,
            edge_locations: vec![
                EdgeLocation {
                    region: "us-east".to_string(),
                    endpoint: "edge1.cdn.example.com".to_string(),
                    latency_ms: 10.0,
                },
                EdgeLocation {
                    region: "eu-west".to_string(),
                    endpoint: "edge2.cdn.example.com".to_string(),
                    latency_ms: 25.0,
                },
            ],
            cache_rules: HashMap::new(),
        }
    }
    
    pub fn add_cache_rule(&mut self, pattern: String, ttl: u64) {
        self.cache_rules.insert(pattern.clone(), CacheRule {
            path_pattern: pattern,
            ttl_seconds: ttl,
            cache_control: format!("public, max-age={}", ttl),
            bypass_conditions: vec![],
        });
    }
    
    pub async fn purge_cache(&self, path: &str) -> Result<()> {
        // Simulate CDN purge
        match &self.provider {
            CDNProvider::CloudFlare => {
                // CloudFlare API call
            }
            _ => {}
        }
        Ok(())
    }
    
    pub fn get_nearest_edge(&self, client_region: &str) -> &EdgeLocation {
        self.edge_locations.iter()
            .min_by_key(|loc| if loc.region == client_region { 0 } else { 100 })
            .unwrap_or(&self.edge_locations[0])
    }
}
