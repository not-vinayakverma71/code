/// Exact 1:1 Translation of TypeScript metrics from codex-reference/shared/getApiMetrics.ts
/// DAY 5 H3-4: Port metrics collection

use serde::{Deserialize, Serialize};
use serde_json;

/// TokenUsage structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub total_tokens_in: u32,
    pub total_tokens_out: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cache_writes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cache_reads: Option<u32>,
    pub total_cost: f64,
    pub context_tokens: u32,
}

/// ParsedApiReqStartedTextType - exact translation lines 8-15
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedApiReqStartedText {
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub cache_writes: u32,
    pub cache_reads: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_protocol: Option<String>, // "anthropic" or "openai"
}

/// ClineMessage for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClineMessage {
    #[serde(rename = "say")]
    Say {
        say: String,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        context_condense: Option<ContextCondense>,
        ts: u64,
    },
    #[serde(rename = "ask")]
    Ask {
        ask: String,
        text: String,
        ts: u64,
    },
}

/// ContextCondense structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextCondense {
    pub cost: f64,
    pub new_context_tokens: u32,
}

/// ClineSayTool for fast apply
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineSayTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_apply_result: Option<FastApplyResult>,
}

/// FastApplyResult structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FastApplyResult {
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub cost: f64,
}

/// getApiMetrics - exact translation lines 34-112
pub fn get_api_metrics(messages: &[ClineMessage]) -> TokenUsage {
    let mut result = TokenUsage::default();
    
    // Calculate running totals - lines 45-81
    for message in messages {
        match message {
            ClineMessage::Say { say, text, context_condense, .. } => {
                if say == "api_req_started" && !text.is_empty() {
                    // Parse API request metrics
                    if let Ok(parsed) = serde_json::from_str::<ParsedApiReqStartedText>(text) {
                        result.total_tokens_in += parsed.tokens_in;
                        result.total_tokens_out += parsed.tokens_out;
                        
                        if parsed.cache_writes > 0 {
                            result.total_cache_writes = Some(
                                result.total_cache_writes.unwrap_or(0) + parsed.cache_writes
                            );
                        }
                        
                        if parsed.cache_reads > 0 {
                            result.total_cache_reads = Some(
                                result.total_cache_reads.unwrap_or(0) + parsed.cache_reads
                            );
                        }
                        
                        if let Some(cost) = parsed.cost {
                            result.total_cost += cost;
                        }
                    }
                } else if say == "condense_context" {
                    if let Some(ref condense) = context_condense {
                        result.total_cost += condense.cost;
                    }
                }
            }
            ClineMessage::Ask { ask, text, .. } => {
                if ask == "tool" && !text.is_empty() {
                    // Handle fast apply results
                    if let Ok(tool) = safe_json_parse::<ClineSayTool>(text) {
                        if let Some(fast_apply) = tool.fast_apply_result {
                            result.total_tokens_in += fast_apply.tokens_in;
                            result.total_tokens_out += fast_apply.tokens_out;
                            result.total_cost += fast_apply.cost;
                        }
                    }
                }
            }
        }
    }
    
    // Calculate context tokens from last API request - lines 84-109
    result.context_tokens = 0;
    for message in messages.iter().rev() {
        match message {
            ClineMessage::Say { say, text, context_condense, .. } => {
                if say == "api_req_started" && !text.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<ParsedApiReqStartedText>(text) {
                        // Calculate based on API protocol
                        result.context_tokens = match parsed.api_protocol.as_deref() {
                            Some("anthropic") => {
                                parsed.tokens_in + parsed.tokens_out + 
                                parsed.cache_writes + parsed.cache_reads
                            }
                            _ => {
                                // OpenAI or unspecified
                                parsed.tokens_in + parsed.tokens_out
                            }
                        };
                        
                        if result.context_tokens > 0 {
                            break;
                        }
                    }
                } else if say == "condense_context" {
                    if let Some(ref condense) = context_condense {
                        result.context_tokens = condense.new_context_tokens;
                        if result.context_tokens > 0 {
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    result
}

/// Safe JSON parse helper
fn safe_json_parse<T: for<'de> Deserialize<'de>>(text: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(text)
}

/// Metrics collector for tracking over time
pub struct MetricsCollector {
    samples: Vec<TokenUsage>,
    start_time: std::time::Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            start_time: std::time::Instant::now(),
        }
    }
    
    pub fn add_sample(&mut self, usage: TokenUsage) {
        self.samples.push(usage);
    }
    
    pub fn get_total(&self) -> TokenUsage {
        let mut total = TokenUsage::default();
        
        for sample in &self.samples {
            total.total_tokens_in += sample.total_tokens_in;
            total.total_tokens_out += sample.total_tokens_out;
            
            if let Some(writes) = sample.total_cache_writes {
                total.total_cache_writes = Some(
                    total.total_cache_writes.unwrap_or(0) + writes
                );
            }
            
            if let Some(reads) = sample.total_cache_reads {
                total.total_cache_reads = Some(
                    total.total_cache_reads.unwrap_or(0) + reads
                );
            }
            
            total.total_cost += sample.total_cost;
            total.context_tokens = sample.context_tokens; // Use last value
        }
        
        total
    }
    
    pub fn get_rate(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.samples.len() as f64 / elapsed
        } else {
            0.0
        }
    }
    
    pub fn reset(&mut self) {
        self.samples.clear();
        self.start_time = std::time::Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_api_metrics() {
        let messages = vec![
            ClineMessage::Say {
                say: "api_req_started".to_string(),
                text: r#"{"tokensIn":10,"tokensOut":20,"cacheWrites":5,"cacheReads":3,"cost":0.005,"apiProtocol":"anthropic"}"#.to_string(),
                context_condense: None,
                ts: 1000,
            },
            ClineMessage::Say {
                say: "api_req_started".to_string(),
                text: r#"{"tokensIn":15,"tokensOut":25,"cacheWrites":0,"cacheReads":0,"cost":0.008}"#.to_string(),
                context_condense: None,
                ts: 2000,
            },
        ];
        
        let metrics = get_api_metrics(&messages);
        
        assert_eq!(metrics.total_tokens_in, 25);
        assert_eq!(metrics.total_tokens_out, 45);
        assert_eq!(metrics.total_cache_writes, Some(5));
        assert_eq!(metrics.total_cache_reads, Some(3));
        // Use approximate comparison for floating point
        assert!((metrics.total_cost - 0.013).abs() < 0.0001, "Expected cost ~0.013, got {}", metrics.total_cost);
        assert_eq!(metrics.context_tokens, 40); // Last request: 15+25
    }
    
    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();
        
        collector.add_sample(TokenUsage {
            total_tokens_in: 100,
            total_tokens_out: 50,
            total_cost: 0.01,
            ..Default::default()
        });
        
        collector.add_sample(TokenUsage {
            total_tokens_in: 200,
            total_tokens_out: 100,
            total_cost: 0.02,
            ..Default::default()
        });
        
        let total = collector.get_total();
        assert_eq!(total.total_tokens_in, 300);
        assert_eq!(total.total_tokens_out, 150);
        assert_eq!(total.total_cost, 0.03);
    }
}
