#!/bin/bash
# Measure IPC server baseline memory in release mode
# Target: <4MB RSS (integrated system: IPC + codec + connection pool)

set -e

echo "Building minimal IPC server in release mode..."
cargo build --release --bin ipc_minimal_server 2>&1 | tail -5

echo ""
echo "Starting minimal IPC server..."
./target/release/ipc_minimal_server &
SERVER_PID=$!

# Wait for server to initialize
sleep 2

echo "Server PID: $SERVER_PID"
echo ""
echo "Memory measurement:"

# Get RSS in KB
RSS_KB=$(cat /proc/$SERVER_PID/status | grep VmRSS | awk '{print $2}')
RSS_MB=$(echo "scale=2; $RSS_KB / 1024" | bc)

echo "  VmRSS: ${RSS_MB}MB"
echo ""

# Check against target (4MB for integrated system)
TARGET_MB=4.0
PASSED=$(echo "$RSS_MB < $TARGET_MB" | bc -l)

if [ "$PASSED" -eq 1 ]; then
    echo "  Status: ✅ PASSED - ${RSS_MB}MB < ${TARGET_MB}MB"
    EXIT_CODE=0
else
    echo "  Status: ❌ FAILED - ${RSS_MB}MB >= ${TARGET_MB}MB"
    EXIT_CODE=1
fi

# Cleanup
kill $SERVER_PID 2>/dev/null || true
sleep 1

exit $EXIT_CODE
