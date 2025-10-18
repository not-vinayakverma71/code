# Metrics Guide

## Available Metrics

### Cache Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `cst_cache_hits_total` | Counter | Total number of cache hits |
| `cst_cache_misses_total` | Counter | Total number of cache misses |
| `cst_cache_promotions_total` | Counter | Tier promotions (cold→warm→hot) |
| `cst_cache_demotions_total` | Counter | Tier demotions (hot→warm→cold) |
| `cst_cache_memory_bytes` | Gauge | Current memory usage |
| `cst_cache_disk_bytes` | Gauge | Current disk usage |

### Operation Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `cst_cache_get_duration_seconds` | Histogram | Cache get operation latency |
| `cst_cache_store_duration_seconds` | Histogram | Cache store operation latency |
| `cst_parse_duration_seconds` | Histogram | Tree-sitter parse duration |
| `cst_verify_duration_seconds` | Histogram | Bytecode verification duration |
| `cst_segment_loads_total` | Counter | Segment loads from disk |
| `cst_bytes_written_total` | Counter | Total bytes written |
| `cst_bytes_read_total` | Counter | Total bytes read |

## Prometheus Configuration

### scrape_configs Example

```yaml
scrape_configs:
  - job_name: 'cst-tree-sitter'
    static_configs:
      - targets: ['localhost:9000']
    scrape_interval: 15s
```

### Starting the Metrics Server

```rust
use lapce_tree_sitter::metrics_server;

#[tokio::main]
async fn main() {
    // Start metrics server on port 9000
    metrics_server::start(9000).await.unwrap();
}
```

## Key Performance Indicators (KPIs)

### Cache Efficiency
```promql
# Cache hit ratio
rate(cst_cache_hits_total[5m]) / 
(rate(cst_cache_hits_total[5m]) + rate(cst_cache_misses_total[5m]))
```

### Memory Pressure
```promql
# Memory usage vs budget (100MB)
cst_cache_memory_bytes / (100 * 1024 * 1024) * 100
```

### Operation Latency
```promql
# P99 get latency
histogram_quantile(0.99, rate(cst_cache_get_duration_seconds_bucket[5m]))

# P99 parse latency
histogram_quantile(0.99, rate(cst_parse_duration_seconds_bucket[5m]))
```

### Tier Distribution
```promql
# Promotion rate
rate(cst_cache_promotions_total[5m])

# Demotion rate  
rate(cst_cache_demotions_total[5m])

# Tier churn (promotions + demotions)
rate(cst_cache_promotions_total[5m]) + rate(cst_cache_demotions_total[5m])
```

### I/O Activity
```promql
# Segment load rate
rate(cst_segment_loads_total[5m])

# Write throughput (bytes/sec)
rate(cst_bytes_written_total[1m])

# Read throughput (bytes/sec)
rate(cst_bytes_read_total[1m])
```

## Alerting Rules

### High Memory Usage
```yaml
groups:
  - name: cst_alerts
    rules:
      - alert: HighMemoryUsage
        expr: cst_cache_memory_bytes > 100 * 1024 * 1024
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "CST cache memory usage exceeds 100MB"
          description: "Memory usage: {{ $value | humanize }}"
```

### Low Cache Hit Ratio
```yaml
- alert: LowCacheHitRatio
  expr: |
    rate(cst_cache_hits_total[5m]) / 
    (rate(cst_cache_hits_total[5m]) + rate(cst_cache_misses_total[5m])) < 0.8
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Cache hit ratio below 80%"
```

### High Latency
```yaml
- alert: HighParseLatency
  expr: histogram_quantile(0.99, rate(cst_parse_duration_seconds_bucket[5m])) > 0.01
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "P99 parse latency exceeds 10ms"
```

## Grafana Dashboard

### Recommended Panels

1. **Cache Performance**
   - Hit ratio gauge
   - Hit/miss rate graph
   - Memory usage over time

2. **Operation Latency**
   - P50/P90/P99 latency heatmap
   - Parse duration histogram
   - Store duration histogram

3. **Tier Management**
   - Tier distribution pie chart
   - Promotion/demotion rates
   - Segment loads per minute

4. **I/O Metrics**
   - Read/write throughput
   - Bytes processed counter
   - Disk usage gauge

### Example Dashboard JSON

```json
{
  "dashboard": {
    "title": "CST Tree-sitter Metrics",
    "panels": [
      {
        "title": "Cache Hit Ratio",
        "targets": [
          {
            "expr": "rate(cst_cache_hits_total[5m]) / (rate(cst_cache_hits_total[5m]) + rate(cst_cache_misses_total[5m]))"
          }
        ],
        "type": "gauge"
      },
      {
        "title": "Memory Usage",
        "targets": [
          {
            "expr": "cst_cache_memory_bytes"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
```

## Performance Baselines

| Metric | Good | Warning | Critical |
|--------|------|---------|----------|
| Cache Hit Ratio | >90% | 80-90% | <80% |
| P99 Get Latency | <5ms | 5-10ms | >10ms |
| P99 Parse Latency | <10ms | 10-20ms | >20ms |
| Memory Usage | <80MB | 80-100MB | >100MB |
| Segment Load Rate | <10/s | 10-50/s | >50/s |
