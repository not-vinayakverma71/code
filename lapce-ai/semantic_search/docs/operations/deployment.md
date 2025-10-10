# Deployment Runbook

## Prerequisites

- AWS credentials configured
- Rust 1.75+ installed
- Docker (optional)
- 2GB+ available RAM
- SSD storage recommended

## Environment Setup

```bash
# Required environment variables
export AWS_REGION=us-east-1
export AWS_ACCESS_KEY_ID=<your-key>
export AWS_SECRET_ACCESS_KEY=<your-secret>

# Optional configuration
export LANCEDB_PATH=/var/lib/semantic-search/data
export CACHE_SIZE=10000
export INDEX_COMPACTION_INTERVAL=3600
export ENABLE_CST=true
export RUST_LOG=info,semantic_search=debug
```

## Initial Deployment

1. **Build Release Binary**
```bash
cd /path/to/semantic_search
cargo build --release
```

2. **Create Data Directories**
```bash
sudo mkdir -p /var/lib/semantic-search/data
sudo mkdir -p /var/lib/semantic-search/cache
sudo mkdir -p /var/log/semantic-search
sudo chown -R $USER:$USER /var/lib/semantic-search
```

3. **Install Systemd Service**
```bash
sudo cp semantic-search.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable semantic-search
sudo systemctl start semantic-search
```

4. **Verify Deployment**
```bash
# Check service status
sudo systemctl status semantic-search

# Check logs
sudo journalctl -u semantic-search -f

# Test search endpoint
curl http://localhost:8080/health
```

## Monitoring Setup

1. **Configure Prometheus**
```yaml
# /etc/prometheus/prometheus.yml
scrape_configs:
  - job_name: 'semantic-search'
    static_configs:
      - targets: ['localhost:9090']
```

2. **Import Grafana Dashboard**
```bash
# Import dashboard from docs/grafana-dashboard.json
```

## Troubleshooting

### Service Won't Start
```bash
# Check logs
sudo journalctl -u semantic-search -n 100

# Common issues:
# - Missing AWS credentials
# - Port already in use
# - Insufficient permissions
```

### High Memory Usage
```bash
# Check memory metrics
curl http://localhost:9090/metrics | grep memory_rss

# Restart with lower cache size
export CACHE_SIZE=5000
sudo systemctl restart semantic-search
```

### Slow Searches
```bash
# Trigger index optimization
curl -X POST http://localhost:8080/admin/optimize-index

# Check index stats
curl http://localhost:8080/admin/index-stats
```

## Rollback Procedure

1. **Stop Service**
```bash
sudo systemctl stop semantic-search
```

2. **Restore Previous Binary**
```bash
cp /backup/semantic-search.old /usr/local/bin/semantic-search
```

3. **Restore Data (if needed)**
```bash
rsync -av /backup/data/ /var/lib/semantic-search/data/
```

4. **Start Service**
```bash
sudo systemctl start semantic-search
```
