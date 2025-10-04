use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Cache_33<K, V> {
    data: HashMap<K, (V, Instant)>,
    ttl: Duration,
}

impl<K: Eq + std::hash::Hash, V: Clone> Cache_33<K, V> {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            data: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }
    
    pub fn get(&self, key: &K) -> Option<V> {
        self.data.get(key).and_then(|(value, time)| {
            if time.elapsed() < self.ttl {
                Some(value.clone())
            } else {
                None
            }
        })
    }
    
    pub fn insert(&mut self, key: K, value: V) {
        self.data.insert(key, (value, Instant::now()));
    }
}