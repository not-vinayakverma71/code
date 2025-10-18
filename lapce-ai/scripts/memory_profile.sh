#!/bin/bash

# Memory profiling script for lapce-ai-rust
# Target: <3MB memory footprint

echo "🔍 Starting Memory Profiling for lapce-ai-rust"
echo "================================================"

# Build in release mode with debug symbols
echo "📦 Building release with debug symbols..."
cd /home/verma/lapce/lapce-ai-rust
RUSTFLAGS="-g" cargo build --release

# Create test program
cat > /tmp/memory_test.rs << 'EOF'
use lapce_ai_rust::shared_memory_transport::SharedMemoryTransport;
use lapce_ai_rust::ipc_server::IpcServer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Starting memory test...");
    
    // Create SharedMemory transport
    let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
    println!("Created SharedMemoryTransport");
    
    // Create IPC server
    let server = Arc::new(IpcServer::new("/tmp/test.sock").await.unwrap());
    println!("Created IPC server");
    
    // Simulate some activity
    for i in 0..100 {
        let msg = format!("Test message {}", i).into_bytes();
        transport.send(&msg).await.unwrap();
        if i % 10 == 0 {
            println!("Sent {} messages", i);
        }
    }
    
    // Keep alive for profiling
    println!("Keeping alive for profiling... Press Ctrl+C to stop");
    sleep(Duration::from_secs(60)).await;
}
EOF

# Compile test program
echo "📦 Building test program..."
rustc --edition 2021 -L target/release/deps /tmp/memory_test.rs -o /tmp/memory_test \
    --extern lapce_ai_rust=target/release/libllapce_ai_rust.rlib \
    --extern tokio=target/release/deps/libtokio*.rlib 2>/dev/null || {
    echo "⚠️  Could not build standalone test, using cargo run instead"
    USE_CARGO=1
}

# Method 1: Using /proc for quick memory check
echo ""
echo "📊 Method 1: Quick memory check with /proc"
echo "-------------------------------------------"
if [ -z "$USE_CARGO" ]; then
    /tmp/memory_test &
else
    cargo run --release --example memory_test 2>/dev/null &
fi
PID=$!
sleep 2

if [ -d /proc/$PID ]; then
    echo "Process PID: $PID"
    
    # Get memory stats
    RSS=$(cat /proc/$PID/status | grep VmRSS | awk '{print $2}')
    VSZ=$(cat /proc/$PID/status | grep VmSize | awk '{print $2}')
    HEAP=$(cat /proc/$PID/status | grep VmData | awk '{print $2}')
    
    echo "Memory Usage:"
    echo "  RSS (Resident Set Size): ${RSS} KB"
    echo "  VSZ (Virtual Size): ${VSZ} KB" 
    echo "  Heap: ${HEAP} KB"
    
    # Check if under 3MB
    RSS_MB=$((RSS / 1024))
    if [ $RSS_MB -lt 3 ]; then
        echo "✅ Memory usage under 3MB target! (${RSS_MB}MB)"
    else
        echo "❌ Memory usage exceeds 3MB target (${RSS_MB}MB)"
    fi
    
    kill $PID 2>/dev/null
fi

# Method 2: Using valgrind massif (if available)
if command -v valgrind &> /dev/null; then
    echo ""
    echo "📊 Method 2: Detailed profiling with valgrind massif"
    echo "----------------------------------------------------"
    
    timeout 10 valgrind --tool=massif --time-unit=B --pages-as-heap=yes \
        --massif-out-file=/tmp/massif.out \
        cargo run --release --example memory_test 2>&1 | head -20
    
    if [ -f /tmp/massif.out ]; then
        ms_print /tmp/massif.out 2>/dev/null | head -50 || {
            echo "Peak memory from massif:"
            grep "mem_heap_B" /tmp/massif.out | tail -1
        }
    fi
fi

# Method 3: Using heaptrack (if available)  
if command -v heaptrack &> /dev/null; then
    echo ""
    echo "📊 Method 3: Heap profiling with heaptrack"
    echo "------------------------------------------"
    
    timeout 10 heaptrack cargo run --release --example memory_test 2>&1 | head -20
    
    if ls heaptrack.*.zst &> /dev/null; then
        heaptrack_print heaptrack.*.zst | head -50
        rm heaptrack.*.zst
    fi
fi

# Method 4: Using time -v for peak memory
echo ""
echo "📊 Method 4: Peak memory with GNU time"
echo "--------------------------------------"
if command -v /usr/bin/time &> /dev/null; then
    timeout 5 /usr/bin/time -v cargo run --release --example memory_test 2>&1 | \
        grep -E "(Maximum resident set size|User time|System time)" || true
fi

echo ""
echo "================================================"
echo "📈 Memory Profiling Complete!"
echo ""
echo "Optimization suggestions:"
echo "  1. Use Arc<str> instead of String for immutable strings"
echo "  2. Box large structs that are rarely accessed"
echo "  3. Use ArrayVec for small fixed-size collections"
echo "  4. Implement object pooling for frequently allocated types"
echo "  5. Use lazy_static for singletons"
echo "  6. Consider using bytes::Bytes for zero-copy operations"
