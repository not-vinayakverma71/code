/// Query Optimization - Day 43 PM
use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct QueryOptimizer {
    query_cache: HashMap<String, CachedQueryPlan>,
    statistics: QueryStatistics,
}

#[derive(Debug, Clone)]
pub struct CachedQueryPlan {
    pub query: String,
    pub plan: ExecutionPlan,
    pub cost: f64,
    pub last_used: Instant,
    pub hit_count: u64,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub steps: Vec<PlanStep>,
    pub estimated_rows: usize,
    pub estimated_cost: f64,
    pub uses_index: bool,
}

#[derive(Debug, Clone)]
pub enum PlanStep {
    IndexScan { index: String, range: Option<(Vec<u8>, Vec<u8>)> },
    FullTableScan { table: String },
    Join { method: JoinMethod, left: Box<PlanStep>, right: Box<PlanStep> },
    Filter { predicate: String },
    Sort { keys: Vec<String> },
    Limit { count: usize },
}

#[derive(Debug, Clone)]
pub enum JoinMethod {
    NestedLoop,
    HashJoin,
    MergeJoin,
}

#[derive(Debug, Clone, Default)]
pub struct QueryStatistics {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub avg_planning_time_us: f64,
    pub avg_execution_time_ms: f64,
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            query_cache: HashMap::new(),
            statistics: QueryStatistics::default(),
        }
    }
    
    pub fn optimize_query(&mut self, query: &str) -> Result<ExecutionPlan> {
        let start = Instant::now();
        
        // Check cache
        if let Some(cached) = self.query_cache.get_mut(query) {
            cached.hit_count += 1;
            cached.last_used = Instant::now();
            self.statistics.cache_hits += 1;
            return Ok(cached.plan.clone());
        }
        
        // Generate new plan
        let plan = self.generate_plan(query)?;
        let cost = self.estimate_cost(&plan);
        
        self.query_cache.insert(query.to_string(), CachedQueryPlan {
            query: query.to_string(),
            plan: plan.clone(),
            cost,
            last_used: Instant::now(),
            hit_count: 1,
        });
        
        let elapsed = start.elapsed().as_micros() as f64;
        self.update_statistics(elapsed);
        
        Ok(plan)
    }
    
    fn generate_plan(&self, query: &str) -> Result<ExecutionPlan> {
        let uses_index = query.contains("WHERE") && query.contains("=");
        let estimated_rows = if uses_index { 10 } else { 1000 };
        
        let mut steps = vec![];
        
        if uses_index {
            steps.push(PlanStep::IndexScan {
                index: "primary_key".to_string(),
                range: None,
            });
        } else {
            steps.push(PlanStep::FullTableScan {
                table: "main_table".to_string(),
            });
        }
        
        if query.contains("ORDER BY") {
            steps.push(PlanStep::Sort {
                keys: vec!["id".to_string()],
            });
        }
        
        if query.contains("LIMIT") {
            steps.push(PlanStep::Limit { count: 100 });
        }
        
        Ok(ExecutionPlan {
            steps,
            estimated_rows,
            estimated_cost: if uses_index { 10.0 } else { 100.0 },
            uses_index,
        })
    }
    
    fn estimate_cost(&self, plan: &ExecutionPlan) -> f64 {
        let mut cost = 0.0;
        
        for step in &plan.steps {
            cost += match step {
                PlanStep::IndexScan { .. } => 1.0,
                PlanStep::FullTableScan { .. } => 100.0,
                PlanStep::Join { method, .. } => match method {
                    JoinMethod::NestedLoop => 1000.0,
                    JoinMethod::HashJoin => 100.0,
                    JoinMethod::MergeJoin => 50.0,
                },
                PlanStep::Filter { .. } => 5.0,
                PlanStep::Sort { .. } => 20.0,
                PlanStep::Limit { .. } => 1.0,
            };
        }
        
        cost
    }
    
    fn update_statistics(&mut self, planning_time_us: f64) {
        self.statistics.total_queries += 1;
        let n = self.statistics.total_queries as f64;
        self.statistics.avg_planning_time_us = 
            (self.statistics.avg_planning_time_us * (n - 1.0) + planning_time_us) / n;
    }
    
    pub fn suggest_indexes(&self) -> Vec<IndexSuggestion> {
        let mut suggestions = vec![];
        
        for (query, plan) in &self.query_cache {
            if !plan.plan.uses_index && plan.hit_count > 10 {
                suggestions.push(IndexSuggestion {
                    query: query.clone(),
                    suggested_index: "CREATE INDEX ON table(column)".to_string(),
                    expected_speedup: 10.0,
                });
            }
        }
        
        suggestions
    }
}

#[derive(Debug, Clone)]
pub struct IndexSuggestion {
    pub query: String,
    pub suggested_index: String,
    pub expected_speedup: f64,
}
