#!/bin/bash

echo "DETAILED VERIFICATION OF ALL 69 LANGUAGES"
echo "=========================================="
echo ""

# List of all 69 languages to test
LANGUAGES=(
    "rust" "javascript" "typescript" "python" "go" "java" "c" "cpp" "csharp"
    "ruby" "php" "swift" "kotlin" "scala" "elixir" "lua" "bash" "css"
    "json" "html" "yaml" "markdown" "r" "matlab" "perl" "dart" "julia"
    "haskell" "graphql" "sql" "zig" "vim" "ocaml" "nix" "make" "cmake"
    "verilog" "erlang" "d" "pascal" "objc" "groovy" "solidity" "fsharp"
    "systemverilog" "elm" "tsx" "jsx" "cobol" "commonlisp" "hcl" "xml"
    "clojure" "nim" "crystal" "fortran" "vhdl" "racket" "ada" "svelte"
    "abap" "scheme" "fennel" "gleam" "astro" "wgsl" "glsl" "tcl" "cairo"
)

echo "Total languages to verify: ${#LANGUAGES[@]}"
echo ""

# Run the test and parse output
../target/release/test_all_63_languages 2>&1 > latest_test.txt

# Count results
echo "Test Results Summary:"
echo "====================="

SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_LANGS=()

for lang in "${LANGUAGES[@]}"; do
    if grep -q "Testing $lang:.*✅ Success" latest_test.txt; then
        echo "✅ $lang: WORKING"
        ((SUCCESS_COUNT++))
    else
        echo "❌ $lang: FAILED or NOT FOUND"
        ((FAIL_COUNT++))
        FAILED_LANGS+=("$lang")
    fi
done

echo ""
echo "=========================================="
echo "FINAL VERIFICATION:"
echo "✅ Working: $SUCCESS_COUNT / ${#LANGUAGES[@]}"
echo "❌ Failed: $FAIL_COUNT / ${#LANGUAGES[@]}"
echo "Success Rate: $(( SUCCESS_COUNT * 100 / ${#LANGUAGES[@]} ))%"
echo ""

if [ ${#FAILED_LANGS[@]} -gt 0 ]; then
    echo "Failed languages:"
    for lang in "${FAILED_LANGS[@]}"; do
        echo "  - $lang"
    done
else
    echo "🎉 ALL 69 LANGUAGES VERIFIED AS WORKING!"
fi
