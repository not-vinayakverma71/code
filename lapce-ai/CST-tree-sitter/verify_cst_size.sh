#!/bin/bash
echo "VERIFYING CST SIZE PER FILE"
echo "============================"

# Run test and extract key metrics
../target/release/test_real_cst_memory 2>&1 | grep -A 20 "RESULTS" | grep -E "Files parsed:|Total lines:|Total bytes:|CST Memory:|KB per CST:"

echo ""
echo "CALCULATING REALISTIC SCENARIOS:"
echo "================================"
echo ""

# From actual test data
FILES=3000
LINES=46000
CST_MB=36.19
SOURCE_MB=0.83

echo "Actual test results:"
echo "  $FILES files, $LINES lines → ${CST_MB} MB CST memory"
echo ""

# Calculate metrics
LINES_PER_FILE=$(echo "$LINES / $FILES" | bc)
MB_PER_FILE=$(echo "scale=6; $CST_MB / $FILES" | bc)
KB_PER_FILE=$(echo "scale=2; $MB_PER_FILE * 1024" | bc)
LINES_PER_MB=$(echo "scale=0; $LINES / $CST_MB" | bc)

echo "Per-file metrics:"
echo "  Lines per file: $LINES_PER_FILE"
echo "  MB per CST: $MB_PER_FILE"
echo "  KB per CST: $KB_PER_FILE"
echo ""

echo "Scaling to realistic codebases:"
echo ""

# Small project: 100 files, 100 lines each
SMALL_FILES=100
SMALL_LINES=100
SMALL_TOTAL_LINES=$((SMALL_FILES * SMALL_LINES))
SMALL_MB=$(echo "scale=2; $SMALL_TOTAL_LINES / $LINES_PER_MB" | bc)
echo "1. Small project (100 files × 100 lines = 10K lines):"
echo "   → ${SMALL_MB} MB"
echo ""

# Medium project: 1000 files, 500 lines each
MED_FILES=1000
MED_LINES=500
MED_TOTAL_LINES=$((MED_FILES * MED_LINES))
MED_MB=$(echo "scale=2; $MED_TOTAL_LINES / $LINES_PER_MB" | bc)
echo "2. Medium project (1000 files × 500 lines = 500K lines):"
echo "   → ${MED_MB} MB"
echo ""

# Large project: 5000 files, 1000 lines each
LARGE_FILES=5000
LARGE_LINES=1000
LARGE_TOTAL_LINES=$((LARGE_FILES * LARGE_LINES))
LARGE_MB=$(echo "scale=2; $LARGE_TOTAL_LINES / $LINES_PER_MB" | bc)
echo "3. Large project (5000 files × 1000 lines = 5M lines):"
echo "   → ${LARGE_MB} MB (${LARGE_MB}MB / 1024 = $(echo "scale=2; $LARGE_MB / 1024" | bc) GB)"
echo ""

# Your scenario: 10K files, 1K lines each
YOUR_FILES=10000
YOUR_LINES=1000
YOUR_TOTAL_LINES=$((YOUR_FILES * YOUR_LINES))
YOUR_MB=$(echo "scale=2; $YOUR_TOTAL_LINES / $LINES_PER_MB" | bc)
YOUR_GB=$(echo "scale=2; $YOUR_MB / 1024" | bc)
echo "4. Your scenario (10K files × 1K lines = 10M lines):"
echo "   → ${YOUR_MB} MB = ${YOUR_GB} GB"
echo ""

echo "⚠️  REALITY CHECK:"
echo "   For 10K files, you need ~${YOUR_GB} GB of RAM!"
echo "   The 5 MB requirement is ${YOUR_MB}x over budget!"
