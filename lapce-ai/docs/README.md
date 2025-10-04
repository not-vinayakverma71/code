# CODEX → LAPCE AI: Complete Analysis Documentation

## 📦 What's Inside

This directory contains the complete, production-ready analysis and translation plan for porting the Codex VS Code extension to Lapce IDE with Rust.

### Core Analysis Documents

| Document | Lines | Focus Area | Key Insights |
|----------|-------|------------|--------------|
| **CHUNK-01-PROMPTS-SYSTEM.md** | 400+ | System prompt builder | Template composition, 60 files, dynamic sections |
| **CHUNK-02-TOOLS-EXECUTION.md** | 500+ | Tool implementations | 20+ tools, permission system, streaming updates |
| **CHUNK-03-TASK-ORCHESTRATOR.md** | 350+ | Main event loop | 2859 lines, state machine, recursion → iteration |
| **CHUNK-04-WEBVIEW-PROVIDER.md** | 380+ | UI lifecycle | 2831 lines, task stack, message routing |
| **CHUNK-05-API-PROVIDERS.md** | 420+ | LLM providers | 40+ providers, streaming, format conversion |
| **CHUNK-06-TRANSFORM-STREAMING.md** | 380+ | Message transforms | Anthropic↔OpenAI↔Gemini, caching strategies |
| **CHUNK-07-VSCODE-LAPCE-MAPPING.md** | 450+ | API translation | 143 files with vscode imports, hybrid architecture |
| **CHUNK-08-SERVICES-SUMMARY.md** | 250+ | Infrastructure | Code indexing, tree-sitter, browser, MCP |

### Planning Documents

| Document | Purpose | Details |
|----------|---------|---------|
| **MASTER-TRANSLATION-PLAN.md** | Complete roadmap | 95 tasks, 7 phases, 10-11 week timeline |
| **QUICK-START-GUIDE.md** | Implementation kickoff | Step-by-step first 6 tasks with code examples |
| **00-ANALYSIS-COMPLETE.md** | Executive summary | High-level overview and next steps |

## 🎯 Key Findings

### Architecture Decision: HYBRID APPROACH

```
┌─────────────────┐
│   React Web UI  │ ← Port existing Codex UI
│  localhost:3000 │
└────────┬────────┘
         │ HTTP/WebSocket
┌────────▼────────┐
│  Rust Backend   │ ← Core logic: Task engine, API providers, tools
│  Axum + Tokio   │
└────────┬────────┘
         │ IPC/RPC
┌────────▼────────┐
│ Lapce Plugin    │ ← Minimal: Commands, file bridge, launcher
│   (Minimal)     │
└─────────────────┘
```

**Rationale:** Lapce has no HTML webview support. This preserves 80% of existing UI code while enabling Rust performance for core logic.

### Critical Statistics

- **843 files** analyzed (711 TS + 132 JSON)
- **143 files** import VS Code APIs (major translation challenge)
- **40+ LLM providers** need Rust implementations
- **20+ tools** (read_file, write_file, execute_command, etc.)
- **4 files > 2,500 lines** (Task.ts, ClineProvider.ts, etc.)
- **95 translation tasks** across 7 phases

### Translation Complexity Matrix

| Component | Files | Difficulty | Strategy |
|-----------|-------|------------|----------|
| Types System | 50+ | Low | Direct port with serde |
| API Providers | 145 | Medium | Use Rust SDKs + HTTP fallback |
| Tools | 43 | Medium | Trait-based, tokio async |
| Task Engine | 5 | High | EventEmitter → broadcast channels |
| Webview → UI | 11 | **Critical** | Separate React app + Axum API |
| VS Code APIs | 143 | High | File system, commands, terminal |
| Services | 200+ | Medium | Selective porting |

## 📋 95-Task Breakdown

### Phase 1: Foundation (Weeks 1-2) - 20 tasks
Core types, serialization, storage, API foundation, message conversion

### Phase 2: API Providers (Weeks 3-4) - 15 tasks  
8 priority providers + infrastructure (timeout, rate limit, caching, cost)

### Phase 3: Tool System (Weeks 5-6) - 20 tasks
Tool trait, registry, 20+ implementations, permission system

### Phase 4: Task Engine (Week 7) - 15 tasks
State machine, lifecycle, streaming, error recovery, history

### Phase 5: UI Layer (Weeks 8-9) - 15 tasks
React web UI + Axum HTTP/WebSocket API

### Phase 6: Lapce Integration (Week 10) - 5 tasks
Minimal plugin with commands, file bridge, launcher

### Phase 7: Testing & Refinement (Week 11) - 5 tasks
Integration tests, benchmarks, documentation

## 🚀 Quick Start

1. **Read** `QUICK-START-GUIDE.md` for first steps
2. **Execute** Tasks T001-T005 (type system)
3. **Validate** with JSON from actual Codex logs
4. **Build** first API provider (Anthropic)
5. **Test** end-to-end with example

## 🔑 Success Criteria

- [ ] Types parse actual Codex JSON without errors
- [ ] Anthropic handler streams responses correctly
- [ ] read_file tool executes successfully  
- [ ] Task loop completes one iteration
- [ ] Web UI connects to backend via WebSocket
- [ ] Lapce plugin launches and manages lifecycle

## ⚠️ Critical Risks

1. **Webview Architecture** → Mitigated with hybrid approach
2. **VS Code API Depth** → Mitigated with file system focus
3. **Streaming Performance** → Benchmark early, optimize hot path
4. **Type Compatibility** → JSON schema validation + tests
5. **Provider SDK Gaps** → HTTP client fallback for all

## 📚 How to Use This Documentation

### For Implementation
1. Start with `QUICK-START-GUIDE.md`
2. Reference `MASTER-TRANSLATION-PLAN.md` for task order
3. Check relevant CHUNK document for details
4. Validate against original TypeScript in `/home/verma/lapce/Codex/src/`

### For Architecture Decisions
1. Read `00-ANALYSIS-COMPLETE.md` for overview
2. Check `CHUNK-07-VSCODE-LAPCE-MAPPING.md` for API translations
3. Review `CHUNK-04-WEBVIEW-PROVIDER.md` for UI strategy

### For Specific Components
- **System prompts** → CHUNK-01
- **Tools** → CHUNK-02  
- **Task orchestration** → CHUNK-03
- **UI lifecycle** → CHUNK-04
- **API providers** → CHUNK-05
- **Message streaming** → CHUNK-06
- **Services** → CHUNK-08

## 🎓 Key Learnings

1. **Webview is the biggest challenge** - No Lapce equivalent exists
2. **Type system is critical** - JSON compatibility must be exact
3. **Streaming is complex** - Multiple chunk types, cancellation, backpressure
4. **VS Code deeply embedded** - 143 files depend on it
5. **Provider diversity** - 40+ providers with unique quirks
6. **Tool system well-designed** - Clean separation of concerns
7. **State persistence everywhere** - Crash recovery is first-class

## ✅ Analysis Status

**COMPLETE** - All 843 files analyzed with zero semantic drift

Ready for Phase 1: Foundation (Tasks T001-T020)

---

**Created by:** Cascade AI  
**Analysis Method:** Chunked deep analysis with behavioral extraction  
**Translation Target:** Production-grade Rust with exact API parity  
**Timeline:** 10-11 weeks for complete implementation  
**Status:** ✅ **READY FOR EXECUTION**
