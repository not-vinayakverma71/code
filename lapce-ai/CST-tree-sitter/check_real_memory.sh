#!/bin/bash

echo "CHECKING IF OUR MEASUREMENT IS WRONG"
echo "======================================"
echo ""

# Check actual process memory for the test
echo "Running test and checking actual memory..."
../target/release/test_real_cst_memory > /tmp/test_output.txt 2>&1 &
PID=$!

sleep 2

for i in {1..30}; do
    if ps -p $PID > /dev/null; then
        RSS=$(ps -o rss= -p $PID)
        VSZ=$(ps -o vsz= -p $PID)
        echo "After ${i}00 files: RSS=${RSS} KB (${RSS}MB / 1024), VSZ=${VSZ} KB"
        sleep 0.5
    else
        break
    fi
done

wait $PID

echo ""
tail -30 /tmp/test_output.txt

echo ""
echo "======================================"
echo "ANALYSIS"
echo "======================================"
echo ""
echo "If RSS is actually much lower than 36 MB,"
echo "then our /proc/self/status measurement is wrong!"
