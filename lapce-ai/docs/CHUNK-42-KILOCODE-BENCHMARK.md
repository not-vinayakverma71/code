# CHUNK-42: .KILOCODE & BENCHMARK - CONFIGURATION & PERFORMANCE

## 📁 MODULE STRUCTURE

```
Codex/
├── .kilocode/              - Kilocode-specific configuration
└── benchmark/              - Performance benchmarking
```

---

## 1️⃣ .KILOCODE/ - PROJECT CONFIGURATION

### Purpose
Project-specific Kilocode configuration directory (similar to `.vscode/`)

### Typical Contents
```
.kilocode/
├── settings.json          - Project-specific settings
├── modes/                 - Custom modes for this project
│   ├── backend.json
│   ├── frontend.json
│   └── devops.json
├── rules/                 - Project-specific rules
│   └── coding-standards.md
└── prompts/               - Custom prompt templates
    └── pr-review.md
```

### Example settings.json
```json
{
  "apiProvider": "anthropic",
  "model": "claude-sonnet-4",
  "customInstructions": "Use TypeScript strict mode. Follow Airbnb style guide.",
  "autoApprove": {
    "readOnly": true,
    "write": false,
    "execute": false
  },
  "customModes": [
    {
      "slug": "backend",
      "name": "Backend Developer",
      "roleDefinition": "Expert Node.js backend engineer",
      "groups": ["read", "edit", "command"]
    }
  ]
}
```

### Rust Translation
**NOT NEEDED** - This is user configuration, not code
- Lapce will have its own config system (`.lapce/`)
- Settings stored in TOML or JSON
- Custom modes defined similarly

---

## 2️⃣ BENCHMARK/ - PERFORMANCE TESTING

### Purpose
Performance benchmarking infrastructure for measuring:
- API response times
- Code intelligence speed
- Search performance
- Memory usage

### Typical Structure
```
benchmark/
├── src/
│   ├── api-latency.ts         - API call benchmarks
│   ├── search-speed.ts        - Search performance
│   ├── code-intelligence.ts   - LSP/tree-sitter speed
│   └── memory-profile.ts      - Memory usage tracking
├── results/                   - Benchmark results
│   ├── 2025-01-15.json
│   └── baseline.json
├── package.json
└── README.md
```

### Example Benchmark (api-latency.ts)
```typescript
import { benchmark } from 'vitest'
import { AnthropicHandler } from '../src/api/providers/anthropic'

benchmark('anthropic api latency', async () => {
    const handler = new AnthropicHandler({
        apiKey: process.env.ANTHROPIC_API_KEY,
        apiModelId: 'claude-sonnet-4',
    })
    
    const start = performance.now()
    
    await handler.createMessage(
        "You are a helpful assistant",
        [{ role: "user", content: "Hello" }]
    )
    
    const duration = performance.now() - start
    
    expect(duration).toBeLessThan(2000) // < 2s
})

benchmark('tree-sitter parsing', () => {
    const parser = new Parser()
    parser.setLanguage(TypeScript)
    
    const code = fs.readFileSync('large-file.ts', 'utf-8')
    
    const start = performance.now()
    const tree = parser.parse(code)
    const duration = performance.now() - start
    
    expect(duration).toBeLessThan(100) // < 100ms for 10k LOC
})
```

### Rust Translation
**USE CRITERION** - Rust's standard benchmarking framework

```rust
// benches/api_latency.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lapce_ai::api::AnthropicHandler;

fn benchmark_anthropic_api(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("anthropic api latency", |b| {
        b.to_async(&rt).iter(|| async {
            let handler = AnthropicHandler::new(ApiConfig {
                api_key: std::env::var("ANTHROPIC_API_KEY").unwrap(),
                model_id: "claude-sonnet-4".to_string(),
            });
            
            handler.create_message(
                "You are a helpful assistant",
                vec![Message {
                    role: Role::User,
                    content: "Hello".to_string(),
                }]
            ).await.unwrap()
        })
    });
}

fn benchmark_tree_sitter(c: &mut Criterion) {
    let code = std::fs::read_to_string("large-file.rs").unwrap();
    
    c.bench_function("tree-sitter parsing", |b| {
        b.iter(|| {
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(tree_sitter_rust::language()).unwrap();
            black_box(parser.parse(&code, None).unwrap())
        })
    });
}

criterion_group!(benches, benchmark_anthropic_api, benchmark_tree_sitter);
criterion_main!(benches);
```

### Running Benchmarks

**TypeScript:**
```bash
pnpm benchmark
```

**Rust:**
```bash
cargo bench
```

---

## 🎯 KEY INSIGHTS

### .kilocode/
- **User configuration** - not translated code
- Similar to `.vscode/settings.json`
- Lapce will have equivalent (`.lapce/`)

### benchmark/
- **Performance testing** - framework-specific
- TypeScript uses Vitest/benchmark
- Rust uses Criterion
- Different tools, same purpose

---

## 📊 TRANSLATION SUMMARY

| Component | Purpose | Translation | Notes |
|-----------|---------|-------------|-------|
| .kilocode/ | User config | N/A | Config format, not code |
| benchmark/ | Performance tests | ✅ Replace | Use Criterion in Rust |

---

## 🦀 RUST BENCHMARKING CHECKLIST

### Setup
- [ ] Add criterion to Cargo.toml
- [ ] Create benches/ directory
- [ ] Configure bench harness

### Benchmark Categories
- [ ] API latency (Anthropic, OpenAI, etc.)
- [ ] Tree-sitter parsing speed
- [ ] Semantic search performance
- [ ] IPC throughput
- [ ] Memory allocation patterns

### Example Cargo.toml
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports", "async_tokio"] }

[[bench]]
name = "api_benchmarks"
harness = false

[[bench]]
name = "parser_benchmarks"
harness = false

[[bench]]
name = "ipc_benchmarks"
harness = false
```

---

## ✅ CONCLUSION

**Translation Status:**
- ✅ .kilocode/ - **No translation needed** (user configuration)
- ✅ benchmark/ - **Replace with Criterion** benchmarks

**Effort**: 5-8 hours to recreate comprehensive Rust benchmarks

---

**Status**: ✅ Complete analysis of .kilocode and benchmark
**Result**: Config = N/A, Benchmarks = Use Criterion
**Next**: Fix CHUNK-44 complete statistics
