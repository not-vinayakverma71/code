# Canary Rollout Plan - CST Pipeline & Semantic Search

**Version**: 1.0  
**Date**: 2025-10-11  
**Status**: Ready for execution pending LanceDB resolution

## Overview

Staged rollout strategy with feature flags, monitoring gates, and automated rollback triggers.

## Feature Flags

### CST Pipeline Feature Flags

```rust
// Feature flag configuration
pub struct FeatureFlags {
    /// Enable CST pipeline for parsing
    pub enable_cst_pipeline: bool,
    
    /// Enable incremental indexing with stable IDs
    pub enable_incremental_indexing: bool,
    
    /// Enable 4-tier cache system
    pub enable_tiered_cache: bool,
    
    /// Enable embedding reuse via change detection
    pub enable_embedding_reuse: bool,
    
    /// Rollout percentage (0-100)
    pub rollout_percentage: u8,
}
```

### Environment Variable Controls

```bash
# Feature flag env vars
export CST_PIPELINE_ENABLED=true           # Master switch
export CST_ROLLOUT_PERCENTAGE=10           # 0-100
export CST_INCREMENTAL_INDEXING=true       # Incremental indexing
export CST_TIERED_CACHE=true               # 4-tier cache
export CST_EMBEDDING_REUSE=true            # Change detection reuse
export CST_LEGACY_FALLBACK=true            # Auto-fallback on errors
```

### Runtime Toggle

```rust
// Hot-reload configuration (no restart required)
impl FeatureFlags {
    pub fn reload_from_env() -> Self {
        Self {
            enable_cst_pipeline: env::var("CST_PIPELINE_ENABLED")
                .unwrap_or("false".into()) == "true",
            rollout_percentage: env::var("CST_ROLLOUT_PERCENTAGE")
                .unwrap_or("0".into()).parse().unwrap_or(0),
            // ... other flags
        }
    }
}
```

## Rollout Stages

### Stage 0: Pre-Production (Week 0)

**Deployment**: Internal staging environment  
**Percentage**: 100% CST pipeline  
**Duration**: 1 week  
**Success Criteria**:
- ✅ All tests passing
- ✅ Zero crashes in 7 days
- ✅ Performance SLOs met
- ✅ Memory stable (<3GB)

**Monitoring**:
```yaml
alerts:
  - error_rate < 0.1%
  - cache_hit_rate > 80%
  - p95_latency < 1s
  - memory_growth_rate < 5MB/hour
```

**Go/No-Go Decision**: Manual review of metrics

---

### Stage 1: Canary (Week 1)

**Deployment**: Production - 10% traffic  
**Percentage**: 10% CST pipeline, 90% legacy  
**Duration**: 48 hours  
**User Selection**: Random sampling

**Success Criteria**:
- Error rate <0.1% higher than legacy
- Zero P0 incidents
- Cache hit rate >80%
- Change detection latency <1ms/1k nodes
- No memory leaks detected

**Monitoring Gates**:
```rust
struct CanaryGate {
    max_error_rate_delta: f64,    // 0.1%
    max_latency_p95_delta: f64,   // 100ms
    min_cache_hit_rate: f64,      // 80%
    max_memory_growth: u64,       // 10MB/min
}
```

**Rollback Triggers** (automatic):
- Error rate >0.5% higher than legacy
- P95 latency >2x legacy
- Cache hit rate <70%
- Any P0 incident
- Memory growth >50MB/min

**Actions on failure**:
1. Auto-rollback to 0%
2. Alert on-call
3. Create incident report
4. Preserve metrics for analysis

---

### Stage 2: Expanded Canary (Week 1-2)

**Deployment**: Production - 25% traffic  
**Percentage**: 25% CST pipeline, 75% legacy  
**Duration**: 72 hours  

**Success Criteria**:
- All Stage 1 criteria met
- 72 hours error-free operation
- Performance metrics stable
- User feedback neutral/positive

**Additional Monitoring**:
- Per-language performance tracking
- Embedding reuse rate >85%
- Index corruption checks

---

### Stage 3: Majority Rollout (Week 2-3)

**Deployment**: Production - 50% traffic  
**Percentage**: 50% CST pipeline, 50% legacy  
**Duration**: 1 week  

**Success Criteria**:
- All previous criteria met
- Cost analysis favorable
- No degradation at scale
- Support tickets unchanged

**Performance Validation**:
```bash
# Compare CST vs Legacy
./scripts/compare_pipeline_performance.sh \
  --cst-percentage 50 \
  --duration 7d \
  --threshold 0.05  # 5% acceptable variance
```

---

### Stage 4: Full Rollout (Week 3-4)

**Deployment**: Production - 100% traffic  
**Percentage**: 100% CST pipeline  
**Duration**: Indefinite  

**Success Criteria**:
- All previous criteria met
- 2 weeks stable operation
- Legacy fallback still available
- Runbook validated

**Legacy Deprecation Plan**:
- Week 4-5: 100% CST, legacy code retained
- Week 6-8: Monitor for edge cases
- Week 9+: Remove legacy code (after final review)

---

## Monitoring Dashboard

### Required Metrics Per Stage

**Core Metrics**:
```promql
# Error rate comparison
rate(cst_errors_total[5m]) / rate(cst_requests_total[5m]) vs
rate(legacy_errors_total[5m]) / rate(legacy_requests_total[5m])

# Latency comparison
histogram_quantile(0.95, rate(cst_latency_bucket[5m])) vs
histogram_quantile(0.95, rate(legacy_latency_bucket[5m]))

# Cache effectiveness
rate(cst_cache_hits_total[5m]) / 
  (rate(cst_cache_hits_total[5m]) + rate(cst_cache_misses_total[5m]))

# Memory tracking
cst_memory_rss_bytes
```

### Stage-Specific Panels

**Stage 1 (Canary)**:
- Side-by-side CST vs Legacy comparison
- Error rate delta graph
- Automatic rollback status indicator

**Stage 2+**:
- Traffic split visualization
- Per-language breakdown
- Cost analysis (compute time, API calls)

### Alert Configuration Per Stage

**Stage 1 (Strictest)**:
```yaml
- alert: CanaryErrorRateHigh
  expr: |
    (rate(cst_errors_total[5m]) / rate(cst_requests_total[5m])) > 
    (rate(legacy_errors_total[5m]) / rate(legacy_requests_total[5m]) + 0.005)
  for: 2m
  labels:
    severity: critical
    action: auto_rollback
```

**Stage 4 (Relaxed)**:
```yaml
- alert: CSTErrorRateHigh
  expr: rate(cst_errors_total[5m]) / rate(cst_requests_total[5m]) > 0.01
  for: 5m
  labels:
    severity: warning
```

## Rollback Procedures

### Automatic Rollback

Triggered by monitoring gates:
```bash
#!/bin/bash
# auto_rollback.sh
set -e

echo "Triggering automatic rollback..."
export CST_ROLLOUT_PERCENTAGE=0
systemctl reload lapce-ai-service

# Verify rollback
sleep 5
if [ "$(get_cst_percentage)" -eq 0 ]; then
    echo "✅ Rollback successful"
    alert_oncall "AUTO_ROLLBACK: CST pipeline rolled back"
else
    echo "❌ Rollback failed"
    alert_oncall "CRITICAL: AUTO_ROLLBACK FAILED"
    exit 1
fi
```

### Manual Rollback

```bash
# Emergency rollback command
./scripts/rollback_cst_pipeline.sh --stage <current_stage>

# Validates:
# 1. Current stage and rollout percentage
# 2. Preserves metrics for analysis
# 3. Notifies team
# 4. Creates incident ticket
```

### Rollback SLA

- **Detection**: <2 minutes (automated monitoring)
- **Decision**: <5 minutes (auto) or <15 minutes (manual)
- **Execution**: <2 minutes (env var change)
- **Verification**: <5 minutes
- **Total**: <10 minutes (auto) or <25 minutes (manual)

## Risk Mitigation

### Pre-Rollout Checklist

- [ ] All unit tests passing (100%)
- [ ] Integration tests passing
- [ ] Security scan clean
- [ ] Performance benchmarks meet SLOs
- [ ] Runbook reviewed and validated
- [ ] On-call team briefed
- [ ] Rollback procedure tested
- [ ] Metrics dashboard configured
- [ ] Alert rules validated
- [ ] Feature flags tested
- [ ] Staging environment stable for 7 days

### During Rollout

- [ ] Monitor dashboard every 15 minutes
- [ ] Check alert channels
- [ ] Review error logs hourly
- [ ] Validate cache hit rates
- [ ] Track memory usage
- [ ] Monitor user feedback channels

### Post-Rollout

- [ ] Analyze performance vs baseline
- [ ] Review all incidents
- [ ] Update documentation
- [ ] Cost analysis
- [ ] User satisfaction survey
- [ ] Lessons learned session

## Communication Plan

### Stakeholders

1. **Engineering Team**: Daily standup updates
2. **On-Call Rotation**: Alert on stage transitions
3. **Management**: Weekly rollout report
4. **Users**: Release notes (transparent about features)

### Notification Schedule

**Stage Transitions**:
```
[ROLLOUT] CST Pipeline Stage 2: 25% traffic
- Started: 2025-10-XX 10:00 UTC
- Duration: 72 hours
- Monitoring: https://grafana/cst-rollout
- Rollback: ./scripts/rollback_cst_pipeline.sh --stage 2
```

**Incidents**:
```
[INCIDENT] CST Pipeline Auto-Rollback
- Trigger: Error rate exceeded threshold
- Stage: 2 (25% traffic)
- Action: Rolled back to 0%
- Status: Investigating
- ETA: 2 hours to resolution
```

## Success Metrics

### Quantitative

- Error rate delta: <0.1% vs legacy
- P95 latency: <1s (target met)
- Cache hit rate: >80% (target met)
- Embedding reuse: >85% (target met)
- Memory usage: <3GB (target met)
- Zero P0 incidents
- Rollback count: 0 (goal)

### Qualitative

- User feedback: Neutral to positive
- Support ticket volume: Unchanged
- Team confidence: High
- Documentation: Complete and accurate

## Post-Rollout Actions

### Week 4 (100% Rollout Complete)

- [ ] Performance analysis report
- [ ] Cost comparison vs legacy
- [ ] User satisfaction survey
- [ ] Incident review and RCA
- [ ] Update architectural docs
- [ ] Knowledge transfer sessions

### Week 8 (Legacy Deprecation Planning)

- [ ] Identify remaining legacy dependencies
- [ ] Plan code removal
- [ ] Update CI/CD pipelines
- [ ] Archive legacy documentation

### Week 12 (Legacy Removal)

- [ ] Remove legacy code
- [ ] Update build systems
- [ ] Simplify configuration
- [ ] Celebrate success 🎉

## Contingency Plans

### Scenario: Performance Degradation

**Trigger**: P95 latency increases 20%  
**Action**:
1. Investigate via metrics (slow query? cache issue?)
2. If resolved in <1 hour: Continue
3. If not: Roll back to previous stage
4. Analyze root cause offline

### Scenario: Memory Leak Detected

**Trigger**: Memory growth >10MB/min sustained  
**Action**:
1. Immediate rollback to 0%
2. Capture heap snapshot
3. Analyze leak offline
4. Fix and re-test in staging before retry

### Scenario: Data Corruption

**Trigger**: Index validation failures  
**Action**:
1. **IMMEDIATE** rollback to 0%
2. Halt all indexing operations
3. Validate data integrity
4. Restore from backup if needed
5. Full incident review before retry

## Approval Gates

### Stage 1 → Stage 2
- Engineering lead approval
- 48 hours incident-free
- Metrics within thresholds

### Stage 2 → Stage 3
- Engineering lead approval
- Product owner sign-off
- 72 hours stable operation

### Stage 3 → Stage 4
- Engineering director approval
- Cost analysis approved
- 1 week stable operation
- User feedback reviewed

## Timeline Summary

| Stage | Percentage | Duration | Total Days |
|-------|-----------|----------|------------|
| 0: Pre-Prod | 100% (staging) | 7 days | 0-7 |
| 1: Canary | 10% | 48 hours | 7-9 |
| 2: Expanded | 25% | 72 hours | 9-12 |
| 3: Majority | 50% | 7 days | 12-19 |
| 4: Full | 100% | Indefinite | 19+ |

**Total rollout time**: ~3 weeks (19 days)

---

**Last Updated**: 2025-10-11  
**Next Review**: Before Stage 1 deployment  
**Owner**: Engineering Team
