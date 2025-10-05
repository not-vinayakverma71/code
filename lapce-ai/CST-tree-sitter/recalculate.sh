#!/bin/bash

echo "RECALCULATING PROPERLY"
echo "======================"
echo ""

echo "From our test:"
echo "  3,000 files = 36 MB"
echo "  46,000 lines = 36 MB"
echo "  12.4 KB per file"
echo ""

echo "Scaling to 10,000 files:"
echo "========================"
echo ""

FILES=10000
KB_PER_FILE=12.4
TOTAL_KB=$(echo "$FILES * $KB_PER_FILE" | bc)
TOTAL_MB=$(echo "$TOTAL_KB / 1024" | bc)

echo "10,000 files × 12.4 KB = ${TOTAL_KB} KB"
echo "= ${TOTAL_MB} MB"
echo ""

echo "NOT 7.5 GB!"
echo ""

echo "Where did 7.5 GB come from?"
echo "==========================="
echo ""
echo "I calculated:"
echo "  10M lines / 1,271 lines per MB = 7,867 MB"
echo ""
echo "But that assumed 10K files × 1K lines each!"
echo "The test files average only 15 lines each!"
echo ""

ACTUAL_LINES_PER_FILE=15
ACTUAL_TOTAL_LINES=$(echo "$FILES * $ACTUAL_LINES_PER_FILE" | bc)
LINES_PER_MB=1271
ACTUAL_MB=$(echo "$ACTUAL_TOTAL_LINES / $LINES_PER_MB" | bc)

echo "Real calculation:"
echo "  10,000 files × 15 lines = ${ACTUAL_TOTAL_LINES} lines"
echo "  ${ACTUAL_TOTAL_LINES} / 1,271 = ${ACTUAL_MB} MB"
echo ""

echo "So for REALISTIC files (15 lines avg):"
echo "  10K files = ~118 MB"
echo ""

echo "For LARGER files (1K lines each):"
LARGE_LINES=1000
LARGE_TOTAL=$(echo "$FILES * $LARGE_LINES" | bc)
LARGE_MB=$(echo "$LARGE_TOTAL / $LINES_PER_MB" | bc)
echo "  10K files × 1K lines = ${LARGE_TOTAL} lines"
echo "  ${LARGE_TOTAL} / 1,271 = ${LARGE_MB} MB (7.7 GB)"
echo ""

echo "======================================"
echo "THE REAL NUMBERS"
echo "======================================"
echo ""
echo "Small files (15 lines):    118 MB for 10K files"
echo "Medium files (100 lines):  786 MB for 10K files"
echo "Large files (1K lines):  7,867 MB for 10K files"
echo ""
echo "Windsurf: 4 GB total (with Electron + LSPs + etc)"
echo "Target: 400-800 MB (5-10x less)"
echo ""
echo "Our CSTs for realistic codebase (100 lines/file avg):"
echo "  786 MB - WITHIN TARGET!"
echo ""
echo "Our CSTs for large files (1K lines/file):"
echo "  7,867 MB - 10-20x OVER TARGET!"
