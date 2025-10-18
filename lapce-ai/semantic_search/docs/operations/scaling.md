# Scaling Runbook - SEM-014-B

## Vertical Scaling

### Memory Optimization

```bash
# Monitor current memory usage
curl http://localhost:9090/metrics | grep memory_rss

# Adjust cache sizes
export CACHE_SIZE=5000  # Reduce from 10000
export CACHE_TTL=300    # 5 minutes instead of default

# Enable memory-mapped storage
export ENABLE_MMAP=true
export MMAP_SIZE_GB=4
```

### CPU Optimization

```bash
# Set worker threads
export TOKIO_WORKER_THREADS=8

# Enable CPU affinity
taskset -c 0-7 /usr/local/bin/semantic-search
```

## Horizontal Scaling

### Load Balancer Setup

```nginx
upstream semantic_search {
    least_conn;
    server 10.0.1.10:8080;
    server 10.0.1.11:8080;
    server 10.0.1.12:8080;
}

server {
    listen 80;
    location / {
        proxy_pass http://semantic_search;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Shared Storage

```bash
# Mount shared NFS for index data
sudo mount -t nfs4 nfs-server:/semantic-data /var/lib/semantic-search/data

# Or use S3-compatible storage
export STORAGE_BACKEND=s3
export S3_BUCKET=semantic-search-data
```

## Performance Tuning

### Index Configuration

```bash
# Increase IVF partitions for large datasets
export IVF_PARTITIONS=1024  # Default: 512
export PQ_SUBVECTORS=128    # Default: 96

# Adjust batch sizes
export INDEX_BATCH_SIZE=5000
export SEARCH_BATCH_SIZE=100
```

### Connection Pooling

```bash
# AWS client settings
export AWS_MAX_CONNECTIONS=50
export AWS_CONNECTION_TIMEOUT=30
```

## Monitoring Thresholds

```yaml
# prometheus_rules.yml
- alert: HighSearchLatency
  expr: histogram_quantile(0.95, rate(semantic_search_latency_seconds_bucket[5m])) > 0.5
  
- alert: HighMemoryUsage
  expr: semantic_search_memory_rss_bytes > 8000000000  # 8GB
  
- alert: LowCacheHitRate
  expr: rate(semantic_search_cache_hits_total[5m]) / (rate(semantic_search_cache_hits_total[5m]) + rate(semantic_search_cache_misses_total[5m])) < 0.7
```

## Capacity Planning

### Estimating Requirements

```
Memory Required = (Index Size * 1.2) + (Cache Size * Entry Size) + 500MB overhead
CPU Cores = (QPS / 100) rounded up
Storage = (Vectors * Dimensions * 4 bytes) * 1.5 for indexes
```

### Example Configurations

**Small (1M vectors)**
- 4 CPU cores
- 8GB RAM
- 50GB SSD

**Medium (10M vectors)**
- 8 CPU cores
- 32GB RAM
- 500GB SSD

**Large (100M vectors)**
- 16+ CPU cores
- 128GB RAM
- 5TB SSD
- Consider sharding
