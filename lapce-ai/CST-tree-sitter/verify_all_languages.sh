#!/bin/bash

echo "VERIFYING ALL 69 LANGUAGES WITH DETAILED OUTPUT"
echo "================================================"
echo ""

# Run the test and capture detailed output
../target/release/test_all_63_languages 2>&1 | tee full_test_output.txt

# Count successes and failures
SUCCESS_COUNT=$(grep -c "✅ Success" full_test_output.txt)
FAIL_COUNT=$(grep -c "❌ Failed" full_test_output.txt)

echo ""
echo "================================================"
echo "VERIFICATION RESULTS:"
echo "✅ Successful: $SUCCESS_COUNT"
echo "❌ Failed: $FAIL_COUNT"
echo ""

# List any failed languages
if [ $FAIL_COUNT -gt 0 ]; then
    echo "Failed languages:"
    grep "❌ Failed" full_test_output.txt | cut -d':' -f1 | sed 's/Testing //'
else
    echo "🎉 ALL LANGUAGES VERIFIED WORKING!"
fi

# Show parser versions for key languages
echo ""
echo "Parser Versions Check:"
for lang in javascript systemverilog elm xml cobol; do
    if [ -f "external-grammars/tree-sitter-$lang/src/parser.c" ]; then
        version=$(grep '#define LANGUAGE_VERSION' "external-grammars/tree-sitter-$lang/src/parser.c" | head -1 | awk '{print $3}')
        echo "  $lang: version $version"
    fi
done

echo ""
echo "Full output saved to: full_test_output.txt"
