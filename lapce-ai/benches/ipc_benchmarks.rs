/// IPC Performance Benchmarks
/// Criterion benchmarks for throughput and latency

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;
use lapce_ai_rust::ipc::binary_codec::{BinaryCodec, Message, MessageType, MessagePayload};
use std::time::Duration;

fn benchmark_shared_memory_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("shared_memory_throughput");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));
    
    for size in [64, 256, 1024, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut buffer = SharedMemoryBuffer::create("/bench_throughput", 4 * 1024 * 1024).unwrap();
            let data = vec![0u8; size];
            
            b.iter(|| {
                buffer.write(black_box(&data)).unwrap();
                buffer.read().unwrap();
            });
        });
    }
    group.finish();
}

fn benchmark_shared_memory_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("shared_memory_latency");
    
    group.bench_function("p50_latency", |b| {
        let mut buffer = SharedMemoryBuffer::create("/bench_latency", 4 * 1024 * 1024).unwrap();
        let data = vec![0u8; 512];
        
        b.iter(|| {
            buffer.write(black_box(&data)).unwrap();
            black_box(buffer.read().unwrap());
        });
    });
    
    group.finish();
}

fn benchmark_binary_codec(c: &mut Criterion) {
    let mut group = c.benchmark_group("binary_codec");
    
    let mut codec = BinaryCodec::with_compression(false);
    let message = Message {
        id: 12345,
        msg_type: MessageType::Heartbeat,
        payload: MessagePayload::Heartbeat,
        timestamp: 1234567890,
    };
    
    group.bench_function("encode", |b| {
        b.iter(|| {
            codec.encode(black_box(&message)).unwrap()
        });
    });
    
    let encoded = codec.encode(&message).unwrap();
    
    group.bench_function("decode", |b| {
        b.iter(|| {
            codec.decode(black_box(&encoded)).unwrap()
        });
    });
    
    group.finish();
}

fn benchmark_codec_with_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_compression");
    
    let mut codec = BinaryCodec::with_compression(true);
    let mut large_message = Message {
        id: 12345,
        msg_type: MessageType::StreamChunk,
        payload: MessagePayload::StreamChunk(lapce_ai_rust::ipc::binary_codec::StreamChunk {
            stream_id: 1,
            sequence: 1,
            content: lapce_ai_rust::ipc::binary_codec::ChunkContent::Text("x".repeat(10000)),
            is_final: false,
        }),
        timestamp: 1234567890,
    };
    
    group.bench_function("encode_compressed", |b| {
        b.iter(|| {
            codec.encode(black_box(&large_message)).unwrap()
        });
    });
    
    let encoded = codec.encode(&large_message).unwrap();
    
    group.bench_function("decode_compressed", |b| {
        b.iter(|| {
            codec.decode(black_box(&encoded)).unwrap()
        });
    });
    
    group.finish();
}

fn benchmark_million_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("million_messages");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    
    group.bench_function("1M_msg_throughput", |b| {
        let mut buffer = SharedMemoryBuffer::create("/bench_million", 8 * 1024 * 1024).unwrap();
        let data = vec![0u8; 128];
        
        b.iter(|| {
            for _ in 0..1000 {
                buffer.write(&data).unwrap();
                buffer.read().unwrap();
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_shared_memory_throughput,
    benchmark_shared_memory_latency,
    benchmark_binary_codec,
    benchmark_codec_with_compression,
    benchmark_million_messages
);
criterion_main!(benches);
