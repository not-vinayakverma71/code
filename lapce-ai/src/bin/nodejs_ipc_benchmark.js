#!/usr/bin/env node

// Node.js IPC Benchmark for comparison
const cluster = require('cluster');
const net = require('net');
const { performance } = require('perf_hooks');

const TEST_DURATION = 10000; // 10 seconds
const MESSAGE_SIZE = 256;
const SOCKET_PATH = '/tmp/nodejs_ipc_bench.sock';

if (cluster.isMaster) {
    console.log('🚀 Node.js IPC Benchmark');
    console.log('=' + '='.repeat(79));
    
    // Clean up any existing socket
    try {
        require('fs').unlinkSync(SOCKET_PATH);
    } catch(e) {}
    
    // Create server
    let messageCount = 0;
    let totalLatency = 0;
    let startTime = performance.now();
    
    const server = net.createServer((socket) => {
        socket.on('data', (data) => {
            // Echo back (round-trip)
            socket.write(data);
            messageCount++;
        });
    });
    
    server.listen(SOCKET_PATH, () => {
        console.log('Server listening on', SOCKET_PATH);
        
        // Fork worker
        const worker = cluster.fork();
        
        // Run for TEST_DURATION
        setTimeout(() => {
            const duration = (performance.now() - startTime) / 1000;
            const throughput = Math.floor(messageCount / duration);
            
            console.log('\n📊 Results:');
            console.log(`  Messages:    ${messageCount}`);
            console.log(`  Duration:    ${duration.toFixed(2)}s`);
            console.log(`  Throughput:  ${throughput} msg/sec`);
            
            // Cleanup
            worker.kill();
            server.close();
            process.exit(0);
        }, TEST_DURATION);
    });
    
} else {
    // Worker process - client
    const client = new net.Socket();
    const message = Buffer.alloc(MESSAGE_SIZE, 0x42);
    
    client.connect(SOCKET_PATH, () => {
        console.log('Client connected');
        
        // Continuous send loop
        function sendLoop() {
            client.write(message);
        }
        
        client.on('data', () => {
            // Received echo, send next
            setImmediate(sendLoop);
        });
        
        // Start sending
        sendLoop();
    });
    
    client.on('error', (err) => {
        console.error('Client error:', err);
    });
}
