# Lapce-AI IPC System Upgrade Guide

## Upgrading from v0.x to v1.0.0

### Breaking Changes

#### 1. Protocol Version
- Old: 16-byte header with mixed endianness
- New: 24-byte canonical header with little-endian encoding
- Action: Update all clients to use new protocol version

#### 2. Header Format
```rust
// Old format (16 bytes)
struct OldHeader {
    magic: u32,      // BE
    version: u8,
    msg_type: u16,   // BE
    length: u32,     // BE
    msg_id: u64,     // BE
}

// New format (24 bytes)
struct CanonicalHeader {
    magic: u32,      // LE - 0x4C504149 ("LPAI")
    version: u8,     // 2
    flags: u8,       // Compression, priority
    msg_type: u16,   // LE
    length: u32,     // LE
    msg_id: u64,     // LE
    crc32: u32,      // LE
}
```

#### 3. Shared Memory Paths
- Old: `/dev/shm/lapce_ipc_*`
- New: `/dev/shm/lapce_ipc_v2_*`
- Action: Update shared memory segment names

### Migration Steps

#### Step 1: Stop Current Services
```bash
sudo systemctl stop lapce-ipc
sudo systemctl stop lapce-ai
```

#### Step 2: Backup Configuration
```bash
sudo cp /etc/lapce-ipc.toml /etc/lapce-ipc.toml.backup
```

#### Step 3: Install New Version
```bash
cargo install --path . --features ipc
sudo cp target/release/lapce_ipc_server /usr/local/bin/
sudo cp lapce-ipc.service /etc/systemd/system/
```

#### Step 4: Update Configuration
```toml
# /etc/lapce-ipc.toml
[server]
bind_address = "127.0.0.1:8080"
protocol_version = 2  # New field

[compatibility]
enable_legacy_support = false  # Set to true for gradual migration
```

#### Step 5: Start New Services
```bash
sudo systemctl daemon-reload
sudo systemctl start lapce-ipc
sudo systemctl status lapce-ipc
```

### Client Migration

#### Rust Clients
```rust
// Old
use lapce_ai::ipc::Client;
let client = Client::connect("127.0.0.1:8080")?;

// New
use lapce_ai::ipc::{Client, ProtocolVersion};
let client = Client::builder()
    .protocol_version(ProtocolVersion::V2)
    .connect("127.0.0.1:8080")?;
```

#### Connection Pool Updates
```rust
// Old
let pool = ConnectionPool::new(config);

// New
let pool = ConnectionPool::builder()
    .max_connections(100)
    .reuse_threshold(0.95)
    .health_check_interval(Duration::from_secs(30))
    .build()?;
```

### Rollback Procedure

If issues occur, rollback to previous version:

```bash
# Stop new service
sudo systemctl stop lapce-ipc

# Restore old binary
sudo cp /usr/local/bin/lapce_ipc_server.backup /usr/local/bin/lapce_ipc_server

# Restore old config
sudo cp /etc/lapce-ipc.toml.backup /etc/lapce-ipc.toml

# Start old service
sudo systemctl start lapce-ipc
```

### Monitoring During Upgrade

Monitor key metrics during upgrade:
- Error rate: Should remain <0.1%
- Latency: P99 should stay <10µs
- Memory: RSS should stay <3MB
- Connection pool reuse: Should maintain >95%

Access metrics at: http://localhost:9090/metrics

### Troubleshooting

#### Issue: Protocol version mismatch
```
Error: Protocol version mismatch: expected 2, got 1
```
Solution: Ensure all clients are updated to v1.0.0

#### Issue: Shared memory segment not found
```
Error: Failed to open shared memory: /dev/shm/lapce_ipc_control
```
Solution: Use new segment names with v2 prefix

#### Issue: CRC32 validation failures
```
Error: CRC32 mismatch: expected 0x12345678, got 0x87654321
```
Solution: Check for network corruption or incompatible client

### Support

For issues during upgrade:
- GitHub Issues: https://github.com/lapce/lapce-ai/issues
- Documentation: https://docs.lapce.dev/ipc
- Community Discord: https://discord.gg/lapce
