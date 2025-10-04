# Codex 1:1 Rust Port — Ultra-Comprehensive Translation Plan

This plan covers a complete, 1:1 translation of the Codex monorepo to Rust with system resource usage optimization, while preserving 100% feature parity and identical UI/UX. No mocks. Production-grade only. Each step has acceptance gates and must pass before proceeding.

## Non‑negotiable Requirements

- 1:1 behavior parity. Do not mutate semantics, outputs, or user-visible UX.
- Preserve tooling contracts, prompt text, and APIs verbatim.
- No mocks. Real integrations and production-grade tests only.
- Optimize for memory, latency, throughput. No regressions.
- Every step is gated by tests; do not advance without green checks.

## Artifacts to Generate

- docs/CODEX-MANIFEST.txt — full, de-duplicated list of Codex files (relative paths).
- docs/CODEX-STATS-EXTENSIONS.csv — counts per extension.
- docs/CODEX-STATS-TOPLEVEL.csv — counts per top-level directory.
- docs/TRANSLATION-TRACKER.csv — row per file: status tracking (pending → in_progress → ported → verified).
- docs/CODEX-PORT-ACCEPTANCE-TESTS.md — acceptance test matrix and criteria.

## Phases & Acceptance Gates

1) Inventory & Scope Freeze
- Generate manifest and stats.
- Tag out-of-scope paths (if any). Default: everything in scope.
- Gate: Manifest commited. Stats generated.

2) Core Runtime (Rust)
- Port core logic in `src/` including prompts, tools, providers adapters, transforms, cost, utils.
- Gate: Unit tests + golden tests for prompts & tools identical.

3) Providers Layer (Rust)
- Port `src/api/providers/*` with streaming, usage, caching parity.
- Gate: Live integration tests per provider (keys required), cost accounting parity.

4) Tools & Operations (Rust)
- Port all edit/search/fs/git/diff tools; enforce exact schemas and error messages.
- Gate: E2E file-edit scenarios pass with golden diffs.

5) Indexing, Search, and Tree-sitter Bindings (Rust)
- Port code index, searchers, definitions, syntax and query infrastructure.
- Gate: Deterministic search/index tests, performance targets met.

6) Frontend/UI (Rust target TBD)
- Port `webview-ui/` and app UIs in `apps/` to the chosen Rust/WASM or native GUI stack.
- Gate: Visual regression + interaction parity.

7) Packaging & Distribution
- Replace Node build pipeline with Rust workspace, binaries, and (if applicable) WASM bundling.
- Gate: Reproducible builds, signed artifacts, CI passing.

8) Full E2E Acceptance
- Run the complete acceptance matrix.
- Gate: All green, performance thresholds met or improved.

## Directory-by-Directory Plan (Top Level)

- src/
  - __Goal__: Port core logic: prompts, tools, responses, commands, API transforms, providers glue.
  - __Key paths__:
    - `src/core/prompts/**` (system assembly, sections, tools, responses, commands)
    - `src/api/providers/**` (anthropic, openai, gemini, bedrock, openrouter, vscode-lm, ollama)
    - `src/api/transform/**` (message conversion, streams, caching)
    - `src/shared/**` (types, config, cost, etc.)
  - __Gate__: Golden tests for prompts + tool descriptions + streaming usage parity.

- packages/
  - __Goal__: Port all shared libraries consumed by apps and core.
  - __Gate__: Unit tests and cross-crate integration tests.

- apps/
  - __Goal__: Port all web apps preserving UX and behavior.
  - __Gate__: Route parity, API parity, visual regression tests.

- webview-ui/
  - __Goal__: Port VSCode webview-based UI to Rust target (WASM/native) with pixel/behavior parity.
  - __Gate__: Interactive UI tests and golden screenshots.

- scripts/
  - __Goal__: Replace Node scripts with Rust CLI tooling.
  - __Gate__: Same flags, same outputs, CI parity.

- benchmark/, launch/
  - __Goal__: Port benchmarking and launch tooling to Rust.
  - __Gate__: Measured performance within targets.

- .github/, configs, workspace files
  - __Goal__: Adapt CI, linting, release to Rust toolchain.
  - __Gate__: End-to-end CI success.

## Translation Tactics

- Prompts & Tools: Copy verbatim strings, XML schemas, examples, and rules. Build prompt-assembly identical ordering and conditional sections.
- Providers: Mirror request/response bodies, headers, streaming parsing, usage tokens, cache markers, errors.
- Tools: Preserve XML parameter names, error texts, and incremental edits (apply_diff/morph gating) exactly.
- Performance: Replace JS/TS runtimes with zero-copy Rust pathways, bounded allocations, and tight streaming loops.
- Testing: Golden tests (bit-for-bit for textual outputs) + live provider tests (real keys).

## Tracking

- Use docs/TRANSLATION-TRACKER.csv to track each file.
- Status flow: pending → in_progress → ported → verified.
- No advancement without tests.

## Open Decisions (require confirmation)

- Rust UI target for `webview-ui/` and `apps/`: Yew/Leptos/Dioxus+Trunk (WASM), or Tauri (native window), or Slint (native)?
- Packaging targets: CLI only, plugin, desktop app, or all of the above?

## Immediate Next Actions

- Generate manifest and stats (commands prepared separately).
- Fill `TRANSLATION-TRACKER.csv` with one row per manifest path.
- Start with `src/` providers + prompts (greatest coupling and risk).
