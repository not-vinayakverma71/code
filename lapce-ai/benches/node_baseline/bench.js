#!/usr/bin/env node
/**
 * Node.js Baseline Benchmark for IPC Message Processing
 * Measures JSON serialization/deserialization throughput
 */

const ITERATIONS = 1_000_000;

// Test message structure matching Rust tests
function createTestMessage(size) {
    return {
        id: 12345,
        timestamp: 1697203200,
        content: 'A'.repeat(Math.floor(size / 2)),
        metadata: ['meta1', 'meta2'],
        data: Buffer.alloc(Math.floor(size / 4)).toJSON().data
    };
}

// Benchmark JSON serialization + deserialization
function benchmarkJSON() {
    const msg = createTestMessage(1024);
    
    console.log('Node.js JSON Benchmark');
    console.log('======================\n');
    
    const start = process.hrtime.bigint();
    
    for (let i = 0; i < ITERATIONS; i++) {
        const json = JSON.stringify(msg);
        const decoded = JSON.parse(json);
        // Touch fields to prevent optimization
        const _ = decoded.id;
    }
    
    const end = process.hrtime.bigint();
    const duration = Number(end - start) / 1_000_000; // Convert to ms
    const throughput = (ITERATIONS / duration) * 1000; // msg/s
    
    console.log(`Iterations: ${ITERATIONS.toLocaleString()}`);
    console.log(`Duration: ${duration.toFixed(2)}ms`);
    console.log(`Throughput: ${(throughput / 1000).toFixed(2)}K msg/s`);
    console.log(`Per-operation: ${(duration * 1000 / ITERATIONS).toFixed(2)}µs`);
    
    // Output for Rust parser
    console.log('\n---RESULT---');
    console.log(JSON.stringify({
        iterations: ITERATIONS,
        duration_ms: duration,
        throughput_msg_per_sec: throughput,
        per_op_us: (duration * 1000 / ITERATIONS)
    }));
}

benchmarkJSON();
