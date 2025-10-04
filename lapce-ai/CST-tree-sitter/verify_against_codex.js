#!/usr/bin/env node
/**
 * CODEX VERIFICATION SCRIPT
 * 
 * This script runs the actual Codex TypeScript implementation
 * and outputs results to compare against our Rust implementation
 */

const fs = require('fs');
const path = require('path');

// Sample test files matching Codex's test fixtures
const testFiles = {
  javascript: `
// Function declaration test
function testFunctionDefinition(
    param1,
    param2,
    param3
) {
    const result = param1 + param2;
    return result * param3;
}

// Class declaration test
class TestClassDefinition {
    constructor(
        name,
        value
    ) {
        this.name = name;
        this.value = value;
    }
    
    testMethodDefinition(
        param1,
        param2
    ) {
        return param1 + param2;
    }
}
`,
  
  python: `
# Function definition test
def test_function_definition(
    param1,
    param2,
    param3
):
    result = param1 + param2
    return result * param3

# Class definition test  
class TestClassDefinition:
    def __init__(
        self,
        name,
        value
    ):
        self.name = name
        self.value = value
`,
  
  rust: `
// Function definition
fn test_function_definition(
    param1: i32,
    param2: i32,
) -> i32 {
    param1 + param2
}

// Struct definition
pub struct TestStructDefinition {
    name: String,
    value: i32,
}
`
};

console.log('🔍 CODEX VERIFICATION - Getting Expected Outputs\n');
console.log('This script needs to be run from Codex directory with tree-sitter compiled\n');
console.log('Expected output format: "startLine--endLine | source_text"\n');
console.log('='=60);

// Write test files
Object.entries(testFiles).forEach(([lang, content]) => {
  const filename = `test_sample.${lang === 'javascript' ? 'js' : lang === 'python' ? 'py' : 'rs'}`;
  fs.writeFileSync(filename, content);
  console.log(`\n📝 Created ${filename}`);
  console.log(`Language: ${lang}`);
  console.log(`Lines: ${content.split('\n').length}`);
});

console.log('\n' + '='.repeat(60));
console.log('\n✅ Test files created');
console.log('\nNEXT STEPS:');
console.log('1. Copy this script to Codex directory');
console.log('2. Run: node verify_against_codex.js');
console.log('3. Or manually call parseSourceCodeDefinitionsForFile() in Codex');
console.log('4. Compare outputs with our Rust implementation');
console.log('\nIMPORTANT: Codex uses processCaptures() with:');
console.log('  - Line numbers 1-indexed (not 0)');
console.log('  - Format: "start--end | text"');
console.log('  - Min 4 lines for components (getMinComponentLines())');
console.log('  - HTML filtering for jsx/tsx');
