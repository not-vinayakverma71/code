#!/bin/bash
# Performance regression check script for CST-tree-sitter
set -e

THRESHOLD=${1:-10}
REGRESSION_FOUND=0

echo "Performance Regression Check"
echo "============================="
echo "Threshold: ${THRESHOLD}%"
echo ""

# Check if criterion generated comparison reports
if [ -d "target/criterion" ]; then
    # Parse criterion output for regressions
    for report in target/criterion/*/base/estimates.json; do
        if [ -f "$report" ]; then
            benchmark=$(basename $(dirname $(dirname "$report")))
            echo "Checking benchmark: $benchmark"
            
            # Check for performance changes in the report
            if [ -f "target/criterion/$benchmark/change/estimates.json" ]; then
                # Extract mean time change percentage using jq if available
                if command -v jq &> /dev/null; then
                    change=$(jq -r '.mean.point_estimate' "target/criterion/$benchmark/change/estimates.json" 2>/dev/null || echo "0")
                    change_pct=$(echo "$change * 100" | bc -l 2>/dev/null || echo "0")
                    
                    # Check if regression exceeds threshold
                    if (( $(echo "$change_pct > $THRESHOLD" | bc -l) )); then
                        echo "  ⚠️  Regression detected: +${change_pct}% (exceeds ${THRESHOLD}% threshold)"
                        REGRESSION_FOUND=1
                    else
                        echo "  ✅ Performance change: ${change_pct}% (within threshold)"
                    fi
                else
                    # Fallback: simple grep check
                    if grep -q '"improved"' "target/criterion/$benchmark/change/estimates.json" 2>/dev/null; then
                        echo "  ✅ Performance improved"
                    elif grep -q '"regressed"' "target/criterion/$benchmark/change/estimates.json" 2>/dev/null; then
                        echo "  ⚠️  Potential regression detected"
                        REGRESSION_FOUND=1
                    else
                        echo "  ✅ No significant change"
                    fi
                fi
            else
                echo "  ℹ️  No comparison data available"
            fi
        fi
    done
else
    echo "⚠️  No benchmark results found in target/criterion"
    echo "Run 'cargo bench' to generate benchmark results"
fi

echo ""
echo "Performance Metrics Summary"
echo "---------------------------"

# Check for CST-specific performance metrics
if [ -f "target/criterion/property_tests/report/index.html" ]; then
    echo "✅ Property tests benchmark completed"
fi

if [ -f "target/criterion/cache_bench/report/index.html" ]; then
    echo "✅ Cache benchmark completed"
fi

# Check memory usage if available
if [ -f "target/criterion/memory_usage.json" ]; then
    echo "✅ Memory usage tracked"
fi

echo ""

if [ $REGRESSION_FOUND -eq 1 ]; then
    echo "❌ Performance regressions detected above ${THRESHOLD}% threshold"
    echo "Please review the benchmark results and optimize if needed."
    echo ""
    echo "To view detailed results, open:"
    echo "  target/criterion/report/index.html"
    exit 1
else
    echo "✅ No performance regressions detected"
    echo "All benchmarks are within the ${THRESHOLD}% threshold"
fi

exit 0
