# Step 19: Comprehensive Testing Framework
## Unit, Integration, and Performance Testing

## ⚠️ CRITICAL: TEST THE TRANSLATION, NOT NEW CODE
**VERIFY 1:1 TYPESCRIPT → RUST PORT ACCURACY**

**TEST AGAINST**:
- `/home/verma/lapce/Codex` - All TypeScript outputs
- Character-for-character response matching
- Same inputs → Same outputs ALWAYS
- Years of edge cases already handled

## ✅ Success Criteria
- [ ] **Test Coverage**: > 95% code coverage
- [ ] **Translation Accuracy**: 100% output match with TypeScript
- [ ] **Performance Tests**: 10x faster than Node.js
- [ ] **Memory Tests**: < 350MB total usage
- [ ] **Load Tests**: 1K concurrent operations
- [ ] **Regression Tests**: Zero behavior changes
- [ ] **Fuzz Testing**: 10K+ random inputs
- [ ] **CI/CD Integration**: All tests automated

## Overview
Testing verifies that our Rust translation produces IDENTICAL behavior to the TypeScript original.

## Unit Testing Strategy

### Test Every Function Translation
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    
    #[test]
    fn test_token_counting_matches() {
        // Load TypeScript test cases
        let test_cases: Vec<TokenTestCase> = 
            load_typescript_fixtures("token_count_tests.json");
        
        for case in test_cases {
            let rust_count = count_tokens(&case.text, &case.model);
            assert_eq!(
                rust_count, case.expected_count,
                "Token count mismatch for: {}", case.text
            );
        }
    }
    
    #[test]
    fn test_message_formatting_exact() {
        // Compare message formatting CHARACTER-BY-CHARACTER
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there"),
        ];
        
        let rust_formatted = format_messages(&messages);
        let ts_formatted = load_typescript_output("formatted_messages.txt");
        
        assert_eq!(rust_formatted, ts_formatted);
    }
}
```

## Integration Testing

### Provider Response Comparison
```rust
#[tokio::test]
async fn test_openai_provider_identical() {
    let provider = OpenAIProvider::new(get_test_key());
    let request = CompletionRequest {
        messages: vec![Message::user("Test")],
        model: "gpt-4".to_string(),
        temperature: 0.0, // Deterministic
    };
    
    // Mock the HTTP response
    let mock_response = load_typescript_response("openai_response.json");
    
    let rust_response = provider.complete_with_mock(request, mock_response).await?;
    let ts_response = load_typescript_output("openai_parsed.json");
    
    assert_json_eq!(rust_response, ts_response);
}

#[tokio::test]
async fn test_all_tools_work_exactly() {
    let tools = load_all_tools();
    
    for tool_name in TOOL_NAMES {
        let test_cases = load_typescript_tests(&format!("{}_tests.json", tool_name));
        
        for case in test_cases {
            let rust_result = tools.execute(tool_name, case.params).await?;
            assert_eq!(
                rust_result, case.expected_output,
                "Tool {} failed on case: {:?}", tool_name, case.name
            );
        }
    }
}
```

## Performance Benchmarking

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_ipc_throughput(c: &mut Criterion) {
    c.bench_function("ipc_message_roundtrip", |b| {
        b.iter(|| {
            let msg = create_test_message();
            let serialized = serialize(black_box(msg));
            let deserialized = deserialize(black_box(serialized));
            deserialized
        });
    });
}

fn benchmark_token_counting(c: &mut Criterion) {
    let text = "A".repeat(10000); // 10K character string
    
    c.bench_function("count_10k_chars", |b| {
        b.iter(|| count_tokens(black_box(&text), "gpt-4"));
    });
}

fn benchmark_vs_nodejs(c: &mut Criterion) {
    let mut group = c.benchmark_group("rust_vs_nodejs");
    
    // Run Node.js version
    group.bench_function("nodejs", |b| {
        b.iter(|| {
            std::process::Command::new("node")
                .arg("benchmark.js")
                .output()
                .unwrap()
        });
    });
    
    // Run Rust version
    group.bench_function("rust", |b| {
        b.iter(|| run_rust_benchmark());
    });
    
    group.finish();
}

criterion_group!(benches, 
    benchmark_ipc_throughput,
    benchmark_token_counting,
    benchmark_vs_nodejs
);
criterion_main!(benches);
```

## Memory Testing

```rust
#[test]
fn test_memory_usage_under_limit() {
    use jemalloc_ctl::{stats, epoch};
    
    // Force collection
    epoch::mib().unwrap().advance().unwrap();
    
    let baseline = stats::allocated::mib().unwrap().read().unwrap();
    
    // Run full system
    let system = create_full_system();
    system.process_large_workload();
    
    epoch::mib().unwrap().advance().unwrap();
    let after = stats::allocated::mib().unwrap().read().unwrap();
    
    let used = after - baseline;
    
    assert!(
        used < 350 * 1024 * 1024, // 350MB
        "Memory usage {} exceeds limit", used
    );
}
```

## Load Testing

```rust
#[tokio::test]
async fn test_concurrent_operations() {
    let system = Arc::new(System::new());
    let mut handles = vec![];
    
    // Spawn 1000 concurrent operations
    for i in 0..1000 {
        let sys = system.clone();
        handles.push(tokio::spawn(async move {
            sys.process_request(create_test_request(i)).await
        }));
    }
    
    // Wait for all
    let results = futures::future::join_all(handles).await;
    
    // Verify all succeeded
    for result in results {
        assert!(result.is_ok());
    }
}
```

## Fuzz Testing

```rust
use arbitrary::{Arbitrary, Unstructured};

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    messages: Vec<String>,
    model: String,
    temperature: f32,
}

#[test]
fn fuzz_message_parsing() {
    let mut data = [0u8; 1024];
    
    for _ in 0..10000 {
        rand::thread_rng().fill(&mut data);
        let u = Unstructured::new(&data);
        
        if let Ok(input) = FuzzInput::arbitrary(&mut u) {
            // Should not panic
            let _ = parse_messages(&input.messages);
        }
    }
}
```

## Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_token_count_properties(text in ".*") {
        let count = count_tokens(&text, "gpt-4");
        
        // Properties that must hold
        prop_assert!(count >= 0);
        prop_assert!(count <= text.len() * 2); // Max 2 tokens per char
        
        if text.is_empty() {
            prop_assert_eq!(count, 0);
        }
    }
    
    #[test]
    fn test_sliding_window_properties(
        messages in prop::collection::vec(any::<String>(), 0..100),
        limit in 100..10000usize
    ) {
        let window = SlidingWindow::new(limit);
        let result = window.prepare_context(messages);
        
        // Must not exceed limit
        let total_tokens = count_all_tokens(&result);
        prop_assert!(total_tokens <= limit);
    }
}
```

## Regression Testing

```rust
#[test]
fn test_no_behavior_regression() {
    // Golden files from TypeScript
    let golden_tests = load_golden_tests();
    
    for test in golden_tests {
        let rust_output = run_test(&test.input);
        assert_eq!(
            rust_output, test.expected_output,
            "Regression in test: {}", test.name
        );
    }
}
```

## Test Fixtures Management

```rust
pub struct TestFixtures {
    typescript_outputs: HashMap<String, Value>,
    test_cases: HashMap<String, Vec<TestCase>>,
}

impl TestFixtures {
    pub fn load() -> Self {
        // Load all TypeScript test outputs
        let mut typescript_outputs = HashMap::new();
        
        for entry in fs::read_dir("fixtures/typescript_outputs").unwrap() {
            let path = entry.unwrap().path();
            let content = fs::read_to_string(&path).unwrap();
            let name = path.file_stem().unwrap().to_str().unwrap();
            
            typescript_outputs.insert(
                name.to_string(),
                serde_json::from_str(&content).unwrap()
            );
        }
        
        Self {
            typescript_outputs,
            test_cases: load_test_cases(),
        }
    }
}
```

## CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Run Unit Tests
        run: cargo test --all
        
      - name: Run Integration Tests
        run: cargo test --test integration
        
      - name: Run Benchmarks
        run: cargo bench
        
      - name: Check Memory Usage
        run: cargo test --test memory_test
        
      - name: Compare with TypeScript
        run: |
          npm test > typescript_output.txt
          cargo test --test comparison > rust_output.txt
          diff typescript_output.txt rust_output.txt
```

## Implementation Checklist
- [ ] Create test fixtures from TypeScript
- [ ] Unit tests for every function
- [ ] Integration tests for all tools
- [ ] Performance benchmarks
- [ ] Memory usage tests
- [ ] Load testing framework
- [ ] Fuzz testing setup
- [ ] CI/CD pipeline
