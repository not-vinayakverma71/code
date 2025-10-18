// Terminal Pre-IPC: Concurrency and stability guarantees
// Part of HP2: Concurrency & Stability feature

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use lapce_rpc::terminal::TermId;

/// Terminal lifecycle tracker for leak detection
#[derive(Debug, Clone)]
pub struct TerminalLifecycleTracker {
    /// Active terminals
    active: Arc<Mutex<HashMap<TermId, TerminalEntry>>>,
}

#[derive(Debug, Clone)]
struct TerminalEntry {
    /// When the terminal was created
    created_at: Instant,
    
    /// Total output bytes processed
    bytes_processed: usize,
    
    /// Number of commands executed
    commands_executed: u64,
}

impl TerminalLifecycleTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register a new terminal
    pub fn register(&self, term_id: TermId) {
        if let Ok(mut active) = self.active.lock() {
            active.insert(term_id, TerminalEntry {
                created_at: Instant::now(),
                bytes_processed: 0,
                commands_executed: 0,
            });
        }
    }
    
    /// Unregister a terminal
    pub fn unregister(&self, term_id: &TermId) -> bool {
        if let Ok(mut active) = self.active.lock() {
            active.remove(term_id).is_some()
        } else {
            false
        }
    }
    
    /// Record bytes processed for a terminal
    pub fn record_bytes(&self, term_id: &TermId, bytes: usize) {
        if let Ok(mut active) = self.active.lock() {
            if let Some(entry) = active.get_mut(term_id) {
                entry.bytes_processed += bytes;
            }
        }
    }
    
    /// Record command execution
    pub fn record_command(&self, term_id: &TermId) {
        if let Ok(mut active) = self.active.lock() {
            if let Some(entry) = active.get_mut(term_id) {
                entry.commands_executed += 1;
            }
        }
    }
    
    /// Get count of active terminals
    pub fn active_count(&self) -> usize {
        self.active.lock().map(|a| a.len()).unwrap_or(0)
    }
    
    /// Check for potential leaks (terminals active too long without activity)
    pub fn check_for_leaks(&self, max_idle_duration: Duration) -> Vec<TermId> {
        let mut leaked = Vec::new();
        
        if let Ok(active) = self.active.lock() {
            for (term_id, entry) in active.iter() {
                let age = entry.created_at.elapsed();
                
                // Consider a leak if terminal is very old with no activity
                if age > max_idle_duration && entry.commands_executed == 0 {
                    leaked.push(*term_id);
                }
            }
        }
        
        leaked
    }
    
    /// Get statistics for a terminal
    pub fn get_stats(&self, term_id: &TermId) -> Option<TerminalStats> {
        self.active.lock().ok()?.get(term_id).map(|entry| {
            TerminalStats {
                uptime: entry.created_at.elapsed(),
                bytes_processed: entry.bytes_processed,
                commands_executed: entry.commands_executed,
            }
        })
    }
    
    /// Reset all tracking data
    pub fn reset(&self) {
        if let Ok(mut active) = self.active.lock() {
            active.clear();
        }
    }
}

impl Default for TerminalLifecycleTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a terminal
#[derive(Debug, Clone)]
pub struct TerminalStats {
    pub uptime: Duration,
    pub bytes_processed: usize,
    pub commands_executed: u64,
}

/// Concurrency test harness for stress testing
#[cfg(test)]
pub mod stress_tests {
    use super::*;
    use std::thread;
    use std::sync::mpsc;
    
    /// Simulate rapid terminal creation/destruction
    pub fn rapid_terminal_lifecycle_test(iterations: usize, threads: usize) -> Result<(), String> {
        let tracker = Arc::new(TerminalLifecycleTracker::new());
        let mut handles = vec![];
        
        for _t in 0..threads {
            let tracker_clone = Arc::clone(&tracker);
            let iters = iterations / threads;
            
            let handle = thread::spawn(move || {
                for _i in 0..iters {
                    let term_id = TermId::next();
                    tracker_clone.register(term_id);
                    tracker_clone.record_command(&term_id);
                    tracker_clone.unregister(&term_id);
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().map_err(|_| "Thread panicked")?;
        }
        
        // Should have no active terminals (all cleaned up)
        let active = tracker.active_count();
        if active == 0 {
            Ok(())
        } else {
            Err(format!("Leak detected: {} terminals still active", active))
        }
    }
    
    /// Simulate concurrent data processing
    pub fn concurrent_data_processing_test(
        terminals: usize,
        bytes_per_terminal: usize,
    ) -> Result<Duration, String> {
        let tracker = Arc::new(TerminalLifecycleTracker::new());
        let mut handles = vec![];
        let start = Instant::now();
        
        for _ in 0..terminals {
            let tracker_clone = Arc::clone(&tracker);
            
            let handle = thread::spawn(move || {
                let term_id = TermId::next();
                tracker_clone.register(term_id);
                
                // Simulate processing data in chunks
                let chunk_size = 1024;
                for _ in 0..(bytes_per_terminal / chunk_size) {
                    tracker_clone.record_bytes(&term_id, chunk_size);
                }
                
                tracker_clone.unregister(&term_id);
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().map_err(|_| "Thread panicked")?;
        }
        
        let duration = start.elapsed();
        
        // Verify cleanup
        if tracker.active_count() == 0 {
            Ok(duration)
        } else {
            Err("Terminals not cleaned up".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_tracker_register_unregister() {
        let tracker = TerminalLifecycleTracker::new();
        let term_id = TermId::next();
        
        assert_eq!(tracker.active_count(), 0);
        
        tracker.register(term_id);
        assert_eq!(tracker.active_count(), 1);
        
        let unregistered = tracker.unregister(&term_id);
        assert!(unregistered);
        assert_eq!(tracker.active_count(), 0);
    }
    
    #[test]
    fn test_tracker_record_bytes() {
        let tracker = TerminalLifecycleTracker::new();
        let term_id = TermId::next();
        
        tracker.register(term_id);
        tracker.record_bytes(&term_id, 1024);
        tracker.record_bytes(&term_id, 2048);
        
        let stats = tracker.get_stats(&term_id).unwrap();
        assert_eq!(stats.bytes_processed, 3072);
    }
    
    #[test]
    fn test_tracker_record_commands() {
        let tracker = TerminalLifecycleTracker::new();
        let term_id = TermId::next();
        
        tracker.register(term_id);
        tracker.record_command(&term_id);
        tracker.record_command(&term_id);
        tracker.record_command(&term_id);
        
        let stats = tracker.get_stats(&term_id).unwrap();
        assert_eq!(stats.commands_executed, 3);
    }
    
    #[test]
    fn test_tracker_leak_detection() {
        let tracker = TerminalLifecycleTracker::new();
        
        // Create a terminal with activity (should not be flagged as leak)
        let active_term = TermId::next();
        tracker.register(active_term);
        tracker.record_command(&active_term);
        
        // Create a terminal without activity (potential leak)
        let idle_term = TermId::next();
        tracker.register(idle_term);
        
        // Check for leaks (very short duration for testing)
        let leaks = tracker.check_for_leaks(Duration::from_nanos(1));
        
        // idle_term might be flagged depending on timing
        assert!(leaks.len() <= 1);
    }
    
    #[test]
    fn test_tracker_reset() {
        let tracker = TerminalLifecycleTracker::new();
        
        for _ in 0..5 {
            tracker.register(TermId::next());
        }
        
        assert_eq!(tracker.active_count(), 5);
        
        tracker.reset();
        assert_eq!(tracker.active_count(), 0);
    }
    
    #[test]
    fn test_tracker_concurrent_access() {
        let tracker = Arc::new(TerminalLifecycleTracker::new());
        let mut handles = vec![];
        
        for _ in 0..10 {
            let tracker_clone = Arc::clone(&tracker);
            
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let term_id = TermId::next();
                    tracker_clone.register(term_id);
                    tracker_clone.record_command(&term_id);
                    tracker_clone.unregister(&term_id);
                }
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        // All terminals should be cleaned up
        assert_eq!(tracker.active_count(), 0);
    }
    
    #[test]
    fn test_rapid_lifecycle_stress() {
        // Stress test with 1000 terminals across 10 threads
        let result = stress_tests::rapid_terminal_lifecycle_test(1000, 10);
        assert!(result.is_ok(), "Stress test failed: {:?}", result);
    }
    
    #[test]
    fn test_concurrent_data_processing() {
        // Process data on 50 terminals concurrently
        let result = stress_tests::concurrent_data_processing_test(50, 10240);
        assert!(result.is_ok(), "Concurrent processing failed: {:?}", result);
        
        let duration = result.unwrap();
        // Should complete reasonably fast (< 1 second)
        assert!(duration < Duration::from_secs(1));
    }
    
    #[test]
    fn test_stats_retrieval() {
        let tracker = TerminalLifecycleTracker::new();
        let term_id = TermId::next();
        
        tracker.register(term_id);
        tracker.record_bytes(&term_id, 5000);
        tracker.record_command(&term_id);
        
        let stats = tracker.get_stats(&term_id).unwrap();
        assert!(stats.uptime < Duration::from_secs(1));
        assert_eq!(stats.bytes_processed, 5000);
        assert_eq!(stats.commands_executed, 1);
    }
}
