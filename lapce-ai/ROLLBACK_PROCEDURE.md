# Rollback Procedure - CST Pipeline & Semantic Search

**Version**: 1.0  
**Date**: 2025-10-11  
**Recovery Time Objective (RTO)**: <10 minutes  
**Recovery Point Objective (RPO)**: <5 minutes

## Quick Reference

### Emergency Rollback Commands

```bash
# IMMEDIATE: Disable CST pipeline (no restart required)
export CST_PIPELINE_ENABLED=false
export CST_ROLLOUT_PERCENTAGE=0
systemctl reload lapce-ai-service

# Verify rollback
curl http://localhost:8080/health | jq '.cst_enabled'  # Should be false

# If service reload fails, full restart
systemctl restart lapce-ai-service
```

### Rollback Decision Matrix

| Severity | Trigger | Action | Approval |
|----------|---------|--------|----------|
| P0: Critical | Data corruption, service down | Immediate rollback | None (auto) |
| P1: High | Error rate >2%, P95 >2s | Rollback within 15min | On-call lead |
| P2: Medium | Error rate >0.5% | Investigate 30min, then rollback | Engineering lead |
| P3: Low | Performance <10% degraded | Monitor, rollback if worsens | Team decision |

## Rollback Types

### Type 1: Configuration Rollback (Fastest - 2 minutes)

**Use Case**: Feature flag issues, percentage adjustment  
**Downtime**: None (hot reload)  
**Data Loss**: None

**Steps**:
```bash
# 1. Set rollback percentage
./scripts/set_cst_percentage.sh 0

# 2. Reload configuration
systemctl reload lapce-ai-service

# 3. Verify
./scripts/verify_rollback.sh

# 4. Monitor for 5 minutes
watch -n 10 './scripts/check_error_rate.sh'
```

### Type 2: Binary Rollback (Fast - 5 minutes)

**Use Case**: Critical bugs in CST implementation  
**Downtime**: ~30 seconds during restart  
**Data Loss**: In-flight requests only

**Steps**:
```bash
# 1. Stop service
systemctl stop lapce-ai-service

# 2. Rollback binary to previous version
cd /opt/lapce-ai
mv lapce-ai-service lapce-ai-service.broken
cp backups/lapce-ai-service.previous lapce-ai-service
chmod +x lapce-ai-service

# 3. Start service with CST disabled
export CST_PIPELINE_ENABLED=false
systemctl start lapce-ai-service

# 4. Verify health
./scripts/health_check.sh --timeout 60

# 5. Restore traffic
./scripts/restore_load_balancer.sh
```

### Type 3: Data Rollback (Slow - 30-60 minutes)

**Use Case**: Index corruption, cache poisoning  
**Downtime**: 5-15 minutes  
**Data Loss**: Up to last backup (typically <1 hour)

**Steps**:
```bash
# 1. Stop indexing service
systemctl stop lapce-ai-indexer

# 2. Backup corrupted state
cd /var/lib/lapce-ai
tar czf corrupted-$(date +%Y%m%d-%H%M%S).tar.gz indices/ cache/
mv indices/ indices.corrupted/
mv cache/ cache.corrupted/

# 3. Restore from last known good backup
./scripts/restore_from_backup.sh --timestamp latest

# 4. Validate data integrity
./scripts/validate_indices.sh
./scripts/validate_cache.sh

# 5. Restart services
systemctl start lapce-ai-indexer
systemctl start lapce-ai-service

# 6. Re-index recent changes
./scripts/reindex_since.sh --since "1 hour ago"
```

## Pre-Rollback Checklist

Before executing rollback:

- [ ] Confirm issue severity (P0/P1/P2/P3)
- [ ] Capture current metrics snapshot
- [ ] Notify team in #incidents channel
- [ ] Create incident ticket
- [ ] Document symptoms and trigger

## Rollback Execution

### Phase 1: Preparation (1 minute)

```bash
# 1. Capture state
./scripts/capture_rollback_state.sh > /tmp/rollback-state-$(date +%s).json

# 2. Alert stakeholders
./scripts/alert_team.sh "Starting rollback: $(reason)"

# 3. Mark in monitoring
curl -X POST http://monitoring/api/annotations \
  -d '{"text": "Rollback started", "tags": ["rollback", "cst"]}'
```

### Phase 2: Execute Rollback (2-5 minutes)

**Configuration rollback**:
```bash
# Automated script handles all steps
./scripts/rollback_cst.sh --type config --verify
```

**Binary rollback**:
```bash
./scripts/rollback_cst.sh --type binary --version previous --verify
```

**Data rollback**:
```bash
./scripts/rollback_cst.sh --type data --backup latest --verify
```

### Phase 3: Verification (2 minutes)

```bash
# Automated verification suite
./scripts/verify_rollback.sh --full

# Manual checks
curl http://localhost:8080/health
curl http://localhost:8080/metrics | grep cst_enabled

# Check error rate
./scripts/check_error_rate.sh --last 5m
```

### Phase 4: Monitoring (5+ minutes)

```bash
# Watch key metrics
watch -n 10 './scripts/rollback_health.sh'

# Monitor alerts
tail -f /var/log/lapce-ai/alerts.log

# After 5 minutes of stability, declare rollback successful
```

## Verification Criteria

Rollback is successful when:

- [ ] Service health endpoint returns 200 OK
- [ ] CST pipeline disabled (or rolled back to previous %)
- [ ] Error rate returns to baseline (<0.1%)
- [ ] P95 latency <1s
- [ ] No active alerts
- [ ] Cache hit rate >70%
- [ ] Memory stable (<3GB)
- [ ] 5 minutes of stable operation

## Post-Rollback Actions

### Immediate (Within 1 hour)

1. **Update incident ticket** with rollback details
2. **Notify stakeholders** of resolution
3. **Preserve logs and metrics** for analysis
   ```bash
   ./scripts/preserve_rollback_data.sh --incident ID-123
   ```
4. **Schedule postmortem** within 24 hours

### Short-term (Within 24 hours)

1. **Root cause analysis**
   - Review metrics leading up to issue
   - Analyze error logs
   - Reproduce issue in staging if possible
   
2. **Document findings** in incident report

3. **Create action items** to prevent recurrence

4. **Update runbooks** if new scenarios discovered

### Medium-term (Within 1 week)

1. **Implement fixes** for root cause
2. **Test fixes** in staging environment
3. **Plan retry** of rollout (if applicable)
4. **Share lessons learned** with team

## Rollback Scenarios

### Scenario 1: High Error Rate

**Symptoms**: Error rate spikes to 5%  
**Trigger**: Automated alert  
**Action**: Configuration rollback to 0%

```bash
# Execute
./scripts/rollback_cst.sh --type config --percentage 0 --reason "error_rate_spike"

# Verify
./scripts/check_error_rate.sh --expect <0.5%

# Monitor
watch -n 10 './scripts/monitor_recovery.sh'
```

### Scenario 2: Memory Leak

**Symptoms**: Memory grows 50MB/min  
**Trigger**: Automated alert  
**Action**: Binary rollback + service restart

```bash
# Execute
./scripts/rollback_cst.sh --type binary --restart --reason "memory_leak"

# Verify memory stable
watch -n 5 'ps aux | grep lapce-ai-service | grep -v grep | awk "{print \$6}"'

# Capture heap dump for analysis
./scripts/capture_heap_dump.sh --pid $(pidof lapce-ai-service)
```

### Scenario 3: Index Corruption

**Symptoms**: Search returns corrupt results, validation errors  
**Trigger**: Manual detection  
**Action**: Data rollback + full re-index

```bash
# Execute
./scripts/rollback_cst.sh --type data --backup latest --reason "index_corruption"

# Validate
./scripts/validate_indices.sh --full

# Re-index if needed
./scripts/reindex_all.sh --concurrency 4 --verify
```

### Scenario 4: Performance Degradation

**Symptoms**: P95 latency increases from 200ms to 1.5s  
**Trigger**: Manual investigation  
**Action**: Gradual rollback (50% → 25% → 10% → 0%)

```bash
# Reduce by 50% first
./scripts/set_cst_percentage.sh 25
sleep 300  # Wait 5 minutes

# Check if improved
if ./scripts/check_latency.sh --threshold 1.0; then
    echo "Latency improved at 25%"
else
    ./scripts/set_cst_percentage.sh 0
    echo "Full rollback to 0%"
fi
```

## Rollback Automation

### Automatic Rollback Triggers

Configured in monitoring system:

```yaml
# prometheus_alerts.yml
- alert: AutoRollbackTrigger
  expr: |
    (rate(cst_errors_total[2m]) / rate(cst_requests_total[2m])) > 0.05
    OR
    histogram_quantile(0.95, rate(cst_latency_bucket[2m])) > 2.0
    OR
    rate(cst_memory_rss_bytes[1m]) > 52428800  # 50MB/min
  for: 2m
  labels:
    severity: critical
    action: auto_rollback
  annotations:
    summary: "Triggering automatic rollback"
    script: "/opt/lapce-ai/scripts/auto_rollback.sh"
```

### Auto-Rollback Script

```bash
#!/bin/bash
# auto_rollback.sh

set -e
INCIDENT_ID=$(generate_incident_id)

log() {
    echo "[$(date)] $*" | tee -a /var/log/lapce-ai/rollback.log
}

log "AUTO-ROLLBACK triggered: $ALERT_NAME"
log "Incident ID: $INCIDENT_ID"

# Capture state before rollback
./scripts/capture_rollback_state.sh > /tmp/rollback-$INCIDENT_ID.json

# Execute rollback
./scripts/rollback_cst.sh --type config --percentage 0 --reason "auto_rollback_$ALERT_NAME"

# Verify
if ./scripts/verify_rollback.sh; then
    log "✅ Auto-rollback successful"
    ./scripts/alert_team.sh "AUTO-ROLLBACK successful - Incident $INCIDENT_ID"
else
    log "❌ Auto-rollback FAILED"
    ./scripts/alert_team.sh "CRITICAL: AUTO-ROLLBACK FAILED - Incident $INCIDENT_ID"
    exit 1
fi

# Create incident ticket
./scripts/create_incident.sh --id $INCIDENT_ID --type auto_rollback --alert "$ALERT_NAME"
```

## Testing Rollback Procedures

### Monthly Rollback Drills

```bash
# Simulate rollback in staging
./scripts/rollback_drill.sh --environment staging --type config

# Verify drill success
./scripts/verify_drill.sh

# Document drill results
./scripts/generate_drill_report.sh > reports/drill-$(date +%Y%m).md
```

### Chaos Engineering

```bash
# Inject failures to test rollback
./scripts/chaos_test.sh --inject error_rate --rate 0.1 --duration 5m
./scripts/chaos_test.sh --inject latency --p95 2.0 --duration 5m
./scripts/chaos_test.sh --inject memory_leak --rate 100MB/min --duration 2m

# Verify automatic rollback triggered correctly
./scripts/verify_auto_rollback.sh
```

## Contact Information

### Escalation Path

1. **On-Call Engineer** (immediate): Slack #oncall, PagerDuty
2. **Engineering Lead** (if >15min): @engineering-lead
3. **Engineering Director** (if >1hr): @director-eng
4. **CTO** (if service down >2hr): @cto

### Runbook Contacts

- **Runbook Owner**: @platform-team
- **Last Updated**: 2025-10-11
- **Next Review**: 2025-11-11

## Appendix

### Rollback Scripts Reference

| Script | Purpose | Runtime |
|--------|---------|---------|
| `rollback_cst.sh` | Main rollback orchestration | 2-5min |
| `verify_rollback.sh` | Verification suite | 1-2min |
| `capture_rollback_state.sh` | Pre-rollback snapshot | <30s |
| `restore_from_backup.sh` | Data restoration | 10-30min |
| `auto_rollback.sh` | Automated rollback handler | 2-5min |
| `rollback_drill.sh` | Test rollback procedures | 5-10min |

### Backup Locations

```
/var/lib/lapce-ai/backups/
├── binaries/
│   ├── lapce-ai-service.v1.2.3
│   ├── lapce-ai-service.v1.2.2
│   └── lapce-ai-service.v1.2.1
├── data/
│   ├── indices-2025-10-11-10-00.tar.gz
│   ├── indices-2025-10-11-11-00.tar.gz
│   └── cache-2025-10-11-10-00.tar.gz
└── configs/
    ├── config-2025-10-11-v1.2.3.toml
    └── config-2025-10-11-v1.2.2.toml
```

### Monitoring Dashboards

- **Main Dashboard**: https://grafana/d/cst-pipeline
- **Rollback Dashboard**: https://grafana/d/rollback-status
- **Incident Timeline**: https://grafana/d/incidents

---

**Emergency Hotline**: Run `./scripts/emergency_help.sh` for instant guidance
