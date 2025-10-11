# Upstream Tiered Cache Metrics Specification (CST-UP03)

**Target:** CST-tree-sitter Phase4Cache  
**Purpose:** Add comprehensive metrics for tiered cache hit/miss tracking  
**Priority:** Medium

## Overview

This document specifies the required metrics additions to CST-tree-sitter's Phase4Cache to enable production observability and performance monitoring of the tiered caching system.

## Required Metrics

### 1. Cache Tier Hit Counters

```rust
pub struct Phase4Stats {
    // Existing fields...
    pub hot_entries: usize,
    pub warm_entries: usize,
    pub cold_entries: usize,
    pub frozen_entries: usize,
    
    // NEW: Hit counters per tier
    pub hot_tier_hits: u64,
    pub warm_tier_hits: u64,
    pub cold_tier_hits: u64,
    pub frozen_tier_hits: u64,
    pub cache_misses: u64,
    
    // NEW: Latency tracking
    pub hot_tier_latency_us: u64,    // Average μs
    pub warm_tier_latency_us: u64,
    pub cold_tier_latency_us: u64,
    pub frozen_tier_latency_us: u64,
    
    // NEW: Promotion/demotion counters
    pub promotions_cold_to_warm: u64,
    pub promotions_warm_to_hot: u64,
    pub demotions_hot_to_warm: u64,
    pub demotions_warm_to_cold: u64,
    pub evictions_to_frozen: u64,
}
```

### 2. Hit Rate Calculation Methods

```rust
impl Phase4Stats {
    /// Overall cache hit rate (any tier)
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 { 0.0 } else {
            self.total_hits() as f64 / total as f64
        }
    }
    
    /// Total hits across all tiers
    pub fn total_hits(&self) -> u64 {
        self.hot_tier_hits 
            + self.warm_tier_hits 
            + self.cold_tier_hits 
            + self.frozen_tier_hits
    }
    
    /// Total requests (hits + misses)
    pub fn total_requests(&self) -> u64 {
        self.total_hits() + self.cache_misses
    }
    
    /// Hot tier hit rate (hot hits / total hits)
    pub fn hot_tier_percentage(&self) -> f64 {
        let total = self.total_hits();
        if total == 0 { 0.0 } else {
            self.hot_tier_hits as f64 / total as f64
        }
    }
    
    /// Memory tier hit rate (hot + warm)
    pub fn memory_tier_hit_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 { 0.0 } else {
            (self.hot_tier_hits + self.warm_tier_hits) as f64 / total as f64
        }
    }
    
    /// Disk tier hit rate (cold + frozen)
    pub fn disk_tier_hit_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 { 0.0 } else {
            (self.cold_tier_hits + self.frozen_tier_hits) as f64 / total as f64
        }
    }
}
```

### 3. Prometheus Metrics Export

```rust
use prometheus::{Counter, Histogram, IntGauge, Registry};

pub struct Phase4Metrics {
    // Hit counters
    hot_tier_hits: Counter,
    warm_tier_hits: Counter,
    cold_tier_hits: Counter,
    frozen_tier_hits: Counter,
    cache_misses: Counter,
    
    // Latency histograms
    hot_tier_latency: Histogram,
    warm_tier_latency: Histogram,
    cold_tier_latency: Histogram,
    frozen_tier_latency: Histogram,
    
    // Size gauges
    hot_tier_bytes: IntGauge,
    warm_tier_bytes: IntGauge,
    cold_tier_bytes: IntGauge,
    frozen_tier_bytes: IntGauge,
    
    // Entry count gauges
    hot_tier_entries: IntGauge,
    warm_tier_entries: IntGauge,
    cold_tier_entries: IntGauge,
    frozen_tier_entries: IntGauge,
    
    // Transition counters
    promotions_total: Counter,
    demotions_total: Counter,
    evictions_total: Counter,
}

impl Phase4Metrics {
    pub fn new(registry: &Registry) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            hot_tier_hits: Counter::new(
                "phase4_hot_tier_hits_total",
                "Total hits to hot tier"
            )?,
            warm_tier_hits: Counter::new(
                "phase4_warm_tier_hits_total",
                "Total hits to warm tier"
            )?,
            cold_tier_hits: Counter::new(
                "phase4_cold_tier_hits_total",
                "Total hits to cold tier"
            )?,
            frozen_tier_hits: Counter::new(
                "phase4_frozen_tier_hits_total",
                "Total hits to frozen tier"
            )?,
            cache_misses: Counter::new(
                "phase4_cache_misses_total",
                "Total cache misses"
            )?,
            
            hot_tier_latency: Histogram::new(
                "phase4_hot_tier_latency_seconds",
                "Hot tier retrieval latency"
            )?,
            // ... similar for other tiers
            
            hot_tier_bytes: IntGauge::new(
                "phase4_hot_tier_bytes",
                "Bytes stored in hot tier"
            )?,
            // ... similar for other tiers
        })
    }
}
```

## Implementation Changes

### File: `src/phase4_cache.rs`

#### Change 1: Update retrieval method

```rust
impl Phase4Cache {
    pub fn retrieve(
        &self,
        path: &Path,
        hash: u64,
    ) -> Option<Arc<SegmentedBytecodeStream>> {
        let start = std::time::Instant::now();
        
        // Try hot tier first
        if let Some(stream) = self.check_hot_tier(path) {
            let elapsed = start.elapsed();
            self.metrics.hot_tier_hits.inc();
            self.metrics.hot_tier_latency.observe(elapsed.as_secs_f64());
            self.update_stats(|s| {
                s.hot_tier_hits += 1;
                s.hot_tier_latency_us = elapsed.as_micros() as u64;
            });
            return Some(stream);
        }
        
        // Try warm tier
        if let Some(stream) = self.check_warm_tier(path) {
            let elapsed = start.elapsed();
            self.metrics.warm_tier_hits.inc();
            self.metrics.warm_tier_latency.observe(elapsed.as_secs_f64());
            self.update_stats(|s| {
                s.warm_tier_hits += 1;
                s.warm_tier_latency_us = elapsed.as_micros() as u64;
            });
            
            // Promote to hot tier if frequently accessed
            self.maybe_promote_to_hot(path);
            return Some(stream);
        }
        
        // Try cold/frozen tier
        if let Some(stream) = self.check_cold_frozen_tier(path, hash) {
            let elapsed = start.elapsed();
            
            if self.is_frozen(path) {
                self.metrics.frozen_tier_hits.inc();
                self.update_stats(|s| {
                    s.frozen_tier_hits += 1;
                    s.frozen_tier_latency_us = elapsed.as_micros() as u64;
                });
            } else {
                self.metrics.cold_tier_hits.inc();
                self.update_stats(|s| {
                    s.cold_tier_hits += 1;
                    s.cold_tier_latency_us = elapsed.as_micros() as u64;
                });
            }
            
            // Promote to warm tier
            self.promote_to_warm(path, stream.clone());
            return Some(stream);
        }
        
        // Cache miss
        self.metrics.cache_misses.inc();
        self.update_stats(|s| s.cache_misses += 1);
        None
    }
}
```

#### Change 2: Track promotions/demotions

```rust
impl Phase4Cache {
    fn promote_to_hot(&self, path: &Path, stream: Arc<SegmentedBytecodeStream>) {
        // Move from warm to hot
        self.metrics.promotions_total.inc();
        self.update_stats(|s| s.promotions_warm_to_hot += 1);
        
        // Implementation...
    }
    
    fn demote_to_warm(&self, path: &Path) {
        // Move from hot to warm
        self.metrics.demotions_total.inc();
        self.update_stats(|s| s.demotions_hot_to_warm += 1);
        
        // Implementation...
    }
    
    fn evict_to_frozen(&self, path: &Path) {
        // Move to frozen tier
        self.metrics.evictions_total.inc();
        self.update_stats(|s| s.evictions_to_frozen += 1);
        
        // Implementation...
    }
}
```

## Integration with semantic_search

### Usage Example

```rust
use lapce_tree_sitter::{Phase4Cache, Phase4Config};

let cache = Phase4Cache::new(Phase4Config::default())?;

// Retrieve with automatic tier tracking
match cache.retrieve(&path, hash) {
    Some(stream) => {
        // Cache hit - check which tier
        let stats = cache.stats();
        println!("Hot tier hit rate: {:.1}%", 
                 stats.hot_tier_percentage() * 100.0);
    }
    None => {
        // Cache miss - parse and store
        let stream = parse_and_encode(&path)?;
        cache.store(path, hash, stream)?;
    }
}

// Export metrics to Prometheus
let stats = cache.stats();
println!("Overall hit rate: {:.1}%", stats.hit_rate() * 100.0);
println!("Memory tier hits: {:.1}%", stats.memory_tier_hit_rate() * 100.0);
println!("Disk tier hits: {:.1}%", stats.disk_tier_hit_rate() * 100.0);
```

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_tier_hit_counters() {
    let cache = Phase4Cache::new(Phase4Config::default()).unwrap();
    
    // Store and retrieve - should hit hot tier
    cache.store(path.clone(), hash, stream.clone()).unwrap();
    let _ = cache.retrieve(&path, hash);
    
    let stats = cache.stats();
    assert_eq!(stats.hot_tier_hits, 1);
    assert_eq!(stats.cache_misses, 0);
}

#[test]
fn test_promotion_counters() {
    let cache = Phase4Cache::new(Phase4Config::default()).unwrap();
    
    // Fill hot tier to trigger demotion
    for i in 0..1000 {
        cache.store(PathBuf::from(format!("file_{}", i)), i, stream.clone()).unwrap();
    }
    
    let stats = cache.stats();
    assert!(stats.demotions_hot_to_warm > 0);
}
```

### Integration Tests

```rust
#[test]
fn test_tiered_retrieval_latency() {
    let cache = Phase4Cache::new(Phase4Config::default()).unwrap();
    
    // Hot tier should be fastest
    let hot_latency = measure_retrieval_latency(&cache, &hot_path);
    
    // Warm tier should be slower than hot
    let warm_latency = measure_retrieval_latency(&cache, &warm_path);
    
    // Frozen tier should be slowest
    let frozen_latency = measure_retrieval_latency(&cache, &frozen_path);
    
    assert!(hot_latency < warm_latency);
    assert!(warm_latency < frozen_latency);
}
```

## Monitoring Dashboard

### Prometheus Queries

```promql
# Overall hit rate
sum(rate(phase4_hot_tier_hits_total[5m])) 
  + sum(rate(phase4_warm_tier_hits_total[5m]))
  + sum(rate(phase4_cold_tier_hits_total[5m]))
  + sum(rate(phase4_frozen_tier_hits_total[5m]))
/ 
(
  sum(rate(phase4_hot_tier_hits_total[5m]))
  + sum(rate(phase4_warm_tier_hits_total[5m]))
  + sum(rate(phase4_cold_tier_hits_total[5m]))
  + sum(rate(phase4_frozen_tier_hits_total[5m]))
  + sum(rate(phase4_cache_misses_total[5m]))
)

# P95 latency by tier
histogram_quantile(0.95, rate(phase4_hot_tier_latency_seconds_bucket[5m]))
histogram_quantile(0.95, rate(phase4_warm_tier_latency_seconds_bucket[5m]))
histogram_quantile(0.95, rate(phase4_cold_tier_latency_seconds_bucket[5m]))
histogram_quantile(0.95, rate(phase4_frozen_tier_latency_seconds_bucket[5m]))

# Promotion rate
rate(phase4_promotions_total[5m])
```

### Grafana Dashboard Panels

1. **Hit Rate by Tier** (Stacked area chart)
   - Hot tier hits
   - Warm tier hits
   - Cold tier hits
   - Frozen tier hits
   - Misses

2. **Latency by Tier** (Line chart)
   - P50, P95, P99 for each tier

3. **Cache Size** (Gauge)
   - Hot tier bytes / max
   - Warm tier bytes / max
   - Disk usage

4. **Promotion/Demotion Rate** (Counter)
   - Promotions per minute
   - Demotions per minute
   - Evictions per minute

## Performance Targets

| Metric | Target | Critical Threshold |
|--------|--------|--------------------|
| Overall hit rate | >85% | <70% |
| Hot tier hit rate | >60% | <40% |
| Memory tier hit rate | >75% | <50% |
| Hot tier P95 latency | <50μs | >200μs |
| Warm tier P95 latency | <500μs | >2ms |
| Frozen tier P95 latency | <10ms | >50ms |

## Rollout Plan

1. **Phase 1:** Add metric counters to Phase4Stats
2. **Phase 2:** Integrate Prometheus metrics
3. **Phase 3:** Update retrieval methods to track hits
4. **Phase 4:** Add promotion/demotion tracking
5. **Phase 5:** Deploy Grafana dashboard
6. **Phase 6:** Set up alerting rules

## Dependencies

- **Prometheus client:** `prometheus = "0.13"`
- **CST-tree-sitter:** Phase4Cache API exposure (CST-UP02)

## Timeline

- **CST-UP02 completion:** Required prerequisite
- **Implementation:** 2-3 days
- **Testing:** 1 day
- **Documentation:** 1 day
- **Total:** ~1 week after CST-UP02

---

*Last updated: 2025-10-11*  
*Status: Specification complete, awaiting upstream implementation*
