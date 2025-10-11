# Operator Runbook - CST Pipeline & Semantic Search

**Version**: 1.0  
**Last Updated**: 2025-10-11  
**On-Call**: #oncall-platform

## Quick Links

- **Metrics**: https://grafana/d/cst-pipeline
- **Logs**: `journalctl -u lapce-ai-service -f`
- **Alerts**: https://alertmanager/
- **Runbook**: You are here

## System Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Parser    │────▶│  CST Cache   │────▶│  Embedder   │
│ (CST/Legacy)│     │  (4-tier)    │     │ (AWS Titan) │
└─────────────┘     └──────────────┘     └─────────────┘
       │                    │                     │
       ▼                    ▼                     ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│Change Detect│     │  Metrics     │     │   LanceDB   │
│  (Stable ID)│     │(Prometheus)  │     │   Index     │
└─────────────┘     └──────────────┘     └─────────────┘
```

## Service Management

### Starting Services

```bash
# Start all services
systemctl start lapce-ai-service
systemctl start lapce-ai-indexer

# Check status
systemctl status lapce-ai-service
journalctl -u lapce-ai-service -n 50

# Verify health
curl http://localhost:8080/health | jq .
```

### Stopping Services

```bash
# Graceful shutdown (wait for in-flight requests)
systemctl stop lapce-ai-service

# Force stop (if hanging)
systemctl kill -s SIGKILL lapce-ai-service

# Stop indexing only
systemctl stop lapce-ai-indexer
```

### Restarting Services

```bash
# Graceful restart
systemctl restart lapce-ai-service

# Reload configuration (no downtime)
systemctl reload lapce-ai-service

# Emergency restart
systemctl restart lapce-ai-service --no-block
```

## Configuration

### Feature Flags

```bash
# /etc/lapce-ai/config.toml

[features]
cst_pipeline_enabled = true
rollout_percentage = 50
incremental_indexing = true
tiered_cache = true
embedding_reuse = true
legacy_fallback = true

[performance]
max_concurrent_parses = 8
cache_hot_size = 100
cache_warm_size = 500
cache_cold_size = 2000

[aws]
region = "us-east-1"
titan_tier = "standard"  # standard | express
max_batch_size = 25
requests_per_second = 5
```

### Reload Configuration

```bash
# Hot-reload (no restart)
systemctl reload lapce-ai-service

# Verify new config loaded
curl http://localhost:8080/config | jq .
```

### Environment Variables

```bash
# /etc/default/lapce-ai

CST_PIPELINE_ENABLED=true
CST_ROLLOUT_PERCENTAGE=50
AWS_REGION=us-east-1
LOG_LEVEL=info
RUST_BACKTRACE=1
```

## Monitoring

### Key Metrics

```bash
# Cache hit rate
curl -s http://localhost:9090/api/v1/query?query='rate(cst_cache_hits_total[5m])/(rate(cst_cache_hits_total[5m])+rate(cst_cache_misses_total[5m]))' | jq '.data.result[0].value[1]'

# P95 latency
curl -s http://localhost:9090/api/v1/query?query='histogram_quantile(0.95,rate(cst_cache_get_duration_seconds_bucket[5m]))' | jq '.data.result[0].value[1]'

# Error rate
curl -s http://localhost:9090/api/v1/query?query='rate(semantic_search_errors_total[5m])' | jq '.data.result[0].value[1]'

# Memory usage
curl -s http://localhost:9090/api/v1/query?query='semantic_search_memory_rss_bytes' | jq '.data.result[0].value[1]'
```

### Health Checks

```bash
# Service health
curl http://localhost:8080/health
# Expected: {"status": "healthy", "cst_enabled": true, "cache_hit_rate": 0.87}

# Detailed health
curl http://localhost:8080/health/detailed | jq .

# Readiness probe
curl http://localhost:8080/ready
# Returns 200 when ready to serve traffic

# Liveness probe  
curl http://localhost:8080/alive
# Returns 200 if service is running
```

### Log Analysis

```bash
# Tail logs
journalctl -u lapce-ai-service -f

# Search for errors
journalctl -u lapce-ai-service --since "1 hour ago" | grep -i error

# Parse errors by type
journalctl -u lapce-ai-service --since "1 hour ago" -o json | jq -r 'select(.level=="ERROR") | .error_type' | sort | uniq -c

# Performance issues
journalctl -u lapce-ai-service --since "10 minutes ago" | grep -E "latency|slow|timeout"

# Cache misses
journalctl -u lapce-ai-service | grep "cache_miss" | tail -20
```

## Common Issues

### Issue: High Error Rate

**Symptoms**: Error rate >1%, alerts firing  
**Diagnosis**:
```bash
# Check error types
curl http://localhost:8080/metrics | grep semantic_search_errors_total

# Check logs
journalctl -u lapce-ai-service --since "10 minutes ago" | grep ERROR

# Check AWS credentials
aws sts get-caller-identity
```

**Resolution**:
1. If AWS credential errors: Check env vars, rotate if expired
2. If parse errors: Check CST rollout percentage, consider rollback
3. If rate limit errors: Reduce `requests_per_second` in config

### Issue: Low Cache Hit Rate

**Symptoms**: Cache hit rate <70%  
**Diagnosis**:
```bash
# Check cache stats
curl http://localhost:8080/cache/stats | jq .

# Check tier distribution
curl http://localhost:8080/metrics | grep cst_cache_tier_size

# Check eviction rate
curl http://localhost:8080/metrics | grep cache_evictions
```

**Resolution**:
```bash
# Increase cache sizes
vim /etc/lapce-ai/config.toml
# cache_hot_size = 200 (was 100)
# cache_warm_size = 1000 (was 500)

systemctl reload lapce-ai-service
```

### Issue: High Memory Usage

**Symptoms**: Memory >3GB, alerts firing  
**Diagnosis**:
```bash
# Check current RSS
ps aux | grep lapce-ai-service | awk '{print $6/1024 " MB"}'

# Check memory growth
curl http://localhost:9090/api/v1/query?query='rate(semantic_search_memory_rss_bytes[30m])'

# Check for leaks
./scripts/check_memory_leak.sh
```

**Resolution**:
```bash
# If memory leak suspected
systemctl restart lapce-ai-service

# If legitimate high usage, tune cache
vim /etc/lapce-ai/config.toml
# Reduce cache sizes

# If persists, capture heap dump
./scripts/capture_heap_dump.sh
```

### Issue: Slow Indexing

**Symptoms**: Indexing throughput <500 files/min  
**Diagnosis**:
```bash
# Check indexing rate
curl http://localhost:8080/metrics | grep index_operations_total

# Check AWS Titan latency
curl http://localhost:8080/metrics | grep aws_titan_request_latency

# Check queue depth
curl http://localhost:8080/indexer/stats | jq .queue_depth
```

**Resolution**:
```bash
# Increase concurrency
vim /etc/lapce-ai/config.toml
# max_concurrent_parses = 16 (was 8)

# Or upgrade AWS Titan tier
# titan_tier = "express" (was "standard")

systemctl reload lapce-ai-service
```

### Issue: Service Won't Start

**Symptoms**: systemctl start fails  
**Diagnosis**:
```bash
# Check status
systemctl status lapce-ai-service

# Check recent logs
journalctl -u lapce-ai-service --since "5 minutes ago"

# Check port conflicts
ss -tulpn | grep 8080

# Check config syntax
./lapce-ai-service --check-config
```

**Resolution**:
```bash
# If port conflict
systemctl stop <conflicting-service>

# If config invalid
vim /etc/lapce-ai/config.toml
./lapce-ai-service --check-config

# If permissions issue
chown -R lapce-ai:lapce-ai /var/lib/lapce-ai
chmod 755 /var/lib/lapce-ai

# Try starting again
systemctl start lapce-ai-service
```

## Performance Tuning

### Cache Optimization

```toml
# High-read workload
cache_hot_size = 200
cache_warm_size = 1000
cache_cold_size = 5000

# Low-memory environment
cache_hot_size = 50
cache_warm_size = 200
cache_cold_size = 500

# Balanced (default)
cache_hot_size = 100
cache_warm_size = 500
cache_cold_size = 2000
```

### Parsing Concurrency

```toml
# High-CPU system (16+ cores)
max_concurrent_parses = 32

# Medium system (8 cores)
max_concurrent_parses = 8

# Low resources (4 cores)
max_concurrent_parses = 4
```

### AWS Titan Settings

```toml
# Cost-optimized
titan_tier = "standard"
requests_per_second = 5
max_batch_size = 25

# Performance-optimized
titan_tier = "express"
requests_per_second = 20
max_batch_size = 50
```

## Deployment

### Deploy New Version

```bash
# 1. Backup current state
./scripts/backup_state.sh

# 2. Stop services
systemctl stop lapce-ai-service lapce-ai-indexer

# 3. Deploy new binary
cp /tmp/lapce-ai-service /opt/lapce-ai/
chmod +x /opt/lapce-ai/lapce-ai-service

# 4. Run migrations if needed
./lapce-index migrate --index /var/lib/lapce-ai/indices --to latest

# 5. Start services
systemctl start lapce-ai-service lapce-ai-indexer

# 6. Verify
./scripts/smoke_test.sh
```

### Rollback Deployment

```bash
# Quick rollback
./scripts/rollback_cst.sh --type binary --version previous

# Manual rollback
systemctl stop lapce-ai-service
cp /opt/lapce-ai/backups/lapce-ai-service.previous /opt/lapce-ai/lapce-ai-service
systemctl start lapce-ai-service
```

## Backup & Restore

### Backup

```bash
# Backup indices
./scripts/backup_index.sh /var/lib/lapce-ai/indices

# Backup configuration
tar czf config-backup-$(date +%Y%m%d).tar.gz /etc/lapce-ai/

# Backup cache (optional, can be rebuilt)
tar czf cache-backup-$(date +%Y%m%d).tar.gz /var/lib/lapce-ai/cache/
```

### Restore

```bash
# Restore indices
./scripts/restore_index.sh --backup /backups/indices-20251011.tar.gz

# Restore configuration
tar xzf config-backup-20251011.tar.gz -C /

# Restart services
systemctl restart lapce-ai-service
```

### Disaster Recovery

```bash
# Full system rebuild
# 1. Stop services
systemctl stop lapce-ai-service lapce-ai-indexer

# 2. Clear corrupted data
trash-put /var/lib/lapce-ai/indices/*
trash-put /var/lib/lapce-ai/cache/*

# 3. Restore from backup or rebuild
./scripts/restore_from_backup.sh --date 20251011
# OR
./scripts/rebuild_index.sh --from-source /path/to/source

# 4. Verify integrity
./lapce-index validate --index /var/lib/lapce-ai/indices

# 5. Start services
systemctl start lapce-ai-service lapce-ai-indexer
```

## Security

### Credentials Management

```bash
# AWS credentials stored in env
cat /etc/default/lapce-ai | grep AWS

# Rotate credentials
./scripts/rotate_aws_credentials.sh

# Verify new credentials
aws sts get-caller-identity
systemctl restart lapce-ai-service
```

### Access Control

```bash
# Service runs as lapce-ai user
ps aux | grep lapce-ai-service

# Data directory permissions
ls -la /var/lib/lapce-ai/
# Should be: drwxr-x--- lapce-ai lapce-ai

# Fix permissions if needed
chown -R lapce-ai:lapce-ai /var/lib/lapce-ai
chmod 750 /var/lib/lapce-ai
```

## Maintenance

### Daily Tasks

- Check monitoring dashboard
- Review error logs
- Verify backup completed
- Check disk usage

```bash
# Daily check script
./scripts/daily_health_check.sh
```

### Weekly Tasks

- Review performance trends
- Analyze cache effectiveness
- Check for updates
- Test rollback procedures

```bash
# Weekly report
./scripts/weekly_report.sh --email ops@example.com
```

### Monthly Tasks

- Index validation and optimization
- Configuration review
- Capacity planning
- Disaster recovery drill

```bash
# Monthly maintenance
./scripts/monthly_maintenance.sh
```

## Troubleshooting Commands

```bash
# Quick diagnostics
./scripts/diagnose.sh

# Full system check
./scripts/system_check.sh --verbose

# Performance profile
./scripts/perf_profile.sh --duration 60s

# Cache analysis
./scripts/analyze_cache.sh --detailed

# Memory analysis
./scripts/memory_profile.sh --heap-dump

# Network check
./scripts/check_connectivity.sh --aws --prometheus
```

## Escalation

### When to Escalate

- **Immediate**: Service down >5 minutes, data corruption
- **Urgent**: Error rate >5%, P0 alerts firing
- **High**: Performance degradation >50%, memory leak
- **Normal**: Non-urgent questions, enhancement requests

### Escalation Path

1. **On-Call Engineer**: #oncall-platform, PagerDuty
2. **Engineering Lead**: @eng-lead (if >15 min)
3. **Director**: @director-eng (if >1 hour)
4. **CTO**: @cto (if customer impact)

## References

- [Architecture Docs](./ARCHITECTURE.md)
- [Security Policy](./SECURITY_VALIDATION_REPORT.md)
- [Performance SLOs](./SLO_VERIFICATION.md)
- [Rollback Procedures](./ROLLBACK_PROCEDURE.md)
- [Canary Rollout](./CANARY_ROLLOUT_PLAN.md)

---

**Questions?** Ask in #platform-team  
**Emergency?** Run `./scripts/emergency_help.sh`
