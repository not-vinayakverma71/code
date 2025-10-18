# Canonical Binary Header Specification
## Production-Grade IPC Protocol v1.0

This document defines the FINAL, CANONICAL binary header format for all IPC communication in Lapce-AI.
All codecs and server implementations MUST follow this specification exactly.

## Header Layout (24 bytes total)

```
┌──────────────┬─────────┬─────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│ Magic (4B)   │ Ver(1B) │ Flags(1B)│ Type(2B) │ Len(4B)  │ ID(8B)   │ CRC32(4B)│
├──────────────┼─────────┼─────────┼──────────┼──────────┼──────────┼──────────┼──────────┤
│ 0x4C415043   │  0x01   │  0x00    │  0x0001  │ 0x0000FF │ 0x00..01 │ 0x1234.. │
│ "LAPC" LE    │ Version │ Bit flags│ Msg Type │ Payload  │ Msg ID   │ Checksum │
│              │         │          │    LE    │ Size LE  │    LE    │    LE    │
└──────────────┴─────────┴─────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
```

## Field Specifications

### 1. Magic Number (4 bytes, offset 0)
- **Value**: `0x4C415043` ("LAPC" in ASCII)
- **Encoding**: Little-Endian
- **Purpose**: Protocol identification and frame synchronization
- **Validation**: Must match exactly or reject message

### 2. Version (1 byte, offset 4)
- **Current Value**: `0x01`
- **Purpose**: Protocol version for backward compatibility
- **Validation**: Accept version 1, reject others with clear error

### 3. Flags (1 byte, offset 5)
- **Bit 0 (0x01)**: Compressed payload (zstd)
- **Bit 1 (0x02)**: Encrypted payload (reserved, not implemented)
- **Bit 2 (0x04)**: Streaming message (part of stream)
- **Bit 3 (0x08)**: Priority message (process first)
- **Bit 4 (0x10)**: Request (expects response)
- **Bit 5 (0x20)**: Response (to a request)
- **Bit 6-7**: Reserved, must be 0

### 4. Message Type (2 bytes, offset 6)
- **Encoding**: Little-Endian
- **Values**: See Message Type Registry below
- **Validation**: Known types only, reject unknown

### 5. Payload Length (4 bytes, offset 8)
- **Encoding**: Little-Endian
- **Range**: 0 to 10,485,760 (10MB max)
- **Purpose**: Size of payload in bytes (after header, before any trailing data)
- **Note**: For compressed messages, this is the compressed size

### 6. Message ID (8 bytes, offset 12)
- **Encoding**: Little-Endian
- **Purpose**: Unique message identifier for correlation
- **Generation**: Monotonic counter or timestamp+random
- **Note**: Responses echo the request's ID

### 7. CRC32 Checksum (4 bytes, offset 20)
- **Encoding**: Little-Endian
- **Algorithm**: CRC32 (IEEE 802.3)
- **Coverage**: Entire message (header + payload)
- **Calculation**: Set to 0 during calculation, then write result
- **Validation**: Must match or reject message

## Message Type Registry

```rust
// Core Protocol Messages
0x0001 - CompletionRequest
0x0002 - CompletionResponse  
0x0003 - StreamChunk
0x0004 - Error
0x0005 - Heartbeat

// Control Messages
0x0010 - Handshake
0x0011 - HandshakeAck
0x0012 - Disconnect

// AI-Specific Messages (Codex)
0x0100 - AskRequest
0x0101 - AskResponse
0x0102 - EditRequest
0x0103 - EditResponse
0x0104 - ChatMessage
0x0105 - ToolCall
0x0106 - ToolResult
```

## Payload Format

After the 24-byte header, the payload follows:
- **Uncompressed**: rkyv-serialized `MessagePayload` 
- **Compressed**: zstd-compressed rkyv data (when flag 0x01 set)

The payload includes:
1. Original timestamp (preserved from sender)
2. Message-specific data per type

## Implementation Requirements

### Encoding Process
1. Serialize payload with rkyv
2. If payload > 1KB, compress with zstd level 3
3. Build header with all fields
4. Calculate CRC32 over complete message
5. Write CRC32 to header
6. Send header + payload as single frame

### Decoding Process  
1. Read 24-byte header
2. Validate magic number
3. Check protocol version
4. Validate payload length <= 10MB
5. Read payload bytes
6. Verify CRC32 checksum
7. Decompress if flag set
8. Deserialize with rkyv

### Error Handling
- Invalid magic: Close connection
- Wrong version: Send error response
- Bad CRC32: Request retransmission
- Oversized: Reject with error
- Unknown type: Log and skip

## Performance Targets
- Header parsing: < 100ns
- CRC32 calculation: < 1μs for 1KB message
- Compression threshold: 1KB (only compress larger)
- Zero-copy: Use rkyv archived types directly when possible

## Security Considerations
- Always validate magic and version first
- Enforce maximum message size
- Verify CRC32 before processing
- Rate limit per connection
- Log protocol violations

## Migration Notes
All components must be updated atomically:
1. `src/ipc/binary_codec.rs` - Full message encode/decode
2. `src/ipc/zero_copy_codec.rs` - Streaming codec
3. `src/ipc/ipc_server.rs` - Server I/O handling
4. Tests must verify cross-codec compatibility

## Compliance Verification
Run `cargo test --test protocol_compliance` to verify implementation matches this spec.
