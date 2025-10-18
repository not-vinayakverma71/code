//! Performance Benchmarks for Prompt System (P15)
//!
//! Targets:
//! - Build prompt: <50ms warm
//! - Custom instructions load: <10ms
//! - Tool descriptions: <5ms
//!
//! Run with: cargo bench --bench prompt_benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use lapce_ai_rust::core::prompt::{
    builder::PromptBuilder,
    modes::get_mode_by_slug,
    settings::SystemPromptSettings,
    sections::custom_instructions::add_custom_instructions,
    tools::{get_tool_descriptions_for_mode, ToolDescriptionContext},
};

// ============================================================================
// Full Prompt Build Benchmarks
// ============================================================================

fn bench_prompt_build_code_mode(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    c.bench_function("prompt_build_code_mode", |b| {
        b.to_async(&rt).iter(|| async {
            let builder = PromptBuilder::new(mode.clone(), workspace.clone(), settings.clone());
            let result = builder.build().await;
            black_box(result.unwrap())
        });
    });
}

fn bench_prompt_build_all_modes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    
    let modes = vec!["code", "architect", "ask", "debug", "orchestrator"];
    
    let mut group = c.benchmark_group("prompt_build_by_mode");
    
    for mode_slug in modes {
        let mode = get_mode_by_slug(mode_slug).unwrap();
        let settings = SystemPromptSettings::default();
        
        group.bench_with_input(
            BenchmarkId::from_parameter(mode_slug),
            &mode_slug,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let builder = PromptBuilder::new(mode.clone(), workspace.clone(), settings.clone());
                    let result = builder.build().await;
                    black_box(result.unwrap())
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Custom Instructions Load Benchmarks
// ============================================================================

fn bench_custom_instructions_empty(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let settings = SystemPromptSettings::default();
    
    c.bench_function("custom_instructions_empty", |b| {
        b.to_async(&rt).iter(|| async {
            let result = add_custom_instructions(
                "",
                "",
                workspace,
                "code",
                None,
                None,
                &settings,
            ).await;
            black_box(result.unwrap())
        });
    });
}

fn bench_custom_instructions_with_rules(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create rules directory with files
    rt.block_on(async {
        let rules_dir = workspace.join(".kilocode/rules");
        tokio::fs::create_dir_all(&rules_dir).await.unwrap();
        
        // Create 5 rule files
        for i in 1..=5 {
            tokio::fs::write(
                rules_dir.join(format!("rule{}.txt", i)),
                format!("Rule {} content here", i)
            ).await.unwrap();
        }
        
        // Create AGENTS.md
        tokio::fs::write(
            workspace.join("AGENTS.md"),
            "# Agent Rules\n\nBe helpful and concise."
        ).await.unwrap();
    });
    
    let settings = SystemPromptSettings::default();
    
    c.bench_function("custom_instructions_with_rules", |b| {
        b.to_async(&rt).iter(|| async {
            let result = add_custom_instructions(
                "",
                "",
                workspace,
                "code",
                None,
                None,
                &settings,
            ).await;
            black_box(result.unwrap())
        });
    });
}

fn bench_custom_instructions_large_workspace(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    // Create larger workspace with nested rules
    rt.block_on(async {
        let rules_dir = workspace.join(".kilocode/rules");
        tokio::fs::create_dir_all(&rules_dir).await.unwrap();
        
        // Create 20 rule files
        for i in 1..=20 {
            tokio::fs::write(
                rules_dir.join(format!("rule{:02}.txt", i)),
                format!("Rule {} with more content to simulate realistic file sizes\n", i).repeat(10)
            ).await.unwrap();
        }
        
        // Create mode-specific rules
        let code_rules = workspace.join(".kilocode/rules-code");
        tokio::fs::create_dir_all(&code_rules).await.unwrap();
        for i in 1..=5 {
            tokio::fs::write(
                code_rules.join(format!("code_rule{}.txt", i)),
                format!("Code-specific rule {}\n", i)
            ).await.unwrap();
        }
        
        // Create AGENTS.md
        tokio::fs::write(
            workspace.join("AGENTS.md"),
            "# Agent Rules\n\n".to_string() + &"Be helpful.\n".repeat(50)
        ).await.unwrap();
    });
    
    let settings = SystemPromptSettings::default();
    
    c.bench_function("custom_instructions_large_workspace", |b| {
        b.to_async(&rt).iter(|| async {
            let result = add_custom_instructions(
                "",
                "",
                workspace,
                "code",
                None,
                None,
                &settings,
            ).await;
            black_box(result.unwrap())
        });
    });
}

// ============================================================================
// Tool Descriptions Benchmarks
// ============================================================================

fn bench_tool_descriptions_code_mode(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    
    c.bench_function("tool_descriptions_code_mode", |b| {
        b.iter(|| {
            let context = ToolDescriptionContext {
                workspace,
                supports_browser: false,
                codebase_search_available: false,
                fast_apply_available: false,
                max_concurrent_file_reads: 5,
                partial_reads_enabled: true,
                todo_list_enabled: false,
                image_generation_enabled: false,
                run_slash_command_enabled: false,
                browser_viewport_size: "1920x1080",
                new_task_require_todos: false,
            };
            
            let result = get_tool_descriptions_for_mode(&mode, &context);
            black_box(result)
        });
    });
}

fn bench_tool_descriptions_all_modes(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let modes = vec!["code", "architect", "ask", "debug", "orchestrator"];
    
    let mut group = c.benchmark_group("tool_descriptions_by_mode");
    
    for mode_slug in modes {
        let mode = get_mode_by_slug(mode_slug).unwrap();
        
        group.bench_with_input(
            BenchmarkId::from_parameter(mode_slug),
            &mode_slug,
            |b, _| {
                b.iter(|| {
                    let context = ToolDescriptionContext {
                        workspace,
                        supports_browser: false,
                        codebase_search_available: false,
                        fast_apply_available: false,
                        max_concurrent_file_reads: 5,
                        partial_reads_enabled: true,
                        todo_list_enabled: false,
                        image_generation_enabled: false,
                        run_slash_command_enabled: false,
                        browser_viewport_size: "1920x1080",
                        new_task_require_todos: false,
                    };
                    
                    let result = get_tool_descriptions_for_mode(&mode, &context);
                    black_box(result)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_tool_descriptions_with_all_features(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    
    let mode = get_mode_by_slug("code").unwrap();
    
    c.bench_function("tool_descriptions_all_features_enabled", |b| {
        b.iter(|| {
            let context = ToolDescriptionContext {
                workspace,
                supports_browser: true,
                codebase_search_available: true,
                fast_apply_available: true,
                max_concurrent_file_reads: 100,
                partial_reads_enabled: true,
                todo_list_enabled: true,
                image_generation_enabled: true,
                run_slash_command_enabled: true,
                browser_viewport_size: "1920x1080",
                new_task_require_todos: true,
            };
            
            let result = get_tool_descriptions_for_mode(&mode, &context);
            black_box(result)
        });
    });
}

// ============================================================================
// Retry Mechanism Benchmark
// ============================================================================

fn bench_prompt_build_with_retry(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    
    let mode = get_mode_by_slug("code").unwrap();
    let settings = SystemPromptSettings::default();
    
    c.bench_function("prompt_build_with_retry", |b| {
        b.to_async(&rt).iter(|| async {
            let builder = PromptBuilder::new(mode.clone(), workspace.clone(), settings.clone());
            let result = builder.build_with_retry().await;
            black_box(result.unwrap())
        });
    });
}

// ============================================================================
// Token Estimation Benchmark
// ============================================================================

fn bench_token_estimation(c: &mut Criterion) {
    let prompts = vec![
        "Short prompt",
        "Medium length prompt with some content ".repeat(10),
        "Very long prompt with lots of content ".repeat(100),
    ];
    
    let mut group = c.benchmark_group("token_estimation");
    
    for (i, prompt) in prompts.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("length_{}", prompt.len())),
            &i,
            |b, _| {
                b.iter(|| {
                    // Simple token estimation: chars / 4
                    let tokens = prompt.len() / 4;
                    black_box(tokens)
                });
            },
        );
    }
    
    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    prompt_build_benches,
    bench_prompt_build_code_mode,
    bench_prompt_build_all_modes,
    bench_prompt_build_with_retry,
);

criterion_group!(
    custom_instructions_benches,
    bench_custom_instructions_empty,
    bench_custom_instructions_with_rules,
    bench_custom_instructions_large_workspace,
);

criterion_group!(
    tool_descriptions_benches,
    bench_tool_descriptions_code_mode,
    bench_tool_descriptions_all_modes,
    bench_tool_descriptions_with_all_features,
);

criterion_group!(
    misc_benches,
    bench_token_estimation,
);

criterion_main!(
    prompt_build_benches,
    custom_instructions_benches,
    tool_descriptions_benches,
    misc_benches,
);
