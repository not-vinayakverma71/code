#![cfg(feature = "phase4-cache-tests")]
//! Memory stress tests to verify eviction and OOM prevention

use lapce_tree_sitter::{Phase4Cache, Phase4Config};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tree_sitter::Parser;

#[test]
#[ignore] // Run with: cargo test --test memory_stress -- --ignored
fn test_memory_budget_enforcement() {
    // Create cache with small budget
    let config = Phase4Config {
        memory_budget_mb: 10, // Only 10MB
        hot_tier_size_mb: 5,
        warm_tier_size_mb: 3,
        cold_tier_size_mb: 2,
        ..Default::default()
    };
    
    let cache = Phase4Cache::new(config).unwrap();
    
    // Generate large amount of data
    let mut total_stored = 0usize;
    let mut evictions = 0usize;
    
    for i in 0..1000 {
        let path = format!("test_{}.rs", i);
        let source = generate_large_source(i);
        
        // Parse and store
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(&source, None).unwrap();
        
        let hash = calculate_hash(&source);
        let result = cache.store(path.clone().into(), hash, tree, source.as_bytes());
        
        if result.is_ok() {
            total_stored += source.len();
        } else {
            evictions += 1;
        }
        
        // Check memory usage doesn't exceed budget
        let stats = cache.get_stats();
        assert!(
            stats.memory_used_mb <= 10.0,
            "Memory budget exceeded: {} MB", stats.memory_used_mb
        );
    }
    
    println!("Stored {} bytes with {} evictions", total_stored, evictions);
    assert!(evictions > 0, "Should have triggered evictions");
}

#[test]
#[ignore]
fn test_concurrent_memory_pressure() {
    let config = Phase4Config {
        memory_budget_mb: 20,
        ..Default::default()
    };
    
    let cache = Arc::new(Phase4Cache::new(config).unwrap());
    let mut handles = vec![];
    
    // Spawn multiple threads that all try to fill the cache
    for thread_id in 0..8 {
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            let mut parser = Parser::new();
            parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
            
            for i in 0..100 {
                let path = format!("thread_{}_file_{}.rs", thread_id, i);
                let source = generate_large_source(i * thread_id);
                
                let tree = parser.parse(&source, None).unwrap();
                let hash = calculate_hash(&source);
                
                let _ = cache_clone.store(path.into(), hash, tree, source.as_bytes());
                
                // Simulate some work
                thread::sleep(Duration::from_millis(1));
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Verify memory budget was respected
    let stats = cache.get_stats();
    assert!(
        stats.memory_used_mb <= 20.0,
        "Memory budget exceeded under concurrent load: {} MB", 
        stats.memory_used_mb
    );
}

#[test]
#[ignore]
fn test_eviction_policy() {
    let config = Phase4Config {
        memory_budget_mb: 5, // Very small budget
        hot_tier_size_mb: 2,
        warm_tier_size_mb: 2,
        cold_tier_size_mb: 1,
        ..Default::default()
    };
    
    let cache = Phase4Cache::new(config).unwrap();
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    // Store items with different access patterns
    let frequently_accessed = "freq.rs";
    let rarely_accessed = "rare.rs";
    let never_accessed = "never.rs";
    
    // Store all three
    for (path, source) in [
        (frequently_accessed, "fn freq() { /* frequently accessed */ }"),
        (rarely_accessed, "fn rare() { /* rarely accessed */ }"),
        (never_accessed, "fn never() { /* never accessed */ }"),
    ] {
        let tree = parser.parse(source, None).unwrap();
        let hash = calculate_hash(source);
        cache.store(path.into(), hash, tree, source.as_bytes()).unwrap();
    }
    
    // Access frequently_accessed multiple times
    for _ in 0..10 {
        let hash = calculate_hash("fn freq() { /* frequently accessed */ }");
        let _ = cache.get(&frequently_accessed.into(), hash);
        thread::sleep(Duration::from_millis(10));
    }
    
    // Access rarely_accessed once
    let hash = calculate_hash("fn rare() { /* rarely accessed */ }");
    let _ = cache.get(&rarely_accessed.into(), hash);
    
    // Now add more items to trigger eviction
    for i in 0..20 {
        let path = format!("new_{}.rs", i);
        let source = generate_large_source(i);
        let tree = parser.parse(&source, None).unwrap();
        let hash = calculate_hash(&source);
        let _ = cache.store(path.into(), hash, tree, source.as_bytes());
    }
    
    // Check what survived
    let freq_hash = calculate_hash("fn freq() { /* frequently accessed */ }");
    let rare_hash = calculate_hash("fn rare() { /* rarely accessed */ }");
    let never_hash = calculate_hash("fn never() { /* never accessed */ }");
    
    let freq_exists = cache.get(&frequently_accessed.into(), freq_hash).is_ok();
    let rare_exists = cache.get(&rarely_accessed.into(), rare_hash).is_ok();
    let never_exists = cache.get(&never_accessed.into(), never_hash).is_ok();
    
    // Frequently accessed should survive
    assert!(freq_exists, "Frequently accessed item should not be evicted");
    
    // Never accessed should be evicted first
    assert!(!never_exists, "Never accessed item should be evicted");
    
    println!("Eviction policy test passed");
}

#[test]
#[ignore]
fn test_memory_leak_prevention() {
    let config = Phase4Config {
        memory_budget_mb: 50,
        ..Default::default()
    };
    
    // Track memory before
    let initial_memory = get_process_memory_mb();
    
    // Create and destroy cache multiple times
    for iteration in 0..10 {
        let cache = Phase4Cache::new(config.clone()).unwrap();
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        
        // Fill cache
        for i in 0..100 {
            let path = format!("iter_{}_file_{}.rs", iteration, i);
            let source = generate_large_source(i);
            let tree = parser.parse(&source, None).unwrap();
            let hash = calculate_hash(&source);
            let _ = cache.store(path.into(), hash, tree, source.as_bytes());
        }
        
        // Cache goes out of scope and should be cleaned up
    }
    
    // Force garbage collection (in a real scenario)
    thread::sleep(Duration::from_millis(100));
    
    // Check memory after
    let final_memory = get_process_memory_mb();
    let memory_increase = final_memory - initial_memory;
    
    println!("Memory increase after stress test: {} MB", memory_increase);
    
    // Allow some increase but not too much (potential leak)
    assert!(
        memory_increase < 100.0,
        "Potential memory leak detected: {} MB increase", 
        memory_increase
    );
}

#[test]
#[ignore]
fn test_oom_prevention() {
    // Try to allocate more than system memory
    let config = Phase4Config {
        memory_budget_mb: 100, // Reasonable budget
        ..Default::default()
    };
    
    let cache = Phase4Cache::new(config).unwrap();
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    // Try to store massive files
    let mut oom_prevented = false;
    
    for i in 0..1000 {
        let path = format!("massive_{}.rs", i);
        // Generate increasingly large sources
        let source = "fn main() { ".to_string() + &"let x = 42; ".repeat(i * 10000) + "}";
        
        if source.len() > 100 * 1024 * 1024 { // > 100MB single file
            let tree = parser.parse(&source, None).unwrap();
            let hash = calculate_hash(&source);
            
            match cache.store(path.into(), hash, tree, source.as_bytes()) {
                Ok(_) => {
                    panic!("Should not accept files larger than memory budget");
                }
                Err(e) => {
                    println!("Correctly rejected large file: {}", e);
                    oom_prevented = true;
                    break;
                }
            }
        }
    }
    
    assert!(oom_prevented, "Should prevent OOM by rejecting large files");
}

// Helper functions

fn generate_large_source(seed: usize) -> String {
    let mut source = String::from("fn main() {\n");
    for i in 0..seed % 100 + 10 {
        source.push_str(&format!("    let var_{} = {};\n", i, i * seed));
        source.push_str(&format!("    println!(\"Value: {{}}\", var_{});\n", i));
    }
    source.push_str("}\n");
    source
}

fn calculate_hash(source: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

fn get_process_memory_mb() -> f64 {
    use sysinfo::{System, SystemExt, ProcessExt};
    
    let mut system = System::new();
    system.refresh_processes();
    
    let pid = std::process::id();
    if let Some(process) = system.process(pid as i32) {
        process.memory() as f64 / 1024.0 / 1024.0
    } else {
        0.0
    }
}
