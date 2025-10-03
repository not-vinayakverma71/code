/// Health Check System for MCP Tools
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

pub struct HealthCheckSystem {
    checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    config: HealthCheckConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub check_interval: Duration,
    pub timeout: Duration,
    pub failure_threshold: u32,
    pub recovery_threshold: u32,
}

#[derive(Clone, Serialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: Instant,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub details: HashMap<String, String>,
}

#[derive(Clone, Serialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthCheckSystem {
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    pub async fn check_tool(&self, tool_name: &str) -> HealthStatus {
        let mut checks = self.checks.write().await;
        let check = checks.entry(tool_name.to_string())
            .or_insert_with(|| HealthCheck {
                name: tool_name.to_string(),
                status: HealthStatus::Unknown,
                last_check: Instant::now(),
                consecutive_failures: 0,
                consecutive_successes: 0,
                details: HashMap::new(),
            });
        
        // Perform actual health check
        let healthy = self.perform_check(tool_name).await;
        
        if healthy {
            check.consecutive_successes += 1;
            check.consecutive_failures = 0;
            
            if check.consecutive_successes >= self.config.recovery_threshold {
                check.status = HealthStatus::Healthy;
            }
        } else {
            check.consecutive_failures += 1;
            check.consecutive_successes = 0;
            
            if check.consecutive_failures >= self.config.failure_threshold {
                check.status = HealthStatus::Unhealthy;
            } else if check.consecutive_failures > 0 {
                check.status = HealthStatus::Degraded;
            }
        }
        
        check.last_check = Instant::now();
        check.status.clone()
    }
    
    async fn perform_check(&self, tool_name: &str) -> bool {
        // Simulate health check - in production would test actual tool
        match tool_name {
            "readFile" | "writeFile" => {
                // Check filesystem access
                tokio::fs::metadata("/tmp").await.is_ok()
            },
            "executeCommand" => {
                // Check command execution capability
                tokio::process::Command::new("echo")
                    .arg("health")
                    .output()
                    .await
                    .is_ok()
            },
            _ => true,
        }
    }
    
    pub async fn get_all_statuses(&self) -> HashMap<String, HealthStatus> {
        let checks = self.checks.read().await;
        checks.iter()
            .map(|(name, check)| (name.clone(), check.status.clone()))
            .collect()
    }
    
    pub async fn get_unhealthy_tools(&self) -> Vec<String> {
        let checks = self.checks.read().await;
        checks.iter()
            .filter(|(_, check)| check.status == HealthStatus::Unhealthy)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
            recovery_threshold: 2,
        }
    }
}
