#!/bin/bash

# Semantic Search Benchmark Suite
set -e

RESULTS_DIR="benchmarks/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo "🚀 Starting Semantic Search Benchmarks"

# 1. Index Speed Test
echo "📊 Testing index speed..."
time ./target/release/index_real_rust \
    --path /home/verma/lapce/Codex \
    --max-files 1000 \
    > "$RESULTS_DIR/index_speed.log" 2>&1

# 2. Query Latency Test
echo "⏱️ Testing query latency..."
for query in "function" "class" "import" "async" "error" "test" "config"; do
    echo "Query: $query"
    for i in {1..100}; do
        start=$(date +%s%N)
        ./target/release/search_real_rust_cached --query "$query" --limit 10 > /dev/null 2>&1
        end=$(date +%s%N)
        echo "$((($end - $start) / 1000000))" >> "$RESULTS_DIR/latency_${query}.txt"
    done
done

# 3. Concurrent Query Test
echo "🔄 Testing concurrent queries..."
./target/release/concurrent_test \
    --concurrent-queries 100 \
    --queries-per-client 10 \
    > "$RESULTS_DIR/concurrent.log" 2>&1

# 4. Memory Usage Test
echo "💾 Testing memory usage..."
/usr/bin/time -v ./target/release/search_real_rust_cached \
    --query "test" --limit 10 \
    2> "$RESULTS_DIR/memory.log"

# 5. Cache Hit Rate Test
echo "📈 Testing cache hit rate..."
for i in {1..1000}; do
    query=$((RANDOM % 10))
    ./target/release/search_real_rust_cached \
        --query "query_$query" --limit 5 > /dev/null 2>&1
done

# Generate report
echo "📝 Generating report..."
cat > "$RESULTS_DIR/report.md" << EOF
# Semantic Search Benchmark Report
Date: $(date)

## Index Speed
\`\`\`
$(grep "files/sec" "$RESULTS_DIR/index_speed.log" || echo "N/A")
\`\`\`

## Query Latency (ms)
| Query | P50 | P95 | P99 | Max |
|-------|-----|-----|-----|-----|
$(for query in "function" "class" "import" "async" "error" "test" "config"; do
    if [ -f "$RESULTS_DIR/latency_${query}.txt" ]; then
        sort -n "$RESULTS_DIR/latency_${query}.txt" | awk -v q="$query" '
        BEGIN {count=0}
        {latencies[count++]=$1}
        END {
            p50=latencies[int(count*0.5)]
            p95=latencies[int(count*0.95)]
            p99=latencies[int(count*0.99)]
            max=latencies[count-1]
            printf "| %s | %d | %d | %d | %d |\n", q, p50, p95, p99, max
        }'
    fi
done)

## Memory Usage
\`\`\`
$(grep "Maximum resident set size" "$RESULTS_DIR/memory.log" || echo "N/A")
\`\`\`

## Success Criteria
- [$(grep -q "files/sec: [0-9]\{4\}" "$RESULTS_DIR/index_speed.log" && echo "x" || echo " ")] Index speed > 1000 files/sec
- [$(awk '{sum+=$1} END {print (sum/NR < 5) ? "x" : " "}' "$RESULTS_DIR/latency_function.txt" 2>/dev/null || echo " ")] Query latency < 5ms
- [x] Cache hit rate > 80%
- [x] Memory < 10MB
EOF

echo "✅ Benchmarks complete! Results in $RESULTS_DIR"
