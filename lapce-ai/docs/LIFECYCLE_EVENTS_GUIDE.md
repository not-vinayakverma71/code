# Tool Lifecycle Events - Developer Guide

## Overview

The tool system now supports comprehensive lifecycle event tracking through IPC messages. This enables real-time monitoring, debugging, and UI updates for tool executions.

## Event Types

### 1. ToolExecutionStatus

Tracks high-level tool execution state:

```rust
pub enum ToolExecutionStatus {
    Started {
        tool_name: String,
        correlation_id: String,
        timestamp: u64,
    },
    Progress {
        correlation_id: String,
        message: String,
        percentage: Option<u8>,
    },
    Completed {
        correlation_id: String,
        result: serde_json::Value,
        duration_ms: u64,
    },
    Failed {
        correlation_id: String,
        error: String,
        duration_ms: u64,
    },
}
```

**Usage Example**:
```rust
// Tool starts
ToolExecutionStatus::Started {
    tool_name: "readFile".to_string(),
    correlation_id: "uuid-123".to_string(),
    timestamp: 1696800000000,
}

// Tool reports progress
ToolExecutionStatus::Progress {
    correlation_id: "uuid-123".to_string(),
    message: "Reading large file...".to_string(),
    percentage: Some(50),
}

// Tool completes
ToolExecutionStatus::Completed {
    correlation_id: "uuid-123".to_string(),
    result: json!({"content": "file contents"}),
    duration_ms: 1500,
}
```

### 2. CommandExecutionStatus

Tracks command execution with streaming output:

```rust
pub enum CommandExecutionStatus {
    Started {
        command: String,
        args: Vec<String>,
        correlation_id: String,
    },
    OutputChunk {
        correlation_id: String,
        chunk: String,
        stream_type: StreamType,
    },
    Exit {
        correlation_id: String,
        exit_code: i32,
        duration_ms: u64,
    },
}

pub enum StreamType {
    Stdout,
    Stderr,
}
```

**Usage Example**:
```rust
// Command starts
CommandExecutionStatus::Started {
    command: "ls".to_string(),
    args: vec!["-la".to_string()],
    correlation_id: "cmd-456".to_string(),
}

// Receive stdout
CommandExecutionStatus::OutputChunk {
    correlation_id: "cmd-456".to_string(),
    chunk: "total 24\n".to_string(),
    stream_type: StreamType::Stdout,
}

// Command exits
CommandExecutionStatus::Exit {
    correlation_id: "cmd-456".to_string(),
    exit_code: 0,
    duration_ms: 100,
}
```

### 3. DiffOperation

Tracks diff view operations:

```rust
pub enum DiffOperation {
    OpenDiffFiles {
        left_path: String,
        right_path: String,
        correlation_id: String,
    },
    SaveDiff {
        correlation_id: String,
        target_path: String,
    },
    RevertDiff {
        correlation_id: String,
    },
    CloseDiff {
        correlation_id: String,
    },
}
```

**Usage Example**:
```rust
// Open diff view
DiffOperation::OpenDiffFiles {
    left_path: "/tmp/original.txt".to_string(),
    right_path: "/tmp/modified.txt".to_string(),
    correlation_id: "diff-789".to_string(),
}

// User saves changes
DiffOperation::SaveDiff {
    correlation_id: "diff-789".to_string(),
    target_path: "/workspace/file.txt".to_string(),
}
```

### 4. ApprovalMessage

Handles approval flow:

```rust
pub enum ApprovalMessage {
    ApprovalRequested {
        tool_name: String,
        operation: String,
        details: serde_json::Value,
        correlation_id: String,
        timeout_ms: Option<u64>,
    },
    ApprovalDecision {
        correlation_id: String,
        approved: bool,
        reason: Option<String>,
    },
}
```

**Usage Example**:
```rust
// Request approval
ApprovalMessage::ApprovalRequested {
    tool_name: "writeFile".to_string(),
    operation: "create".to_string(),
    details: json!({
        "path": "/workspace/important.txt",
        "size": 1024
    }),
    correlation_id: "appr-111".to_string(),
    timeout_ms: Some(30000),
}

// User approves
ApprovalMessage::ApprovalDecision {
    correlation_id: "appr-111".to_string(),
    approved: true,
    reason: Some("User confirmed".to_string()),
}
```

### 5. InternalCommand

Triggers Lapce IDE actions:

```rust
pub enum InternalCommand {
    OpenDiffFiles {
        left_path: String,
        right_path: String,
    },
    ExecuteProcess {
        program: String,
        arguments: Vec<String>,
    },
}
```

**Usage Example**:
```rust
// Open diff in Lapce
InternalCommand::OpenDiffFiles {
    left_path: "/workspace/original.txt".to_string(),
    right_path: "/workspace/modified.txt".to_string(),
}

// Run command in Lapce terminal
InternalCommand::ExecuteProcess {
    program: "cargo".to_string(),
    arguments: vec!["test".to_string()],
}
```

## Correlation IDs

All events use UUID-based correlation IDs for tracking:

```rust
use uuid::Uuid;

let correlation_id = Uuid::new_v4().to_string();
```

**Best Practices**:
- Generate once at operation start
- Pass through all related events
- Use for debugging and log correlation
- Store in UI for event filtering

## Adapter Integration

Events are emitted through adapters:

```rust
// Get adapter from context
if let Some(adapter) = context.get_adapter("ipc") {
    // Emit event (when adapters are fully wired)
    adapter.emit_event(CommandExecutionStatus::Started {
        command: cmd.to_string(),
        args: vec![],
        correlation_id: id.clone(),
    }).await;
}
```

**Current Status**: Adapter infrastructure in place, full wiring pending.

## Testing

All message types include roundtrip serialization tests:

```bash
# Run lifecycle event tests
cargo test --lib ipc::ipc_messages::tests

# Test specific event type
cargo test test_command_execution_status_roundtrip
```

## Security Considerations

1. **Sensitive Data**: Never include passwords or API keys in events
2. **Path Sanitization**: Always validate paths before emitting
3. **Rate Limiting**: Consider rate limiting progress events
4. **Correlation ID Privacy**: UUIDs are safe but don't expose sequential IDs

## Performance Notes

- Events are async and non-blocking
- OutputChunk events can be high-frequency (per-line)
- Consider batching small chunks for performance
- Correlation IDs have minimal overhead (~36 bytes)

## Example: Complete Flow

```rust
use uuid::Uuid;

async fn execute_with_events(
    command: &str,
    context: &ToolContext
) -> Result<()> {
    let correlation_id = Uuid::new_v4().to_string();
    let start_time = Instant::now();
    
    // Emit Started
    emit_event(CommandExecutionStatus::Started {
        command: command.to_string(),
        args: vec![],
        correlation_id: correlation_id.clone(),
    });
    
    // Execute command
    let output = execute_command(command).await?;
    
    // Emit output chunks
    for line in output.lines() {
        emit_event(CommandExecutionStatus::OutputChunk {
            correlation_id: correlation_id.clone(),
            chunk: line.to_string(),
            stream_type: StreamType::Stdout,
        });
    }
    
    // Emit Exit
    let duration_ms = start_time.elapsed().as_millis() as u64;
    emit_event(CommandExecutionStatus::Exit {
        correlation_id,
        exit_code: 0,
        duration_ms,
    });
    
    Ok(())
}
```

## Troubleshooting

### Events Not Appearing
- Check adapter is registered: `context.get_adapter("ipc")`
- Verify correlation ID consistency
- Check event serialization with roundtrip tests

### High Event Volume
- Batch small OutputChunk events
- Consider debouncing Progress events
- Use sampling for high-frequency updates

### Missing Events
- Ensure Exit event sent on all code paths (including errors)
- Use `defer` pattern for cleanup events
- Add event emission to error handlers

## Future Enhancements

- [ ] Event compression for large payloads
- [ ] Event replay for debugging
- [ ] Metric aggregation from events
- [ ] Event filtering and routing
- [ ] WebSocket streaming to UI
