/// Lapce AI Rust Implementation
/// High-performance semantic code search and analysis

pub fn init() {
    println!("Lapce AI Rust - Semantic Search initialized");
}

// Core modules
pub mod ai_completion;
pub mod ai_providers;
pub mod ai_tools;
pub mod auto_reconnection;
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
pub mod connection_pool_complete;
pub mod cross_platform_ipc;
pub mod distributed_search;
pub mod doc_generator_complete;
pub mod embedding_api;
pub mod error_handling;
pub mod error_handling_patterns;
pub mod event_emitter;
pub mod events_exact_translation;
pub mod file_watcher;
pub mod global_settings_exact_translation;
pub mod handler_registration;
pub mod handler_registration_types;
pub mod hybrid_search;
pub mod ipc_config;
pub mod ipc_messages;
pub mod ipc_server;
pub mod ipc_server_complete;
pub mod lapce_plugin;
pub mod lapce_plugin_protocol;
pub mod macos_shared_memory;
pub mod mcp_tools;
pub mod message_framing;
pub mod message_routing_dispatch;
pub mod metrics_collection;
pub mod mistral_format;
pub mod mock_types;
pub mod model_params;
pub mod multi_language_parser;
pub mod nodejs_comparison;
pub mod openai_format;
pub mod optimized_cache;
pub mod optimized_vector_search;
pub mod production_hardening;
pub mod provider_pool;
pub mod query_optimizer;
pub mod r1_format;
pub mod refactoring_engine;
pub mod register_code_actions;
pub mod register_terminal_actions;
pub mod search_files_tool;
pub mod search_tools;
pub mod semantic_engine;
pub mod services;
pub mod shared_memory_complete;
pub mod shared_memory_nuclear;
pub mod simple_format;
pub mod streaming_pipeline;
pub mod streaming_response;
pub mod tantivy_search;
pub mod task_connection_handling;
pub mod task_exact_translation;
pub mod test_generator_complete;
pub mod titan_embedding_client;
pub mod token_counting;
pub mod tools;
pub mod tools_translation;
pub mod types;
pub mod types_message;
pub mod types_tool;
pub mod windows_shared_memory;
pub mod working_cache_system;
pub mod xml_parsing_utils;

// New Connection Pool modules (bb8-based)
pub mod connection_pool_manager;
pub mod https_connection_manager;
pub mod websocket_pool_manager;
pub mod connection_metrics;
pub mod geo_routing;
pub mod adaptive_scaler;

// Public exports
pub use crate::ai_completion::*;
pub use crate::ai_providers::*;
pub use crate::ipc_server::IpcServer;
pub use crate::ipc_config::IpcConfig;
pub use ipc_messages::{ClineMessage, ClineAsk, ClineAskResponse};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use auto_reconnection::{AutoReconnectionManager, ReconnectionStrategy, ConnectionState};
