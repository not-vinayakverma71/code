// Enhanced Approval System v2 - Production-grade with risk matrix and persistence
// Part of Approvals core TODO #5 - pre-IPC

use std::sync::Arc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};

// Risk assessment matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMatrix {
    tool_risks: HashMap<String, RiskProfile>,
    operation_modifiers: HashMap<String, f32>,
    path_patterns: Vec<PathRiskRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    pub base_risk: RiskLevel,
    pub factors: Vec<RiskFactor>,
    pub auto_approve_threshold: Option<RiskLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub name: String,
    pub weight: f32,
    pub condition: RiskCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskCondition {
    FileCount(usize),
    FileSize(u64),
    PathPattern(String),
    CommandPattern(String),
    NetworkAccess,
    SystemAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRiskRule {
    pub pattern: String,
    pub risk_modifier: f32,
    pub description: String,
}

/// Unified approval request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestV2 {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: String,
    
    // Operation details
    pub tool_name: String,
    pub operation: OperationType,
    pub scope: OperationScope,
    pub rationale: String,
    
    // Risk assessment
    pub risk_level: RiskLevel,
    pub risk_score: f32,
    pub risk_factors: Vec<String>,
    
    // Affected resources
    pub affected_resources: Vec<AffectedResource>,
    
    // Additional context
    pub metadata: HashMap<String, serde_json::Value>,
    pub preview: Option<String>,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OperationType {
    Read,
    Write,
    Delete,
    Execute,
    NetworkAccess,
    SystemModify,
    BulkOperation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationScope {
    pub workspace: PathBuf,
    pub paths: Vec<PathBuf>,
    pub is_recursive: bool,
    pub estimated_impact: ImpactEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEstimate {
    pub files_affected: usize,
    pub bytes_affected: u64,
    pub operations_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedResource {
    pub resource_type: ResourceType,
    pub path: Option<PathBuf>,
    pub action: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    File,
    Directory,
    Process,
    Network,
    SystemConfig,
    Environment,
}

/// Enhanced risk levels with numeric scoring
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    None = 0,      // No risk (pure read operations)
    Low = 1,       // Minimal risk (single file read)
    Medium = 2,    // Moderate risk (file modifications)
    High = 3,      // Significant risk (bulk operations)
    Critical = 4,  // Critical risk (system modifications)
    Extreme = 5,   // Extreme risk (irreversible operations)
}

impl RiskLevel {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s <= 0.0 => RiskLevel::None,
            s if s <= 1.0 => RiskLevel::Low,
            s if s <= 2.0 => RiskLevel::Medium,
            s if s <= 3.0 => RiskLevel::High,
            s if s <= 4.0 => RiskLevel::Critical,
            _ => RiskLevel::Extreme,
        }
    }
}

/// Approval decision with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecisionV2 {
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub decision: Decision,
    pub scope: ApprovalScope,
    pub conditions: Vec<ApprovalCondition>,
    pub expiry: Option<DateTime<Utc>>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    Approved,
    Denied,
    ApprovedWithConditions,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalScope {
    ThisOperation,
    ThisSession,
    ThisTool,
    ThisPattern(String),
    UntilExpiry,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalCondition {
    pub condition_type: ConditionType,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    MaxFiles(usize),
    MaxBytes(u64),
    PathRestriction(String),
    TimeWindow(u64), // seconds
    RequireBackup,
    DryRunFirst,
}

/// Persistent approval store
pub struct ApprovalStore {
    store_path: PathBuf,
    decisions: Arc<RwLock<HashMap<String, PersistedDecision>>>,
    patterns: Arc<RwLock<Vec<ApprovalPattern>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedDecision {
    decision: ApprovalDecisionV2,
    created_at: DateTime<Utc>,
    last_used: DateTime<Utc>,
    use_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApprovalPattern {
    pattern: String,
    decision: Decision,
    scope: ApprovalScope,
    created_at: DateTime<Utc>,
    expiry: Option<DateTime<Utc>>,
}

impl ApprovalStore {
    pub fn new(store_path: PathBuf) -> Result<Self> {
        let mut store = Self {
            store_path,
            decisions: Arc::new(RwLock::new(HashMap::new())),
            patterns: Arc::new(RwLock::new(Vec::new())),
        };
        store.load()?;
        Ok(store)
    }
    
    fn load(&mut self) -> Result<()> {
        if self.store_path.exists() {
            let content = std::fs::read_to_string(&self.store_path)?;
            let data: StoreData = serde_json::from_str(&content)?;
            
            *self.decisions.blocking_write() = data.decisions;
            *self.patterns.blocking_write() = data.patterns;
        }
        Ok(())
    }
    
    pub async fn save(&self) -> Result<()> {
        let data = StoreData {
            decisions: self.decisions.read().await.clone(),
            patterns: self.patterns.read().await.clone(),
            version: "2.0".to_string(),
        };
        
        let content = serde_json::to_string_pretty(&data)?;
        if let Some(parent) = self.store_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.store_path, content)?;
        Ok(())
    }
    
    pub async fn store_decision(&self, request_id: String, decision: ApprovalDecisionV2) -> Result<()> {
        let persisted = PersistedDecision {
            decision,
            created_at: Utc::now(),
            last_used: Utc::now(),
            use_count: 1,
        };
        
        self.decisions.write().await.insert(request_id, persisted);
        self.save().await?;
        Ok(())
    }
    
    pub async fn find_matching_decision(&self, request: &ApprovalRequestV2) -> Option<ApprovalDecisionV2> {
        // Check exact matches
        if let Some(persisted) = self.decisions.read().await.get(&request.id) {
            if let Some(expiry) = &persisted.decision.expiry {
                if *expiry > Utc::now() {
                    return Some(persisted.decision.clone());
                }
            } else {
                return Some(persisted.decision.clone());
            }
        }
        
        // Check patterns
        for pattern in self.patterns.read().await.iter() {
            if let Some(expiry) = &pattern.expiry {
                if *expiry <= Utc::now() {
                    continue;
                }
            }
            
            if self.matches_pattern(&request.tool_name, &pattern.pattern) {
                return Some(ApprovalDecisionV2 {
                    request_id: request.id.clone(),
                    timestamp: Utc::now(),
                    decision: pattern.decision.clone(),
                    scope: pattern.scope.clone(),
                    conditions: Vec::new(),
                    expiry: pattern.expiry,
                    rationale: Some("Matched pattern".to_string()),
                });
            }
        }
        
        None
    }
    
    fn matches_pattern(&self, tool_name: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return tool_name.starts_with(prefix);
        }
        
        tool_name == pattern
    }
    
    pub async fn clear_expired(&self) -> Result<()> {
        let now = Utc::now();
        
        // Remove expired decisions
        self.decisions.write().await.retain(|_, persisted| {
            if let Some(expiry) = &persisted.decision.expiry {
                *expiry > now
            } else {
                true
            }
        });
        
        // Remove expired patterns
        self.patterns.write().await.retain(|pattern| {
            if let Some(expiry) = &pattern.expiry {
                *expiry > now
            } else {
                true
            }
        });
        
        self.save().await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct StoreData {
    version: String,
    decisions: HashMap<String, PersistedDecision>,
    patterns: Vec<ApprovalPattern>,
}

/// Enhanced approval manager
pub struct ApprovalManagerV2 {
    risk_matrix: Arc<RiskMatrix>,
    store: Arc<ApprovalStore>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<ApprovalDecisionV2>>>>,
    ui_tx: Option<mpsc::UnboundedSender<ApprovalRequestV2>>,
}

impl ApprovalManagerV2 {
    pub fn new(store_path: PathBuf) -> Result<Self> {
        Ok(Self {
            risk_matrix: Arc::new(Self::default_risk_matrix()),
            store: Arc::new(ApprovalStore::new(store_path)?),
            pending: Arc::new(Mutex::new(HashMap::new())),
            ui_tx: None,
        })
    }
    
    fn default_risk_matrix() -> RiskMatrix {
        let mut tool_risks = HashMap::new();
        
        // Define risk profiles for each tool
        tool_risks.insert("read_file".to_string(), RiskProfile {
            base_risk: RiskLevel::None,
            factors: vec![],
            auto_approve_threshold: Some(RiskLevel::Low),
        });
        
        tool_risks.insert("write_file".to_string(), RiskProfile {
            base_risk: RiskLevel::Medium,
            factors: vec![
                RiskFactor {
                    name: "large_file".to_string(),
                    weight: 0.5,
                    condition: RiskCondition::FileSize(10 * 1024 * 1024),
                },
            ],
            auto_approve_threshold: None,
        });
        
        tool_risks.insert("execute_command".to_string(), RiskProfile {
            base_risk: RiskLevel::High,
            factors: vec![
                RiskFactor {
                    name: "system_command".to_string(),
                    weight: 1.0,
                    condition: RiskCondition::SystemAccess,
                },
                RiskFactor {
                    name: "network_access".to_string(),
                    weight: 0.8,
                    condition: RiskCondition::NetworkAccess,
                },
            ],
            auto_approve_threshold: None,
        });
        
        tool_risks.insert("apply_diff".to_string(), RiskProfile {
            base_risk: RiskLevel::Medium,
            factors: vec![
                RiskFactor {
                    name: "multi_file".to_string(),
                    weight: 0.5,
                    condition: RiskCondition::FileCount(5),
                },
            ],
            auto_approve_threshold: None,
        });
        
        let mut operation_modifiers = HashMap::new();
        operation_modifiers.insert("delete".to_string(), 1.5);
        operation_modifiers.insert("bulk".to_string(), 1.3);
        operation_modifiers.insert("system".to_string(), 2.0);
        
        let path_patterns = vec![
            PathRiskRule {
                pattern: ".*\\.env.*".to_string(),
                risk_modifier: 2.0,
                description: "Environment files contain secrets".to_string(),
            },
            PathRiskRule {
                pattern: ".*/\\.git/.*".to_string(),
                risk_modifier: 1.5,
                description: "Git internals are sensitive".to_string(),
            },
            PathRiskRule {
                pattern: ".*/node_modules/.*".to_string(),
                risk_modifier: 0.5,
                description: "Dependencies are lower risk".to_string(),
            },
        ];
        
        RiskMatrix {
            tool_risks,
            operation_modifiers,
            path_patterns,
        }
    }
    
    pub fn calculate_risk(&self, request: &mut ApprovalRequestV2) -> RiskLevel {
        let profile = self.risk_matrix.tool_risks
            .get(&request.tool_name)
            .cloned()
            .unwrap_or(RiskProfile {
                base_risk: RiskLevel::High,
                factors: vec![],
                auto_approve_threshold: None,
            });
        
        let mut score = profile.base_risk as u32 as f32;
        let mut factors = Vec::new();
        
        // Apply operation modifiers
        if let Some(modifier) = self.risk_matrix.operation_modifiers.get(&format!("{:?}", request.operation).to_lowercase()) {
            score *= modifier;
            factors.push(format!("Operation modifier: {}", modifier));
        }
        
        // Apply path patterns
        for resource in &request.affected_resources {
            if let Some(path) = &resource.path {
                for rule in &self.risk_matrix.path_patterns {
                    if regex::Regex::new(&rule.pattern).ok()
                        .and_then(|r| Some(r.is_match(&path.to_string_lossy())))
                        .unwrap_or(false) {
                        score *= rule.risk_modifier;
                        factors.push(format!("Path rule: {}", rule.description));
                    }
                }
            }
        }
        
        // Apply risk factors
        for factor in &profile.factors {
            // Check factor conditions
            let applies = match &factor.condition {
                RiskCondition::FileCount(count) => {
                    request.scope.estimated_impact.files_affected >= *count
                }
                RiskCondition::FileSize(size) => {
                    request.scope.estimated_impact.bytes_affected >= *size
                }
                _ => false,
            };
            
            if applies {
                score += factor.weight;
                factors.push(factor.name.clone());
            }
        }
        
        request.risk_score = score;
        request.risk_factors = factors;
        
        RiskLevel::from_score(score)
    }
    
    pub async fn request_approval(&self, mut request: ApprovalRequestV2) -> Result<ApprovalDecisionV2> {
        // Calculate risk
        request.risk_level = self.calculate_risk(&mut request);
        
        // Check persistent store for existing decision
        if let Some(decision) = self.store.find_matching_decision(&request).await {
            return Ok(decision);
        }
        
        // Auto-approve if below threshold
        if let Some(profile) = self.risk_matrix.tool_risks.get(&request.tool_name) {
            if let Some(threshold) = profile.auto_approve_threshold {
                if request.risk_level <= threshold {
                    let decision = ApprovalDecisionV2 {
                        request_id: request.id.clone(),
                        timestamp: Utc::now(),
                        decision: Decision::Approved,
                        scope: ApprovalScope::ThisOperation,
                        conditions: Vec::new(),
                        expiry: None,
                        rationale: Some("Auto-approved: below risk threshold".to_string()),
                    };
                    
                    self.store.store_decision(request.id.clone(), decision.clone()).await?;
                    return Ok(decision);
                }
            }
        }
        
        // Send to UI if available
        if let Some(tx) = &self.ui_tx {
            let (response_tx, response_rx) = oneshot::channel();
            
            self.pending.lock().await.insert(request.id.clone(), response_tx);
            tx.send(request.clone())?;
            
            match tokio::time::timeout(std::time::Duration::from_secs(30), response_rx).await {
                Ok(Ok(decision)) => {
                    self.store.store_decision(request.id.clone(), decision.clone()).await?;
                    Ok(decision)
                }
                _ => {
                    // Timeout or error - auto-deny
                    let decision = ApprovalDecisionV2 {
                        request_id: request.id.clone(),
                        timestamp: Utc::now(),
                        decision: Decision::Denied,
                        scope: ApprovalScope::ThisOperation,
                        conditions: Vec::new(),
                        expiry: None,
                        rationale: Some("Timeout or error".to_string()),
                    };
                    
                    self.store.store_decision(request.id, decision.clone()).await?;
                    Ok(decision)
                }
            }
        } else {
            // No UI - auto-deny high risk
            let decision = ApprovalDecisionV2 {
                request_id: request.id.clone(),
                timestamp: Utc::now(),
                decision: if request.risk_level <= RiskLevel::Low {
                    Decision::Approved
                } else {
                    Decision::Denied
                },
                scope: ApprovalScope::ThisOperation,
                conditions: Vec::new(),
                expiry: None,
                rationale: Some("No UI available".to_string()),
            };
            
            self.store.store_decision(request.id, decision.clone()).await?;
            Ok(decision)
        }
    }
    
    pub async fn handle_decision(&self, decision: ApprovalDecisionV2) -> Result<()> {
        let mut pending = self.pending.lock().await;
        if let Some(tx) = pending.remove(&decision.request_id) {
            let _ = tx.send(decision);
        }
        Ok(())
    }
}

// Add to Cargo.toml:
// regex = "1.10"
// chrono = { version = "0.4", features = ["serde"] }

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_risk_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("approvals.json");
        let manager = ApprovalManagerV2::new(store_path).unwrap();
        
        let mut request = ApprovalRequestV2 {
            id: "test-123".to_string(),
            timestamp: Utc::now(),
            correlation_id: "corr-123".to_string(),
            tool_name: "write_file".to_string(),
            operation: OperationType::Write,
            scope: OperationScope {
                workspace: PathBuf::from("/workspace"),
                paths: vec![PathBuf::from("test.txt")],
                is_recursive: false,
                estimated_impact: ImpactEstimate {
                    files_affected: 1,
                    bytes_affected: 1024,
                    operations_count: 1,
                },
            },
            rationale: "Test write".to_string(),
            risk_level: RiskLevel::None,
            risk_score: 0.0,
            risk_factors: Vec::new(),
            affected_resources: vec![
                AffectedResource {
                    resource_type: ResourceType::File,
                    path: Some(PathBuf::from("test.txt")),
                    action: "write".to_string(),
                    details: None,
                },
            ],
            metadata: HashMap::new(),
            preview: None,
            reversible: true,
        };
        
        let risk = manager.calculate_risk(&mut request);
        assert_eq!(risk, RiskLevel::Medium);
    }
    
    #[tokio::test]
    async fn test_approval_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("approvals.json");
        let store = ApprovalStore::new(store_path).unwrap();
        
        let decision = ApprovalDecisionV2 {
            request_id: "test-123".to_string(),
            timestamp: Utc::now(),
            decision: Decision::Approved,
            scope: ApprovalScope::ThisTool,
            conditions: Vec::new(),
            expiry: None,
            rationale: Some("Test approval".to_string()),
        };
        
        store.store_decision("test-123".to_string(), decision.clone()).await.unwrap();
        
        // Create request to find matching decision
        let request = ApprovalRequestV2 {
            id: "test-123".to_string(),
            timestamp: Utc::now(),
            correlation_id: "corr-123".to_string(),
            tool_name: "test_tool".to_string(),
            operation: OperationType::Write,
            scope: OperationScope {
                workspace: PathBuf::from("/workspace"),
                paths: vec![],
                is_recursive: false,
                estimated_impact: ImpactEstimate {
                    files_affected: 0,
                    bytes_affected: 0,
                    operations_count: 0,
                },
            },
            rationale: "Test".to_string(),
            risk_level: RiskLevel::Low,
            risk_score: 1.0,
            risk_factors: Vec::new(),
            affected_resources: Vec::new(),
            metadata: HashMap::new(),
            preview: None,
            reversible: true,
        };
        
        let found = store.find_matching_decision(&request).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().decision, Decision::Approved);
    }
}
