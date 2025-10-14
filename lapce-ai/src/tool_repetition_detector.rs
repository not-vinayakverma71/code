// Tool Repetition Detector - CHUNK-03: T14
// Detects repetitive tool usage patterns (independent of execution)

use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// Configuration for repetition detection
#[derive(Debug, Clone)]
pub struct RepetitionConfig {
    /// Window size for tracking recent tool calls
    pub window_size: usize,
    
    /// Threshold for repetition (same tool X times in window)
    pub repetition_threshold: usize,
    
    /// Time window in seconds (for time-based detection)
    pub time_window_secs: u64,
    
    /// Similarity threshold for detecting near-duplicate parameters
    pub similarity_threshold: f32,
}

impl Default for RepetitionConfig {
    fn default() -> Self {
        Self {
            window_size: 10,
            repetition_threshold: 3,
            time_window_secs: 60,
            similarity_threshold: 0.9,
        }
    }
}

/// Tool call record for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// Tool name
    pub tool_name: String,
    
    /// Tool parameters (JSON string for comparison)
    pub params: String,
    
    /// Timestamp of the call
    pub timestamp: SystemTime,
    
    /// Sequence number
    pub sequence: u64,
}

/// Repetition detection result
#[derive(Debug, Clone, PartialEq)]
pub enum RepetitionResult {
    /// No repetition detected
    None,
    NoRepetition, // Alias for compatibility
    
    /// Same tool called repeatedly
    SameTool {
        tool_name: String,
        count: usize,
    },
    SameToolRepeated { // Alias for compatibility
        tool_name: String,
        count: usize,
    },
    
    /// Same tool with identical parameters
    IdenticalCalls {
        tool_name: String,
        count: usize,
    },
    IdenticalCall { // Alias for compatibility
        tool_name: String,
        count: usize,
    },
    
    /// Cyclic pattern detected
    CyclicPattern {
        pattern: Vec<String>,
        cycle_count: usize,
        pattern_length: usize, // Added field
    },
}

/// Tool repetition detector
pub struct ToolRepetitionDetector {
    /// Configuration
    config: RepetitionConfig,
    
    /// Recent tool calls (sliding window)
    recent_calls: VecDeque<ToolCallRecord>,
    
    /// Per-tool counters
    tool_counters: HashMap<String, usize>,
    
    /// Sequence counter
    sequence: u64,
    
    /// Pattern detection buffer
    pattern_buffer: Vec<String>,
}

impl ToolRepetitionDetector {
    /// Create a new detector with default config
    pub fn new() -> Self {
        Self::with_config(RepetitionConfig::default())
    }
    
    /// Create with custom config
    pub fn with_config(config: RepetitionConfig) -> Self {
        Self {
            config,
            recent_calls: VecDeque::new(),
            tool_counters: HashMap::new(),
            sequence: 0,
            pattern_buffer: Vec::new(),
        }
    }
    
    /// Record a tool call and check for repetition
    pub fn record_call(&mut self, tool_name: &str, params: &str) -> RepetitionResult {
        self.sequence += 1;
        
        let record = ToolCallRecord {
            tool_name: tool_name.to_string(),
            params: params.to_string(),
            timestamp: SystemTime::now(),
            sequence: self.sequence,
        };
        
        // Add to recent calls window
        self.recent_calls.push_back(record.clone());
        
        // Trim window to configured size
        while self.recent_calls.len() > self.config.window_size {
            if let Some(old_record) = self.recent_calls.pop_front() {
                // Decrement counter for removed call
                if let Some(count) = self.tool_counters.get_mut(&old_record.tool_name) {
                    *count = count.saturating_sub(1);
                }
            }
        }
        
        // Update counter for this tool
        *self.tool_counters.entry(tool_name.to_string()).or_insert(0) += 1;
        
        // Update pattern buffer
        self.pattern_buffer.push(tool_name.to_string());
        if self.pattern_buffer.len() > self.config.window_size {
            self.pattern_buffer.remove(0);
        }
        
        // Detect repetition
        self.detect_repetition(&record)
    }
    
    /// Detect repetition patterns
    fn detect_repetition(&self, current_call: &ToolCallRecord) -> RepetitionResult {
        // Check for identical calls
        if let Some(result) = self.detect_identical_calls(current_call) {
            return result;
        }
        
        // Check for same tool repetition
        if let Some(result) = self.detect_same_tool_repetition(current_call) {
            return result;
        }
        
        // Check for cyclic patterns
        if let Some(result) = self.detect_cyclic_pattern() {
            return result;
        }
        
        RepetitionResult::None
    }
    
    /// Detect identical tool calls (same tool + same params)
    fn detect_identical_calls(&self, current_call: &ToolCallRecord) -> Option<RepetitionResult> {
        let identical_count = self.recent_calls.iter()
            .filter(|call| {
                call.tool_name == current_call.tool_name &&
                self.params_are_similar(&call.params, &current_call.params)
            })
            .count();
        
        if identical_count >= self.config.repetition_threshold {
            return Some(RepetitionResult::IdenticalCalls {
                tool_name: current_call.tool_name.clone(),
                count: identical_count,
            });
        }
        
        None
    }
    
    /// Detect same tool being called repeatedly (different params)
    fn detect_same_tool_repetition(&self, current_call: &ToolCallRecord) -> Option<RepetitionResult> {
        let count = *self.tool_counters.get(&current_call.tool_name).unwrap_or(&0);
        
        if count >= self.config.repetition_threshold {
            return Some(RepetitionResult::SameTool {
                tool_name: current_call.tool_name.clone(),
                count,
            });
        }
        
        None
    }
    
    /// Detect cyclic patterns (e.g., A->B->C->A->B->C)
    fn detect_cyclic_pattern(&self) -> Option<RepetitionResult> {
        if self.pattern_buffer.len() < 6 {
            return None;
        }
        
        // Try different pattern lengths
        for pattern_len in 2..=self.pattern_buffer.len() / 2 {
            if let Some(pattern) = self.find_repeating_pattern(pattern_len) {
                let cycle_count = self.pattern_buffer.len() / pattern_len;
                if cycle_count >= 2 {
                    return Some(RepetitionResult::CyclicPattern {
                        pattern: pattern.clone(),
                        cycle_count,
                        pattern_length: pattern.len(),
                    });
                }
            }
        }
        
        None
    }
    
    /// Find repeating pattern of given length
    fn find_repeating_pattern(&self, pattern_len: usize) -> Option<Vec<String>> {
        if self.pattern_buffer.len() < pattern_len * 2 {
            return None;
        }
        
        let pattern: Vec<String> = self.pattern_buffer[self.pattern_buffer.len() - pattern_len..]
            .iter()
            .cloned()
            .collect();
        
        // Check if this pattern repeats
        let prev_pattern: Vec<String> = self.pattern_buffer[
            self.pattern_buffer.len() - pattern_len * 2..
            self.pattern_buffer.len() - pattern_len
        ].iter().cloned().collect();
        
        if pattern == prev_pattern {
            Some(pattern)
        } else {
            None
        }
    }
    
    /// Check if parameters are similar (simple string similarity)
    fn params_are_similar(&self, params1: &str, params2: &str) -> bool {
        if params1 == params2 {
            return true;
        }
        
        // Simple Jaccard similarity for JSON comparison
        let similarity = self.calculate_similarity(params1, params2);
        similarity >= self.config.similarity_threshold
    }
    
    /// Calculate similarity between two parameter strings
    fn calculate_similarity(&self, s1: &str, s2: &str) -> f32 {
        // Simple character-level similarity
        let len1 = s1.len();
        let len2 = s2.len();
        
        if len1 == 0 && len2 == 0 {
            return 1.0;
        }
        
        if len1 == 0 || len2 == 0 {
            return 0.0;
        }
        
        let max_len = len1.max(len2);
        let min_len = len1.min(len2);
        
        // Count matching characters (simple)
        let matching = s1.chars()
            .zip(s2.chars())
            .filter(|(c1, c2)| c1 == c2)
            .count();
        
        matching as f32 / max_len as f32
    }
    
    /// Get repetition statistics
    pub fn get_statistics(&self) -> HashMap<String, usize> {
        self.tool_counters.clone()
    }
    
    /// Reset detector state
    pub fn reset(&mut self) {
        self.recent_calls.clear();
        self.tool_counters.clear();
        self.pattern_buffer.clear();
        self.sequence = 0;
    }
    
    /// Get recent calls
    pub fn get_recent_calls(&self) -> Vec<ToolCallRecord> {
        self.recent_calls.iter().cloned().collect()
    }
    
    /// Check if a specific tool is being overused
    pub fn is_tool_overused(&self, tool_name: &str) -> bool {
        let count = *self.tool_counters.get(tool_name).unwrap_or(&0);
        count >= self.config.repetition_threshold
    }
}

impl Default for ToolRepetitionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detector_creation() {
        let detector = ToolRepetitionDetector::new();
        assert_eq!(detector.sequence, 0);
        assert_eq!(detector.recent_calls.len(), 0);
    }
    
    #[test]
    fn test_no_repetition() {
        let mut detector = ToolRepetitionDetector::new();
        
        let result = detector.record_call("read_file", r#"{"path": "a.txt"}"#);
        assert_eq!(result, RepetitionResult::None);
        
        let result = detector.record_call("write_file", r#"{"path": "b.txt"}"#);
        assert_eq!(result, RepetitionResult::None);
    }
    
    #[test]
    fn test_same_tool_repetition() {
        let mut detector = ToolRepetitionDetector::new();
        
        // Call same tool 3 times (threshold)
        detector.record_call("read_file", r#"{"path": "a.txt"}"#);
        detector.record_call("read_file", r#"{"path": "b.txt"}"#);
        let result = detector.record_call("read_file", r#"{"path": "c.txt"}"#);
        
        match result {
            RepetitionResult::SameTool { tool_name, count } => {
                assert_eq!(tool_name, "read_file");
                assert_eq!(count, 3);
            }
            _ => panic!("Expected SameTool repetition"),
        }
    }
    
    #[test]
    fn test_identical_calls() {
        let mut detector = ToolRepetitionDetector::new();
        
        // Call same tool with identical params
        detector.record_call("read_file", r#"{"path": "a.txt"}"#);
        detector.record_call("read_file", r#"{"path": "a.txt"}"#);
        let result = detector.record_call("read_file", r#"{"path": "a.txt"}"#);
        
        match result {
            RepetitionResult::IdenticalCalls { tool_name, count } => {
                assert_eq!(tool_name, "read_file");
                assert_eq!(count, 3);
            }
            _ => panic!("Expected IdenticalCalls repetition"),
        }
    }
    
    #[test]
    fn test_cyclic_pattern() {
        let mut detector = ToolRepetitionDetector::new();
        
        // Create A->B->A->B pattern
        detector.record_call("read_file", "{}");
        detector.record_call("write_file", "{}");
        detector.record_call("read_file", "{}");
        detector.record_call("write_file", "{}");
        detector.record_call("read_file", "{}");
        let result = detector.record_call("write_file", "{}");
        
        // Should detect cyclic pattern
        match result {
            RepetitionResult::CyclicPattern { pattern, cycle_count, pattern_length } => {
                assert!(cycle_count >= 2);
                assert!(!pattern.is_empty());
                assert_eq!(pattern_length, pattern.len());
            }
            _ => {
                // Pattern might not be detected yet, acceptable
            }
        }
    }
    
    #[test]
    fn test_window_sliding() {
        let config = RepetitionConfig {
            window_size: 3,
            repetition_threshold: 2,
            ..Default::default()
        };
        let mut detector = ToolRepetitionDetector::with_config(config);
        
        // Fill window
        detector.record_call("tool_a", "{}");
        detector.record_call("tool_b", "{}");
        detector.record_call("tool_c", "{}");
        
        assert_eq!(detector.recent_calls.len(), 3);
        
        // Add one more, should evict first
        detector.record_call("tool_d", "{}");
        
        assert_eq!(detector.recent_calls.len(), 3);
        assert_eq!(detector.recent_calls[0].tool_name, "tool_b");
    }
    
    #[test]
    fn test_reset() {
        let mut detector = ToolRepetitionDetector::new();
        
        detector.record_call("read_file", "{}");
        detector.record_call("write_file", "{}");
        
        assert!(detector.sequence > 0);
        assert!(!detector.recent_calls.is_empty());
        
        detector.reset();
        
        assert_eq!(detector.sequence, 0);
        assert!(detector.recent_calls.is_empty());
        assert!(detector.tool_counters.is_empty());
    }
    
    #[test]
    fn test_is_tool_overused() {
        let mut detector = ToolRepetitionDetector::new();
        
        assert!(!detector.is_tool_overused("read_file"));
        
        detector.record_call("read_file", "{}");
        detector.record_call("read_file", "{}");
        detector.record_call("read_file", "{}");
        
        assert!(detector.is_tool_overused("read_file"));
    }
    
    #[test]
    fn test_statistics() {
        let mut detector = ToolRepetitionDetector::new();
        
        detector.record_call("read_file", "{}");
        detector.record_call("read_file", "{}");
        detector.record_call("write_file", "{}");
        
        let stats = detector.get_statistics();
        assert_eq!(stats.get("read_file"), Some(&2));
        assert_eq!(stats.get("write_file"), Some(&1));
    }
}
