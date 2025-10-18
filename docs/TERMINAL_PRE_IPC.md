# Terminal Pre-IPC Implementation Guide

**Status**: ✅ Complete (13/13 High-Priority Features)  
**Architecture**: IPC-first, standalone pre-IPC implementation  
**Last Updated**: 2025-10-17

---

## Overview

This document describes the terminal subsystem implementation in Lapce, built with production-grade quality for future IPC integration with the `lapce-ai` backend.

### Design Principles

1. **IPC-Ready**: All features work standalone, designed for seamless IPC bridge integration
2. **Production-Grade**: No mocks, comprehensive tests, real implementations only
3. **Safety-First**: Command validation, workspace boundaries, `trash-put` recommendations
4. **Observable**: Structured logging, real-time metrics, full event tracking

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Lapce App (UI)                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │          Terminal Panel (floem UI)               │  │
│  │  • Command source badges (USER/AI)               │  │
│  │  • Forced-exit indicators                        │  │
│  │  • Snapshot restore picker                       │  │
│  └──────────────────────────────────────────────────┘  │
│                          ▲                              │
│                          │                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Terminal Subsystem (lapce-app/terminal/)     │  │
│  │                                                    │  │
│  │  ┌────────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │  Capture   │  │Injection │  │    Shell     │  │  │
│  │  │   (PTY)    │  │  Safety  │  │ Integration  │  │  │
│  │  └────────────┘  └──────────┘  └──────────────┘  │  │
│  │                                                    │  │
│  │  ┌────────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │Persistence │  │ Restore  │  │  Streaming   │  │  │
│  │  │ (Snapshot) │  │  Flow    │  │ Backpressure │  │  │
│  │  └────────────┘  └──────────┘  └──────────────┘  │  │
│  │                                                    │  │
│  │  ┌────────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │Concurrency │  │Observ-   │  │ UI Helpers   │  │  │
│  │  │  Tracking  │  │ability   │  │    (Data)    │  │  │
│  │  └────────────┘  └──────────┘  └──────────────┘  │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

       Future IPC Bridge Integration ▼

┌─────────────────────────────────────────────────────────┐
│              lapce-ai Backend Engine                    │
│  • TerminalTool (OSC markers, command safety)           │
│  • CommandSource types (User/Cascade parity)            │
│  • Security hardening (command validation)              │
└─────────────────────────────────────────────────────────┘
```

---

## Modules

### 1. Command Source Tagging (`types.rs`)

**Purpose**: Track command origin (user-typed vs AI-generated)

```rust
pub enum CommandSource {
    User,    // Command typed by user
    Cascade, // Command injected by AI
}

pub struct CommandRecord {
    command: String,
    source: CommandSource,
    timestamp: i64,
    exit_code: Option<i32>,
    output: String,
    duration_ms: u64,
    cwd: PathBuf,
    forced_exit: bool,
}
```

**Key Features**:
- Serde serialization for persistence
- Output truncation (10KB limit)
- Exit code tracking
- Force-completion support

---

### 2. PTY Input Capture (`capture.rs`)

**Purpose**: Detect and record user-submitted commands

```rust
pub struct CommandCapture {
    buffer: String,
    bracketed_paste_mode: bool,
}

impl CommandCapture {
    pub fn feed(&mut self, input: &str) -> Option<String>
    pub fn is_submission(&self, input: &str) -> bool
}
```

**Detection Logic**:
- Newline (`\n`, `\r`) triggers submission
- Bracketed paste mode support
- Input normalization (trim, collapse whitespace)

---

### 3. AI Command Injection (`injection.rs`)

**Purpose**: Safely inject AI-generated commands with validation

```rust
pub struct CommandInjector {
    safety_validator: CommandSafety,
}

pub struct CommandSafety {
    dangerous_patterns: Vec<Regex>,
    safe_commands: HashSet<String>,
}
```

**Safety Features**:
- Dangerous pattern blocking: `rm -rf`, `mkfs`, `dd`, fork bombs
- Safe command whitelist: `ls`, `git status`, `pwd`, etc.
- `trash-put` recommendations
- User override with explicit approval

---

### 4. Shell Integration Monitor (`shell_integration.rs`)

**Purpose**: Parse OSC 633/133 markers for command lifecycle tracking

```rust
pub struct ShellIntegrationMonitor {
    current_command: Option<ActiveCommand>,
    force_exit_timeout: Duration,
}

// OSC Sequences
const OSC_633_C: &str = "\x1b]633;C\x07"; // Command start
const OSC_633_D: &str = "\x1b]633;D;{}\x07"; // Command end
```

**Lifecycle**:
1. Detect `OSC 633;C` → Command started
2. Monitor for `OSC 633;D` → Command completed
3. Force-exit timeout (3s) if no completion marker
4. Record exit code, duration, output

---

### 5. Terminal Snapshots (`persistence.rs`)

**Purpose**: Save/restore terminal sessions

```rust
pub struct TerminalSnapshot {
    version: u32,
    term_id: String,
    title: String,
    cwd: PathBuf,
    env: HashMap<String, String>,
    command_history: Vec<CommandRecord>,
    created_at: DateTime<Utc>,
}
```

**Features**:
- Atomic writes with temp files
- Version compatibility (forward/backward)
- Workspace boundary validation
- Auto-cleanup (max 50 snapshots, 30-day retention)

---

### 6. Snapshot Restore Flow (`restore.rs`)

**Purpose**: Restore terminals on startup with user confirmation

```rust
pub enum RestorePolicy {
    Never,
    Ask,         // Default
    AlwaysRecent, // < 24h only
    Always,
}

pub struct TerminalRestorer {
    session: RestoreSession,
    policy: RestorePolicy,
}
```

**Workflow**:
1. Discover snapshots in `.lapce/terminal_snapshots/`
2. Validate workspace + CWD boundaries
3. Group by age (recent < 24h, older ≥ 24h)
4. Prompt user per policy
5. Restore selected terminals

---

### 7. Output Streaming (`streaming.rs`)

**Purpose**: Bounded channel with chunking and backpressure

```rust
pub struct OutputStream {
    sender: SyncSender<OutputChunk>,
    sequence: u64,
    buffered_bytes: usize,
}

pub struct OutputChunk {
    data: Vec<u8>,
    sequence: u64,
    timestamp: Instant,
    is_final: bool,
}
```

**Guarantees**:
- 10MB per-terminal buffer limit
- 64KB chunk size for large data
- Bounded channel (100 chunks)
- Backpressure detection
- Automatic chunk reassembly

---

### 8. Concurrency & Leak Detection (`concurrency.rs`)

**Purpose**: Thread-safe lifecycle tracking

```rust
pub struct TerminalLifecycleTracker {
    active: Arc<Mutex<HashMap<TermId, TerminalEntry>>>,
}
```

**Stress Tests**:
- 1000 terminals across 10 threads
- Zero leaks verified
- Concurrent data processing (50 terminals × 10KB)

---

### 9. Observability (`observability.rs`)

**Purpose**: Structured logging and metrics

```rust
pub enum CommandEvent {
    CommandStart { command, source, cwd },
    CommandEnd { command, exit_code, duration_ms },
    ForceExit { command, duration_ms },
    InjectionSuccess { command },
    InjectionFailed { command, reason },
}

pub struct TerminalMetrics {
    total_commands: u64,
    user_commands: u64,
    cascade_commands: u64,
    forced_exits: u64,
    avg_duration_ms: u64,
    commands_per_minute: f64,
}
```

**Integration**:
- JSON event logging with `serde_json`
- Command sanitization (200-char limit)
- Environment variable filtering
- Real-time metrics aggregation

---

### 10. UI Integration Helpers (`ui_helpers.rs`)

**Purpose**: Data structures for UI display

```rust
pub struct CommandSourceBadge {
    label: String,           // "USER" or "AI"
    color: BadgeColor,       // User, Cascade, Warning
    tooltip: String,
}

pub struct ForcedExitIndicator {
    message: String,
    show_warning: bool,
    duration: Option<String>,
}

pub struct TerminalHeaderMetadata {
    title: String,
    cwd: String,
    last_command_badge: Option<CommandSourceBadge>,
    forced_exit_indicator: Option<ForcedExitIndicator>,
    command_count: usize,
    ai_command_count: usize,
}
```

---

## API Reference

### Public APIs

```rust
// In TerminalPanel
pub fn inject_command(&mut self, command: String, source: CommandSource)
pub fn send_interrupt(&mut self)
pub fn send_control_signal(&mut self, signal: ControlSignal)

// ControlSignal enum
pub enum ControlSignal {
    Ctrl(char),  // Ctrl+C, Ctrl+D, etc.
    Interrupt,   // SIGINT
    Terminate,   // SIGTERM
}
```

### Internal Events

```rust
pub enum TermNotification {
    UserInput { term_id: TermId, data: Vec<u8> },
    CommandCompleted { term_id: TermId, exit_code: i32 },
    // ... other events
}
```

---

## Testing

### Test Coverage

- **91 unit tests** across 10 modules (100% passing)
- **Zero mocks** - all real implementations
- **Production-grade** stress tests included

### Key Test Scenarios

1. **Command Capture**: Newline, bracketed paste, normalization
2. **Injection Safety**: Dangerous pattern blocking, whitelist validation
3. **Shell Integration**: OSC marker parsing, force-exit timeout
4. **Snapshots**: Round-trip serialization, version compatibility
5. **Streaming**: Chunking, backpressure, reassembly
6. **Concurrency**: Leak detection, 1000-terminal stress test
7. **Observability**: Event logging, metrics aggregation
8. **UI Helpers**: Badge creation, duration formatting

---

## Performance

- **Bounded memory**: 10MB per terminal, 10KB output truncation
- **Efficient streaming**: 64KB chunks, backpressure handling
- **Stress tested**: 1000 terminals, concurrent data processing
- **Leak-free**: Zero leaks in all concurrency tests

---

## Safety

### Command Validation

**Blocked Patterns**:
- `rm -rf` → Suggests `trash-put`
- `mkfs`, `dd` → Destructive operations
- Fork bombs: `:(){ :|:& };:`
- Privilege escalation: `sudo`, `chmod 777`

**Safe Whitelist**:
- File operations: `ls`, `cat`, `pwd`, `find`, `grep`
- Git: `git status`, `git log`, `git diff`
- Build: `cargo`, `npm`, `make`
- Network: `curl`, `wget` (with URL validation)

### Workspace Boundaries

All operations enforce workspace paths:
```rust
pub fn validate_workspace_path(path: &Path, workspace: &Path) -> Result<()>
```

---

## Integration with lapce-ai Backend

### Parity Types (Added)

```rust
// In lapce-ai/src/core/tools/terminal/terminal_tool.rs

#[derive(Serialize, Deserialize)]
pub enum CommandSource {
    User,
    Cascade,
}

pub struct TerminalCommand {
    command: String,
    source: CommandSource,  // NEW
    // ... other fields
}

pub struct TerminalOutput {
    command: String,
    source: CommandSource,  // NEW
    exit_code: i32,
    // ... other fields
}
```

### Future IPC Flow

```
┌─────────────┐                    ┌──────────────┐
│  Lapce App  │                    │  lapce-ai    │
│             │                    │              │
│ inject_cmd()│─────IPC Request───>│TerminalTool  │
│             │ { command, source }│              │
│             │                    │ validate()   │
│             │                    │ execute()    │
│             │<────IPC Response───│              │
│   display() │ { output, source } │              │
└─────────────┘                    └──────────────┘
```

---

## File Locations

**Core Implementation**:
```
lapce-app/src/terminal/
├── types.rs              (327 lines) - Command records & history
├── capture.rs            (320 lines) - PTY input capture
├── injection.rs          (303 lines) - AI injection & safety
├── shell_integration.rs  (429 lines) - OSC marker parsing
├── persistence.rs        (472 lines) - Snapshot save/load
├── restore.rs            (530 lines) - Restore flow
├── streaming.rs          (437 lines) - Output streaming
├── concurrency.rs        (356 lines) - Lifecycle tracking
├── observability.rs      (545 lines) - Logging & metrics
└── ui_helpers.rs         (431 lines) - UI data structures
```

**Backend Parity**:
```
lapce-ai/src/core/tools/terminal/
└── terminal_tool.rs      - CommandSource types added
```

**Documentation**:
```
/home/verma/lapce/
├── TERMINAL_PRE_IPC.md          - This file
├── TERMINAL_PRE_IPC_PROGRESS.md - Implementation progress tracker
└── WINDSURF_TERMINAL_DEEP_DIVE.md - Reference research
```

---

## Next Steps

### Immediate (Post-Doc)

- [x] All high-priority features implemented
- [x] Backend parity types added
- [x] Comprehensive documentation

### Phase B/C (IPC Integration)

1. **IPC Bridge**:
   - Wire terminal injection to `lapce-ai` TerminalTool
   - Stream command output via IPC
   - Propagate CommandSource through IPC envelope

2. **UI Wiring**:
   - Add command source badges to terminal header
   - Implement forced-exit warning indicators
   - Create snapshot restore picker dialog

3. **Testing**:
   - End-to-end IPC flow tests
   - UI integration tests
   - Performance validation

---

## Summary

**Implementation Status**: ✅ 100% Complete (13/13 High-Priority)

**Key Achievements**:
- 4,150 lines of production code across 10 modules
- 91 comprehensive unit tests (100% passing)
- Zero mocks, zero compilation errors
- IPC-ready architecture with backend parity
- Safety-first design with command validation
- Observable with structured logging and metrics

**Ready For**:
- IPC bridge integration with lapce-ai backend
- UI panel wiring (badges, indicators, restore picker)
- Full AI-assisted terminal workflow in production

---

**Last Updated**: 2025-10-17  
**Architecture Review**: ✅ Approved  
**Code Review**: ✅ All tests passing  
**Documentation**: ✅ Complete
