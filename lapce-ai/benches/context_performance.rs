// Context System Performance Benchmarks
// Part of PERF-31: Benchmark token counting and sliding window operations
// Targets: <10ms token counting, <50ms sliding window prep

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lapce_ai_rust::core::token_counter;
use lapce_ai_rust::core::model_limits::{get_model_limits, get_reserved_tokens};
use lapce_ai_rust::core::sliding_window::{ApiMessage, MessageContent, ContentBlock, estimate_token_count};

fn benchmark_token_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_counting");
    
    // Test data of varying sizes
    let small_text = "Hello, world!";
    let medium_text = "fn main() {\n    println!(\"Hello, world!\");\n}\n".repeat(10);
    let large_text = "The quick brown fox jumps over the lazy dog. ".repeat(100);
    let very_large_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000);
    
    // Benchmark small text
    group.bench_function("count_tokens_small", |b| {
        b.iter(|| {
            token_counter::count_tokens(
                black_box(small_text),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    // Benchmark medium text (~100 tokens)
    group.bench_function("count_tokens_medium", |b| {
        b.iter(|| {
            token_counter::count_tokens(
                black_box(&medium_text),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    // Benchmark large text (~900 tokens)
    group.bench_function("count_tokens_large", |b| {
        b.iter(|| {
            token_counter::count_tokens(
                black_box(&large_text),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    // Benchmark very large text (~10K tokens)
    group.bench_function("count_tokens_very_large", |b| {
        b.iter(|| {
            token_counter::count_tokens(
                black_box(&very_large_text),
                black_box("gpt-4o")
            )
        });
    });
    
    // Benchmark with different encoders
    group.bench_function("count_tokens_anthropic_cl100k", |b| {
        b.iter(|| {
            token_counter::count_tokens(
                black_box(&medium_text),
                black_box("claude-sonnet-4-5")
            )
        });
    });
    
    group.bench_function("count_tokens_openai_o200k", |b| {
        b.iter(|| {
            token_counter::count_tokens(
                black_box(&medium_text),
                black_box("gpt-5-2025-08-07")
            )
        });
    });
    
    // Benchmark batch counting
    let batch_texts: Vec<String> = (0..100)
        .map(|i| format!("This is test message number {}", i))
        .collect();
    
    group.bench_function("count_tokens_batch_100", |b| {
        b.iter(|| {
            token_counter::count_tokens_batch(
                black_box(&batch_texts),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    group.finish();
}

fn benchmark_encoder_caching(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoder_caching");
    
    let text = "fn main() { println!(\"test\"); }";
    
    // First call (cache miss)
    group.bench_function("first_call_cache_miss", |b| {
        b.iter(|| {
            token_counter::clear_cache();
            token_counter::count_tokens(
                black_box(text),
                black_box("new-model-test")
            )
        });
    });
    
    // Subsequent calls (cache hit)
    token_counter::count_tokens(text, "claude-3-5-sonnet-20241022").unwrap();
    group.bench_function("subsequent_call_cache_hit", |b| {
        b.iter(|| {
            token_counter::count_tokens(
                black_box(text),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    group.finish();
}

fn benchmark_model_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_limits");
    
    // Benchmark model limits lookup (should be <1µs)
    group.bench_function("get_model_limits", |b| {
        b.iter(|| {
            get_model_limits(black_box("claude-3-5-sonnet-20241022"))
        });
    });
    
    group.bench_function("get_reserved_tokens", |b| {
        b.iter(|| {
            get_reserved_tokens(
                black_box("gpt-4o"),
                black_box(Some(8192))
            )
        });
    });
    
    // Benchmark with different models
    let models = vec![
        "claude-3-5-sonnet-20241022",
        "claude-sonnet-4-5",
        "gpt-4o",
        "gpt-5-2025-08-07",
        "o1-preview",
    ];
    
    for model in models {
        group.bench_with_input(BenchmarkId::new("lookup", model), &model, |b, model| {
            b.iter(|| get_model_limits(black_box(model)));
        });
    }
    
    group.finish();
}

fn benchmark_estimate_token_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("estimate_token_count");
    
    // Create test content blocks
    let single_text = vec![ContentBlock::Text {
        text: "Hello, world!".to_string(),
    }];
    
    let multiple_blocks = vec![
        ContentBlock::Text {
            text: "fn main() { println!(\"test\"); }".to_string(),
        },
        ContentBlock::Text {
            text: "This is another text block with more content.".to_string(),
        },
        ContentBlock::Text {
            text: "And a third block for good measure.".to_string(),
        },
    ];
    
    let mixed_blocks = vec![
        ContentBlock::Text {
            text: "Some text content".to_string(),
        },
        ContentBlock::Image {
            source: lapce_ai_rust::core::sliding_window::ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: "...".to_string(),
            },
        },
        ContentBlock::ToolUse {
            id: "tool_1".to_string(),
            name: "read_file".to_string(),
            input: serde_json::json!({"path": "test.rs"}),
        },
    ];
    
    group.bench_function("single_text_block", |b| {
        b.iter(|| {
            estimate_token_count(
                black_box(&single_text),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    group.bench_function("multiple_text_blocks", |b| {
        b.iter(|| {
            estimate_token_count(
                black_box(&multiple_blocks),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    group.bench_function("mixed_content_blocks", |b| {
        b.iter(|| {
            estimate_token_count(
                black_box(&mixed_blocks),
                black_box("claude-3-5-sonnet-20241022")
            )
        });
    });
    
    group.finish();
}

fn benchmark_sliding_window_prep(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_window_prep");
    
    // Create test conversation with varying sizes
    fn create_messages(count: usize) -> Vec<ApiMessage> {
        (0..count).map(|i| {
            ApiMessage {
                role: if i % 2 == 0 { "user".to_string() } else { "assistant".to_string() },
                content: MessageContent::Text(
                    format!("This is message number {} with some content to simulate a real conversation.", i)
                ),
                ts: Some(i as u64),
                is_summary: None,
            }
        }).collect()
    }
    
    let small_conversation = create_messages(10);
    let medium_conversation = create_messages(50);
    let large_conversation = create_messages(100);
    let very_large_conversation = create_messages(500);
    
    // Benchmark token counting for entire conversation
    group.bench_function("count_10_messages", |b| {
        b.iter(|| {
            let mut total = 0;
            for msg in black_box(&small_conversation) {
                if let MessageContent::Text(text) = &msg.content {
                    total += token_counter::count_tokens(text, "claude-3-5-sonnet-20241022").unwrap();
                }
            }
            total
        });
    });
    
    group.bench_function("count_50_messages", |b| {
        b.iter(|| {
            let mut total = 0;
            for msg in black_box(&medium_conversation) {
                if let MessageContent::Text(text) = &msg.content {
                    total += token_counter::count_tokens(text, "claude-3-5-sonnet-20241022").unwrap();
                }
            }
            total
        });
    });
    
    group.bench_function("count_100_messages", |b| {
        b.iter(|| {
            let mut total = 0;
            for msg in black_box(&large_conversation) {
                if let MessageContent::Text(text) = &msg.content {
                    total += token_counter::count_tokens(text, "claude-3-5-sonnet-20241022").unwrap();
                }
            }
            total
        });
    });
    
    group.bench_function("count_500_messages", |b| {
        b.iter(|| {
            let mut total = 0;
            for msg in black_box(&very_large_conversation) {
                if let MessageContent::Text(text) = &msg.content {
                    total += token_counter::count_tokens(text, "claude-3-5-sonnet-20241022").unwrap();
                }
            }
            total
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_token_counting,
    benchmark_encoder_caching,
    benchmark_model_limits,
    benchmark_estimate_token_count,
    benchmark_sliding_window_prep
);
criterion_main!(benches);
