# Operations Guide

## Deployment

### Docker Deployment

```bash
# Build image
docker build -t cst-tree-sitter:latest .

# Run with resource limits
docker run -d \
  --name cst-tree-sitter \
  -p 9000:9000 \
  -m 512m \
  --cpus="2.0" \
  -v /data/cst:/data \
  cst-tree-sitter:latest
```

### Systemd Service

```ini
[Unit]
Description=CST Tree-sitter Service
After=network.target

[Service]
Type=simple
User=cst-service
WorkingDirectory=/opt/cst-tree-sitter
ExecStart=/opt/cst-tree-sitter/bin/cst-server
Restart=on-failure
RestartSec=10

# Resource limits
MemoryMax=512M
CPUQuota=200%

# Security
PrivateTmp=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CST_CACHE_DIR` | `/tmp/cst-cache` | Cache directory |
| `CST_MEMORY_MB` | `100` | Memory budget in MB |
| `CST_SEGMENT_SIZE` | `262144` | Segment size in bytes |
| `CST_METRICS_PORT` | `9000` | Prometheus metrics port |
| `CST_LOG_LEVEL` | `info` | Log level (debug/info/warn/error) |
| `CST_MAX_WORKERS` | CPU count | Worker thread pool size |

## Monitoring

### Health Checks

```bash
# HTTP health endpoint
curl http://localhost:9000/health

# Expected response
{
  "status": "healthy",
  "cache_entries": 1234,
  "memory_usage_mb": 45.6,
  "uptime_seconds": 3600
}
```

### Key Metrics to Monitor

1. **Memory Usage**
   - Alert if > 80% of budget
   - Check for memory leaks (steady increase)

2. **Cache Hit Ratio**
   - Should be > 90% in steady state
   - Low ratio indicates working set too large

3. **Parse Latency**
   - P99 should be < 10ms
   - Spike indicates performance issue

4. **Disk I/O**
   - Segment loads should be < 10/sec
   - High rate indicates cache thrashing

### Logging

```bash
# View logs
journalctl -u cst-tree-sitter -f

# Log levels
export CST_LOG_LEVEL=debug  # Verbose debugging
export CST_LOG_LEVEL=info   # Normal operation
export CST_LOG_LEVEL=warn   # Warnings only
export CST_LOG_LEVEL=error  # Errors only

# Structured log example
{
  "timestamp": "2024-01-08T10:00:00Z",
  "level": "INFO",
  "module": "cache",
  "message": "Cache hit",
  "key": "file.rs:12345",
  "latency_ms": 0.5
}
```

## Backup & Recovery

### Backup Strategy

1. **Cache State**
   ```bash
   # Backup frozen tier
   rsync -av /data/cst/frozen/ /backup/cst-frozen/
   
   # Backup metadata
   cp /data/cst/metadata.json /backup/
   ```

2. **Configuration**
   ```bash
   # Backup config
   cp /etc/cst-tree-sitter/config.toml /backup/
   ```

### Recovery Procedure

1. **Stop Service**
   ```bash
   systemctl stop cst-tree-sitter
   ```

2. **Restore Data**
   ```bash
   # Restore frozen tier
   rsync -av /backup/cst-frozen/ /data/cst/frozen/
   
   # Restore metadata
   cp /backup/metadata.json /data/cst/
   ```

3. **Verify Integrity**
   ```bash
   # Run integrity check
   cst-cli verify --data-dir /data/cst
   ```

4. **Start Service**
   ```bash
   systemctl start cst-tree-sitter
   ```

## Scaling

### Vertical Scaling

1. **Increase Memory**
   ```bash
   # Update memory budget
   export CST_MEMORY_MB=200
   
   # Restart service
   systemctl restart cst-tree-sitter
   ```

2. **Increase CPU**
   ```bash
   # Update worker threads
   export CST_MAX_WORKERS=8
   ```

### Horizontal Scaling

1. **Load Balancing**
   ```nginx
   upstream cst_backend {
       server cst1:9000;
       server cst2:9000;
       server cst3:9000;
   }
   ```

2. **Shared Storage**
   - Use NFS/GlusterFS for frozen tier
   - Redis for distributed cache

## Maintenance

### Regular Tasks

| Task | Frequency | Command |
|------|-----------|---------|
| Clear old segments | Daily | `cst-cli cleanup --older-than 7d` |
| Defragment cache | Weekly | `cst-cli defrag` |
| Backup frozen tier | Daily | `backup-cst.sh` |
| Rotate logs | Daily | `logrotate -f /etc/logrotate.d/cst` |
| Update grammars | Monthly | `cst-cli update-grammars` |

### Cache Management

```bash
# View cache statistics
cst-cli stats

# Clear entire cache
cst-cli clear --all

# Clear specific language
cst-cli clear --language rust

# Prewarm cache
cst-cli prewarm --directory /src

# Export cache metrics
cst-cli export-metrics --format json > metrics.json
```

### Grammar Management

```bash
# List installed grammars
cst-cli grammars list

# Update grammar
cst-cli grammars update rust

# Add new grammar
cst-cli grammars add tree-sitter-ruby

# Test grammar
cst-cli grammars test ruby sample.rb
```

## Security

### Access Control

1. **File Permissions**
   ```bash
   chmod 750 /opt/cst-tree-sitter
   chown -R cst-service:cst-service /data/cst
   ```

2. **Network Security**
   ```bash
   # Firewall rules
   ufw allow from 10.0.0.0/8 to any port 9000
   ufw deny 9000
   ```

### Audit Logging

```bash
# Enable audit logging
export CST_AUDIT_LOG=/var/log/cst-audit.log

# Audit log format
{
  "timestamp": "2024-01-08T10:00:00Z",
  "action": "cache_access",
  "user": "app-service",
  "resource": "file.rs",
  "result": "success"
}
```

## Disaster Recovery

### Failure Scenarios

1. **Memory Exhaustion**
   - Service auto-restarts with systemd
   - Check for memory leaks in logs
   - Reduce cache size temporarily

2. **Disk Full**
   - Clear old segments: `cst-cli cleanup --force`
   - Move frozen tier to larger disk
   - Enable compression: `CST_COMPRESS=true`

3. **Corruption**
   - Run integrity check: `cst-cli verify`
   - Clear corrupted segments
   - Restore from backup if needed

### Emergency Procedures

```bash
# Emergency cache clear
echo 3 > /proc/sys/vm/drop_caches
rm -rf /data/cst/segments/*

# Emergency restart
systemctl restart cst-tree-sitter --force

# Rollback to previous version
docker run -d cst-tree-sitter:stable
```
