// Cross-OS success criteria validation for IPC, Binary Protocol, Connection Pool
// This suite is designed to run on Linux, macOS, and Windows without external services.
// It validates key invariants and provides measurable metrics printed to stdout for CI.

#![cfg(test)]

use std::time::{Duration, Instant};
use tokio_util::codec::{Encoder, Decoder};

// --- Binary protocol checks ---
#[test]
fn test_canonical_header_size_and_constants() {
    use lapce_ai_rust::ipc::binary_codec::{HEADER_SIZE, MAGIC_HEADER, PROTOCOL_VERSION};
    assert_eq!(HEADER_SIZE, 24, "Canonical header must be 24 bytes");
    assert_eq!(MAGIC_HEADER, 0x4C415043, "Magic must be 'LAPC' LE");
    assert_eq!(PROTOCOL_VERSION, 1, "Protocol version must be 1");
}

#[test]
fn test_binary_codec_roundtrip() {
    use lapce_ai_rust::ipc::binary_codec::{BinaryCodec, Message, MessagePayload, MessageType, CompletionRequest};

    let mut codec = BinaryCodec::new();
    let msg = Message {
        id: 42,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "ping".into(),
            model: "gpt-test".into(),
            max_tokens: 16,
            temperature: 0.1,
            stream: false,
        }),
        timestamp: 123456,
    };

    let t0 = Instant::now();
    let encoded = codec.encode(&msg).expect("encode");
    let decoded = codec.decode(&encoded).expect("decode");
    let dt = t0.elapsed();

    assert_eq!(decoded.id, msg.id);
    assert_eq!(decoded.msg_type, msg.msg_type);
    // Print metric for CI logs
    eprintln!("binary_codec_roundtrip_ns={} size_bytes={}", dt.as_nanos(), encoded.len());
}

#[test]
fn test_zero_copy_codec_roundtrip() {
    use bytes::BytesMut;
    use lapce_ai_rust::ipc::zero_copy_codec::ZeroCopyCodec;
    use lapce_ai_rust::ipc::binary_codec::{Message, MessagePayload, MessageType, CompletionRequest, HEADER_SIZE};

    let mut codec = ZeroCopyCodec::new();
    let msg = Message {
        id: 1,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "hello".into(),
            model: "gpt".into(),
            max_tokens: 8,
            temperature: 0.0,
            stream: false,
        }),
        timestamp: 777,
    };

    let mut buf = BytesMut::new();
    let t0 = Instant::now();
    codec.encode(msg.clone(), &mut buf).expect("encode");
    assert!(buf.len() >= HEADER_SIZE);

    // Decode using ZeroCopyCodec::decode contract
    let decoded = {
        // decode in-place using the same buffer
        let mut local = buf.clone();
        codec.decode(&mut local).expect("decode call").expect("Some")
    };

    let dt = t0.elapsed();
    assert_eq!(decoded.msg_type, msg.msg_type);
    eprintln!("zero_copy_roundtrip_ns={} frame_len={}", dt.as_nanos(), buf.len());
}

// --- Shared memory checks (cross-OS) ---
// Note: macOS GitHub Actions runners don't allow shm_open due to sandbox restrictions
#[cfg(target_os = "linux")]
#[test]
fn test_shared_memory_roundtrip_posix() {
    use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;

    let mut shm = SharedMemoryBuffer::create("/ci_shm", 1 * 1024 * 1024).expect("create shm");
    let payload = b"hello-shm";

    shm.write(payload).expect("write");
    let got = shm.read().expect("read some");
    assert_eq!(got, payload);
}

#[cfg(target_os = "windows")]
#[test]
fn test_shared_memory_roundtrip_windows() {
    use lapce_ai_rust::ipc::SharedMemoryBuffer;
    
    let mut shm = SharedMemoryBuffer::create("ci_shm_win", 1 * 1024 * 1024).expect("create shm");
    let payload = b"hello-shm-windows";
    
    shm.write(payload).expect("write");
    let got = shm.read().expect("read some");
    assert_eq!(got, payload);
}

// --- Connection pool non-network invariants ---
// Construct the pool with min_idle=0 to avoid pre-warm external calls; validate stats and config wiring.
#[tokio::test]
async fn test_connection_pool_construct_stats() {
    use lapce_ai_rust::connection_pool_manager::{ConnectionPoolManager, PoolConfig};

    let cfg = PoolConfig {
        max_connections: 10,
        min_idle: 0, // avoid pre-warm
        ..Default::default()
    };

    let pool = ConnectionPoolManager::new(cfg.clone()).await.expect("pool new");
    let stats = pool.get_stats();
    assert_eq!(stats.total_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
    // Print current config hints for CI visibility
    eprintln!("pool_max_connections={} min_idle={}", cfg.max_connections, cfg.min_idle);
}

// --- Micro performance smoke (encode/decode latency budget) ---
// Provide a very small budget check that should pass on all OS runners; heavy gates remain in Linux benches/CI.
#[test]
fn test_micro_latency_smoke() {
    use lapce_ai_rust::ipc::binary_codec::{BinaryCodec, Message, MessagePayload, MessageType, CompletionRequest};

    let mut codec = BinaryCodec::new();
    let msg = Message {
        id: 99,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "x".repeat(32),
            model: "m".into(),
            max_tokens: 4,
            temperature: 0.0,
            stream: false,
        }),
        timestamp: 0,
    };

    // Run N iterations and assert an ultra-relaxed budget that should pass anywhere
    let n = 500;
    let t0 = Instant::now();
    for _ in 0..n {
        let enc = codec.encode(&msg).unwrap();
        let _ = codec.decode(&enc).unwrap();
    }
    let dt = t0.elapsed();
    let per_op_ns = dt.as_nanos() as f64 / (n as f64);
    eprintln!("micro_latency_ns_per_op={}", per_op_ns);

    // Very relaxed to avoid flakes on slower runners; Linux has strict gates elsewhere.
    assert!(per_op_ns < 2_000_000.0, "micro budget too slow: {}ns/op", per_op_ns);
}
