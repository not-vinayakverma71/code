// PHASE 1, STEP 1.1.3: QUALITY METRICS CALCULATOR
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub precision_at_k: HashMap<usize, f64>,
    pub recall_at_k: HashMap<usize, f64>,
    pub f1_at_k: HashMap<usize, f64>,
    pub ndcg_at_k: HashMap<usize, f64>,
    pub map: f64,  // Mean Average Precision
    pub mrr: f64,  // Mean Reciprocal Rank
    pub accuracy_at_1: f64,
    pub accuracy_at_5: f64,
    pub accuracy_at_10: f64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct RelevanceJudgment {
    pub file_path: String,
    pub relevance: f32,  // 0.0 to 1.0
    pub is_relevant: bool,  // Binary relevance
}

pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Calculate Precision@K
    /// Precision = (# of relevant items in top K) / K
    pub fn precision_at_k(
        results: &[SearchResult],
        relevance: &[RelevanceJudgment],
        k: usize,
    ) -> f64 {
        let relevant_set: HashSet<String> = relevance
            .iter()
            .filter(|r| r.is_relevant)
            .map(|r| r.file_path.clone())
            .collect();
        
        let top_k = results.iter().take(k);
        let relevant_in_k = top_k
            .filter(|r| relevant_set.contains(&r.file_path))
            .count();
        
        if k == 0 {
            return 0.0;
        }
        
        relevant_in_k as f64 / k as f64
    }
    
    /// Calculate Recall@K
    /// Recall = (# of relevant items in top K) / (total # of relevant items)
    pub fn recall_at_k(
        results: &[SearchResult],
        relevance: &[RelevanceJudgment],
        k: usize,
    ) -> f64 {
        let relevant_set: HashSet<String> = relevance
            .iter()
            .filter(|r| r.is_relevant)
            .map(|r| r.file_path.clone())
            .collect();
        
        let total_relevant = relevant_set.len();
        if total_relevant == 0 {
            return 0.0;
        }
        
        let top_k = results.iter().take(k);
        let relevant_in_k = top_k
            .filter(|r| relevant_set.contains(&r.file_path))
            .count();
        
        relevant_in_k as f64 / total_relevant as f64
    }
    
    /// Calculate F1 Score@K
    /// F1 = 2 * (precision * recall) / (precision + recall)
    pub fn f1_at_k(
        results: &[SearchResult],
        relevance: &[RelevanceJudgment],
        k: usize,
    ) -> f64 {
        let precision = Self::precision_at_k(results, relevance, k);
        let recall = Self::recall_at_k(results, relevance, k);
        
        if precision + recall == 0.0 {
            return 0.0;
        }
        
        2.0 * (precision * recall) / (precision + recall)
    }
    
    /// Calculate NDCG@K (Normalized Discounted Cumulative Gain)
    pub fn ndcg_at_k(
        results: &[SearchResult],
        relevance: &[RelevanceJudgment],
        k: usize,
    ) -> f64 {
        // Create relevance map
        let relevance_map: HashMap<String, f32> = relevance
            .iter()
            .map(|r| (r.file_path.clone(), r.relevance))
            .collect();
        
        // Calculate DCG@K
        let dcg = Self::dcg_at_k(results, &relevance_map, k);
        
        // Calculate ideal DCG@K (sort by relevance)
        let mut ideal_relevances: Vec<f32> = relevance
            .iter()
            .map(|r| r.relevance)
            .collect();
        ideal_relevances.sort_by(|a, b| b.partial_cmp(a).unwrap());
        
        let mut ideal_results: Vec<SearchResult> = ideal_relevances
            .iter()
            .enumerate()
            .map(|(i, &rel)| SearchResult {
                file_path: format!("ideal_{}", i),
                score: rel,
            })
            .collect();
        
        let ideal_relevance_map: HashMap<String, f32> = ideal_results
            .iter()
            .zip(ideal_relevances.iter())
            .map(|(r, &rel)| (r.file_path.clone(), rel))
            .collect();
        
        let idcg = Self::dcg_at_k(&ideal_results, &ideal_relevance_map, k);
        
        if idcg == 0.0 {
            return 0.0;
        }
        
        dcg / idcg
    }
    
    /// Calculate DCG@K (Discounted Cumulative Gain)
    fn dcg_at_k(
        results: &[SearchResult],
        relevance_map: &HashMap<String, f32>,
        k: usize,
    ) -> f64 {
        results
            .iter()
            .take(k)
            .enumerate()
            .map(|(i, result)| {
                let rel = relevance_map.get(&result.file_path).unwrap_or(&0.0);
                let discount = (i as f64 + 2.0).log2();
                *rel as f64 / discount
            })
            .sum()
    }
    
    /// Calculate MRR (Mean Reciprocal Rank)
    pub fn mean_reciprocal_rank(
        results_list: &[Vec<SearchResult>],
        relevance_list: &[Vec<RelevanceJudgment>],
    ) -> f64 {
        let mut sum_rr = 0.0;
        
        for (results, relevance) in results_list.iter().zip(relevance_list.iter()) {
            let relevant_set: HashSet<String> = relevance
                .iter()
                .filter(|r| r.is_relevant)
                .map(|r| r.file_path.clone())
                .collect();
            
            // Find first relevant result
            for (i, result) in results.iter().enumerate() {
                if relevant_set.contains(&result.file_path) {
                    sum_rr += 1.0 / (i + 1) as f64;
                    break;
                }
            }
        }
        
        if results_list.is_empty() {
            return 0.0;
        }
        
        sum_rr / results_list.len() as f64
    }
    
    /// Calculate MAP (Mean Average Precision)
    pub fn mean_average_precision(
        results_list: &[Vec<SearchResult>],
        relevance_list: &[Vec<RelevanceJudgment>],
    ) -> f64 {
        let mut sum_ap = 0.0;
        
        for (results, relevance) in results_list.iter().zip(relevance_list.iter()) {
            sum_ap += Self::average_precision(results, relevance);
        }
        
        if results_list.is_empty() {
            return 0.0;
        }
        
        sum_ap / results_list.len() as f64
    }
    
    /// Calculate Average Precision for a single query
    fn average_precision(
        results: &[SearchResult],
        relevance: &[RelevanceJudgment],
    ) -> f64 {
        let relevant_set: HashSet<String> = relevance
            .iter()
            .filter(|r| r.is_relevant)
            .map(|r| r.file_path.clone())
            .collect();
        
        if relevant_set.is_empty() {
            return 0.0;
        }
        
        let mut sum_precision = 0.0;
        let mut num_relevant_found = 0;
        
        for (i, result) in results.iter().enumerate() {
            if relevant_set.contains(&result.file_path) {
                num_relevant_found += 1;
                let precision_at_i = num_relevant_found as f64 / (i + 1) as f64;
                sum_precision += precision_at_i;
            }
        }
        
        sum_precision / relevant_set.len() as f64
    }
    
    /// Calculate all metrics for a set of queries
    pub fn calculate_all_metrics(
        results_list: &[Vec<SearchResult>],
        relevance_list: &[Vec<RelevanceJudgment>],
    ) -> QualityMetrics {
        let k_values = vec![1, 3, 5, 10];
        
        let mut precision_at_k = HashMap::new();
        let mut recall_at_k = HashMap::new();
        let mut f1_at_k = HashMap::new();
        let mut ndcg_at_k = HashMap::new();
        
        for k in &k_values {
            let mut sum_precision = 0.0;
            let mut sum_recall = 0.0;
            let mut sum_f1 = 0.0;
            let mut sum_ndcg = 0.0;
            
            for (results, relevance) in results_list.iter().zip(relevance_list.iter()) {
                sum_precision += Self::precision_at_k(results, relevance, *k);
                sum_recall += Self::recall_at_k(results, relevance, *k);
                sum_f1 += Self::f1_at_k(results, relevance, *k);
                sum_ndcg += Self::ndcg_at_k(results, relevance, *k);
            }
            
            let n = results_list.len() as f64;
            precision_at_k.insert(*k, sum_precision / n);
            recall_at_k.insert(*k, sum_recall / n);
            f1_at_k.insert(*k, sum_f1 / n);
            ndcg_at_k.insert(*k, sum_ndcg / n);
        }
        
        // Calculate accuracy metrics
        let accuracy_at_1 = *precision_at_k.get(&1).unwrap_or(&0.0);
        let accuracy_at_5 = *precision_at_k.get(&5).unwrap_or(&0.0);
        let accuracy_at_10 = *precision_at_k.get(&10).unwrap_or(&0.0);
        
        QualityMetrics {
            precision_at_k,
            recall_at_k,
            f1_at_k,
            ndcg_at_k,
            map: Self::mean_average_precision(results_list, relevance_list),
            mrr: Self::mean_reciprocal_rank(results_list, relevance_list),
            accuracy_at_1,
            accuracy_at_5,
            accuracy_at_10,
        }
    }
}

/// Format metrics for display
pub fn format_metrics(metrics: &QualityMetrics) -> String {
    let mut output = String::new();
    
    output.push_str("📊 SEARCH QUALITY METRICS\n");
    output.push_str(&"=".repeat(50));
    output.push_str("\n\n");
    
    output.push_str("Precision@K:\n");
    for k in &[1, 3, 5, 10] {
        if let Some(p) = metrics.precision_at_k.get(k) {
            output.push_str(&format!("  P@{}: {:.3}\n", k, p));
        }
    }
    
    output.push_str("\nRecall@K:\n");
    for k in &[1, 3, 5, 10] {
        if let Some(r) = metrics.recall_at_k.get(k) {
            output.push_str(&format!("  R@{}: {:.3}\n", k, r));
        }
    }
    
    output.push_str("\nF1@K:\n");
    for k in &[1, 3, 5, 10] {
        if let Some(f) = metrics.f1_at_k.get(k) {
            output.push_str(&format!("  F1@{}: {:.3}\n", k, f));
        }
    }
    
    output.push_str("\nNDCG@K:\n");
    for k in &[1, 3, 5, 10] {
        if let Some(n) = metrics.ndcg_at_k.get(k) {
            output.push_str(&format!("  NDCG@{}: {:.3}\n", k, n));
        }
    }
    
    output.push_str(&format!("\nMAP: {:.3}\n", metrics.map));
    output.push_str(&format!("MRR: {:.3}\n", metrics.mrr));
    
    output.push_str(&format!("\nAccuracy@1: {:.3}\n", metrics.accuracy_at_1));
    output.push_str(&format!("Accuracy@5: {:.3}\n", metrics.accuracy_at_5));
    output.push_str(&format!("Accuracy@10: {:.3}\n", metrics.accuracy_at_10));
    
    // Success criteria check
    output.push_str("\n");
    output.push_str(&"=".repeat(50));
    output.push_str("\n✅ SUCCESS CRITERIA (Target: >90% accuracy):\n");
    
    let avg_accuracy = (metrics.accuracy_at_1 + metrics.accuracy_at_5 + metrics.accuracy_at_10) / 3.0;
    if avg_accuracy >= 0.9 {
        output.push_str(&format!("  ✅ PASSED: Average accuracy = {:.1}%\n", avg_accuracy * 100.0));
    } else {
        output.push_str(&format!("  ❌ FAILED: Average accuracy = {:.1}% (need >90%)\n", avg_accuracy * 100.0));
    }
    
    output
}
