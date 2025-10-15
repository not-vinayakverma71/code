/// Lapce AI Rust Implementation
/// High-performance semantic code search and analysis

pub fn init() {
    println!("Lapce AI Rust - Semantic Search initialized");
}

// Core modules
pub mod ai_completion;
pub mod ai_providers;
pub mod ai_tools;
pub mod assistant_message_parser;
pub mod buffer_management;
pub mod core;
pub mod handlers;  // P1-1: Tool execution handlers
pub mod auto_reconnection;
pub mod backoff_util;
pub mod binary_codec;
pub mod bug_detection_ai;
pub mod cache;
pub mod cache_integration;
pub mod caching_strategy;
pub mod circuit_breaker;
pub mod code_chunker;
pub mod code_parser;
pub mod complete_engine;
pub mod concurrent_handler;
// pub mod connection_pool_complete; // Module file doesn't exist
pub mod cross_platform_ipc;
pub mod distributed_search;
pub mod doc_generator_complete;
pub mod embedding_api;
pub mod error_handling;
pub mod error_handling_patterns;
pub mod event_emitter;
pub mod events_exact_translation;
pub mod file_watcher;
pub mod global_sled;
pub mod global_settings_exact_translation;
pub mod handler_registration;
pub mod handler_registration_types;
pub mod hybrid_search;
pub mod ipc_config;
pub mod lancedb_semantic_search;
pub mod ipc;
pub mod ipc_messages;
pub mod ipc_server_complete;
pub mod lapce_plugin;
pub mod lapce_plugin_protocol;
// pub mod macos_shared_memory; // Moved to ipc module
pub mod mcp_tools;
pub mod message_framing;
pub mod message_router;
pub mod message_routing_dispatch;
pub mod metrics_collection;
pub mod mistral_format;
pub mod mock_types;
pub mod model_params;
pub mod multi_language_parser;
pub mod nodejs_comparison;
pub mod openai_format;
// pub mod anthropic_provider_handler; // Module file doesn't exist
// pub mod api_client_complete; // Module file doesn't exist
// pub mod api_provider_integration; // Has compilation issues
// pub mod orchestrator_integration; // Has unresolved dependencies
pub mod optimized_vector_search;
pub mod production_hardening;
pub mod provider_pool;
pub mod query_optimizer;
pub mod r1_format;
pub mod refactoring_engine;
pub mod register_code_actions;
pub mod register_terminal_actions;
pub mod roo_controllers;
pub mod search_files_tool;
pub mod search_tools;
pub mod semantic_engine;
pub mod services;
// pub mod shared_memory_complete; // Moved to ipc module
// pub mod shared_memory_nuclear; // Module file doesn't exist
pub mod simple_format;
pub mod streaming_pipeline;
// streaming_response moved to streaming_pipeline module
pub mod subtask_manager;
pub mod tantivy_search;
pub mod task_connection_handling;
pub mod task_exact_translation;
pub mod task_manager;
pub mod task_orchestration_loop;
pub mod task_orchestrator_metrics;
pub mod task_persistence;
pub mod titan_embedder;
pub mod titan_embedding_client;
pub mod tool_executor;
pub mod tool_repetition_detector;
pub mod types_api;
pub mod types_codebase_index;
pub mod types_events;
pub mod types_experiment;
pub mod types_followup;
pub mod types_global_settings;
pub mod types_history;
pub mod types_ipc;
pub mod types_kilocode;
pub mod types_kilo_languages;
pub mod types_marketplace;
pub mod types_mcp;
pub mod types_message;
pub mod types_model;
pub mod types_mode;
pub mod types_provider_settings;
pub mod types_telemetry;
pub mod types_tool;
pub mod types_vscode;
pub mod token_counting;
// pub mod tools; // Module not implemented yet
pub mod tools_translation;
pub mod types;
// pub mod windows_shared_memory; // Moved to ipc module
pub mod working_cache_system;
pub mod xml_parsing_utils;

// New Connection Pool modules (bb8-based)
pub mod connection_pool_manager;
pub mod https_connection_manager;
pub mod https_connection_manager_real;
pub mod https_pool_wrapper;
pub mod http2_multiplexer;
pub mod connection_reuse;
pub mod websocket_pool_manager;
pub mod connection_metrics;
pub mod geo_routing;
pub mod adaptive_scaler;

// Public exports - commented out until modules are ready
// pub use crate::ai_completion::*;
// pub use crate::ai_providers::*;
pub use crate::ipc::ipc_server::IpcServer;
pub use crate::ipc_config::IpcConfig;
pub use ipc_messages::{ClineMessage, ClineAsk, ClineAskResponse};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use auto_reconnection::{AutoReconnectionManager, ReconnectionStrategy, ConnectionState};
