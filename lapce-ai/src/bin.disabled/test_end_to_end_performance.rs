/// End-to-end Integration and Performance Test
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json;

// Import all IPC modules
use lapce_ai_rust::ipc_messages::*;
use lapce_ai_rust::events_exact_translation::*;
use lapce_ai_rust::tools_translation::*;
use lapce_ai_rust::global_settings_exact_translation::*;

// Import performance modules
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer as lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;
use lapce_ai_rust::optimized_cache::OptimizedCache;
use lapce_ai_rust::optimized_vector_search::OptimizedVectorSearch;
// use lapce_ai_rust::connection_pool_complete::ConnectionPool as WorkingConnectionPool; // Module doesn't exist

#[tokio::main]
async fn main() {
    println!("🚀 END-TO-END SYSTEM PERFORMANCE TEST");
    println!("=====================================\n");
    
    // Test 1: IPC Protocol Serialization Performance
    println!("📊 IPC PROTOCOL PERFORMANCE:");
    test_ipc_performance().await;
    
    // Test 2: SharedMemory Transport Performance  
    println!("\n📊 SHARED MEMORY TRANSPORT:");
    test_shared_memory_performance().await;
    
    // Test 3: Cache System Performance
    println!("\n📊 CACHE SYSTEM PERFORMANCE:");
    test_cache_performance().await;
    
    // Test 4: Vector Search Performance
    println!("\n📊 VECTOR SEARCH PERFORMANCE:");
    test_vector_search_performance().await;
    
    // Test 5: Connection Pool Performance
    println!("\n📊 CONNECTION POOL PERFORMANCE:");
    test_connection_pool_performance().await;
    
    // Test 6: Combined System Performance
    println!("\n📊 COMBINED SYSTEM PERFORMANCE:");
    test_combined_system().await;
    
    println!("\n✅ ALL PERFORMANCE TESTS COMPLETED!");
}

async fn test_ipc_performance() {
    let iterations = 100_000;
    
    // Test ClineMessage serialization
    let msg = lapce_ai_rust::ipc_messages::ClineMessage {
        ts: 1234567890,
        msg_type: "ask".to_string(),
        ask: Some(ClineAsk::Command),
        say: None,
        text: Some("Execute this command?".to_string()),
        images: None,
        partial: Some(false),
        reasoning: Some("User requested action".to_string()),
        conversation_history_index: Some(10),
        checkpoint: None,
        progress_status: None,
        context_condense: None,
        is_protected: Some(false),
        api_protocol: Some("openai".to_string()),
        metadata: None,
    };
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = serde_json::to_vec(&msg).unwrap();
    }
    let duration = start.elapsed();
    
    let msgs_per_sec = iterations as f64 / duration.as_secs_f64();
    let avg_time_us = duration.as_micros() as f64 / iterations as f64;
    
    println!("  • ClineMessage serialization:");
    println!("    - Throughput: {:.2}M msg/sec", msgs_per_sec / 1_000_000.0);
    println!("    - Latency: {:.2}μs/msg", avg_time_us);
    
    // Test TaskEvent serialization
    let event = crate::event_emitter::TaskEvent::TaskStarted {
        payload: ("task-123".to_string(),),
        task_id: Some(42),
    };
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = serde_json::to_vec(&event).unwrap();
    }
    let duration = start.elapsed();
    
    let events_per_sec = iterations as f64 / duration.as_secs_f64();
    let avg_time_us = duration.as_micros() as f64 / iterations as f64;
    
    println!("  • TaskEvent serialization:");
    println!("    - Throughput: {:.2}M events/sec", events_per_sec / 1_000_000.0);
    println!("    - Latency: {:.2}μs/event", avg_time_us);
}

async fn test_shared_memory_performance() {
    let iterations = 1_000_000;
    let message_size = 256; // bytes
    let data = vec![0u8; message_size];
    
    // Create shared memory
    let mut shm = lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("test_perf", 4 * 1024 * 1024).unwrap();
    
    // Write test
    let start = Instant::now();
    for _ in 0..iterations {
        shm.write(&data).unwrap();
    }
    let write_duration = start.elapsed();
    
    let write_throughput = iterations as f64 / write_duration.as_secs_f64();
    let write_latency = write_duration.as_nanos() as f64 / iterations as f64;
    
    println!("  • Write performance:");
    println!("    - Throughput: {:.2}M msg/sec", write_throughput / 1_000_000.0);
    println!("    - Latency: {:.0}ns/msg", write_latency);
    println!("    - Bandwidth: {:.2} GB/s", (message_size * iterations) as f64 / write_duration.as_secs_f64() / 1_000_000_000.0);
    
    // Read test
    let mut buffer = vec![0u8; message_size];
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = shm.read(&mut buffer);
    }
    let read_duration = start.elapsed();
    
    let read_throughput = iterations as f64 / read_duration.as_secs_f64();
    let read_latency = read_duration.as_nanos() as f64 / iterations as f64;
    
    println!("  • Read performance:");
    println!("    - Throughput: {:.2}M msg/sec", read_throughput / 1_000_000.0);
    println!("    - Latency: {:.0}ns/msg", read_latency);
}

async fn test_cache_performance() {
    let cache = OptimizedCache::new();
    
    let iterations = 100_000;
    let key = "test_key".to_string();
    let value = vec![0u8; 1024]; // 1KB value
    
    // Write test
    let start = Instant::now();
    for i in 0..iterations {
        let k = format!("{}{}", key, i);
        cache.set(k, value.clone()).await;
    }
    let write_duration = start.elapsed();
    
    let write_ops_per_sec = iterations as f64 / write_duration.as_secs_f64();
    
    println!("  • Write performance:");
    println!("    - Throughput: {:.2}M ops/sec", write_ops_per_sec / 1_000_000.0);
    println!("    - Latency: {:.2}μs/op", write_duration.as_micros() as f64 / iterations as f64);
    
    // Read test (cache hits)
    let start = Instant::now();
    for i in 0..iterations {
        let k = format!("{}{}", key, i % 1000); // Read first 1000 entries repeatedly
        let _ = cache.get(&k).await;
    }
    let read_duration = start.elapsed();
    
    let read_ops_per_sec = iterations as f64 / read_duration.as_secs_f64();
    
    println!("  • Read performance (cache hits):");
    println!("    - Throughput: {:.2}M ops/sec", read_ops_per_sec / 1_000_000.0);
    println!("    - Latency: {:.2}μs/op", read_duration.as_micros() as f64 / iterations as f64);
}

async fn test_vector_search_performance() {
    let mut search = OptimizedVectorSearch::new(384).unwrap();
    
    // Add vectors
    let num_vectors = 10_000;
    let start = Instant::now();
    for i in 0..num_vectors {
        let vector = vec![0.1; 128];
        search.add(format!("doc_{}", i), vector).unwrap();
    }
    let index_duration = start.elapsed();
    
    println!("  • Indexing performance:");
    println!("    - Indexed {} vectors in {:.2}s", num_vectors, index_duration.as_secs_f64());
    println!("    - Rate: {:.0} vectors/sec", num_vectors as f64 / index_duration.as_secs_f64());
    
    // Search test
    let query_vector = vec![0.1; 128];
    let iterations = 1000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = search.search(&query_vector, 10).unwrap();
    }
    let search_duration = start.elapsed();
    
    let search_qps = iterations as f64 / search_duration.as_secs_f64();
    let search_latency = search_duration.as_micros() as f64 / iterations as f64;
    
    println!("  • Search performance:");
    println!("    - QPS: {:.0} queries/sec", search_qps);
    println!("    - Latency: {:.0}μs/query", search_latency);
}

async fn test_connection_pool_performance() {
    // Test connection pool creation and management
    let num_connections = 1000;
    let start = Instant::now();
    
    // Create pool
    let _pool = WorkingConnectionPool::new(num_connections);
    let creation_time = start.elapsed();
    
    println!("  • Connection pool performance:");
    println!("    - Pool size: {} connections", num_connections);
    println!("    - Creation time: {:.2}ms", creation_time.as_millis());
    
    // Simulate concurrent operations
    let operations = 10_000;
    let start = Instant::now();
    
    for _ in 0..operations {
        // Simulate connection work
        tokio::time::sleep(Duration::from_nanos(100)).await;
    }
    
    let duration = start.elapsed();
    let ops_per_sec = operations as f64 / duration.as_secs_f64();
    
    println!("    - Operations: {}", operations);
    println!("    - Throughput: {:.0} ops/sec", ops_per_sec);
    println!("    - Avg latency: {:.2}μs/op", duration.as_micros() as f64 / operations as f64);
}

async fn test_combined_system() {
    // Simulate real-world IPC message flow
    let start = Instant::now();
    let iterations = 10_000;
    
    for i in 0..iterations {
        // 1. Create IPC message
        let msg = TaskCommand::StartNewTask {
                data: StartNewTaskData {
                    configuration: RooCodeSettings {
                        global: GlobalSettings {
                            mode: Some("code".to_string()),
                            ..default_global_settings()
                        },
                        provider: ProviderSettings::default(),
                    },
                    text: format!("Task {}", i),
                    images: None,
                    new_tab: Some(false),
                },
        };
        
        // 2. Serialize
        let serialized = serde_json::to_vec(&msg).unwrap();
        
        // 3. Would send through SharedMemory (simulated)
        let _ = serialized.len();
        
        // 4. Create response
        let response = Ack {
            client_id: format!("client-{}", i),
            pid: std::process::id(),
            ppid: 1,
        };
        
        // 5. Serialize response
        let _ = serde_json::to_vec(&response).unwrap();
    }
    
    let duration = start.elapsed();
    let requests_per_sec = iterations as f64 / duration.as_secs_f64();
    
    println!("  • Full IPC round-trip simulation:");
    println!("    - Iterations: {}", iterations);
    println!("    - Total time: {:.2}s", duration.as_secs_f64());
    println!("    - Throughput: {:.0} req/sec", requests_per_sec);
    println!("    - Latency: {:.2}μs/req", duration.as_micros() as f64 / iterations as f64);
}

// Helper function
fn default_global_settings() -> GlobalSettings {
    GlobalSettings {
        current_api_config_name: None,
        list_api_config_meta: None,
        pinned_api_configs: None,
        last_shown_announcement_id: None,
        custom_instructions: None,
        task_history: None,
        condensing_api_config_id: None,
        custom_condensing_prompt: None,
        auto_approval_enabled: None,
        always_allow_read_only: None,
        always_allow_read_only_outside_workspace: None,
        always_allow_write: None,
        always_allow_write_outside_workspace: None,
        always_allow_write_protected: None,
        write_delay_ms: None,
        always_allow_browser: None,
        always_approve_resubmit: None,
        request_delay_seconds: None,
        always_allow_mcp: None,
        always_allow_mode_switch: None,
        always_allow_subtasks: None,
        always_allow_execute: None,
        always_allow_followup_questions: None,
        followup_auto_approve_timeout_ms: None,
        always_allow_update_todo_list: None,
        allowed_commands: None,
        denied_commands: None,
        command_execution_timeout: None,
        command_timeout_allowlist: None,
        prevent_completion_with_open_todos: None,
        allowed_max_requests: None,
        allowed_max_cost: None,
        auto_condense_context: None,
        auto_condense_context_percent: None,
        max_concurrent_file_reads: None,
        allow_very_large_reads: None,
        include_diagnostic_messages: None,
        max_diagnostic_messages: None,
        browser_tool_enabled: None,
        browser_viewport_size: None,
        show_auto_approve_menu: None,
        show_task_timeline: None,
        local_workflow_toggles: None,
        global_workflow_toggles: None,
        local_rules_toggles: None,
        global_rules_toggles: None,
        screenshot_quality: None,
        remote_browser_enabled: None,
        remote_browser_host: None,
        cached_chrome_host_url: None,
        enable_checkpoints: None,
        tts_enabled: None,
        tts_speed: None,
        sound_enabled: None,
        sound_volume: None,
        system_notifications_enabled: None,
        max_open_tabs_context: None,
        max_workspace_files: None,
        show_roo_ignored_files: None,
        max_read_file_line: None,
        max_image_file_size: None,
        max_total_image_size: None,
        terminal_output_line_limit: None,
        terminal_output_character_limit: None,
        terminal_shell_integration_timeout: None,
        terminal_shell_integration_disabled: None,
        terminal_command_delay: None,
        terminal_powershell_counter: None,
        terminal_zsh_clear_eol_mark: None,
        terminal_zsh_oh_my: None,
        terminal_zsh_p10k: None,
        terminal_zdotdir: None,
        terminal_compress_progress_bar: None,
        diagnostics_enabled: None,
        rate_limit_seconds: None,
        diff_enabled: None,
        fuzzy_match_threshold: None,
        experiments: None,
        morph_api_key: None,
        codebase_index_models: None,
        codebase_index_config: None,
        language: None,
        telemetry_setting: None,
        mcp_enabled: None,
        enable_mcp_server_creation: None,
        mcp_marketplace_catalog: None,
        remote_control_enabled: None,
        mode: None,
        mode_api_configs: None,
        custom_modes: None,
        custom_mode_prompts: None,
        custom_support_prompts: None,
        enhancement_api_config_id: None,
        dismissed_notification_ids: None,
        commit_message_api_config_id: None,
        terminal_command_api_config_id: None,
        ghost_service_settings: GhostServiceSettings {
            enabled: None,
            api_key: None,
            model: None,
        },
        include_task_history_in_enhance: None,
        history_preview_collapsed: None,
        profile_thresholds: None,
        has_opened_mode_selector: None,
        last_mode_export_path: None,
        last_mode_import_path: None,
    }
}
