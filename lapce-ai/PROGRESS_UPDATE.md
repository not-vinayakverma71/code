# Test Fixing Progress

**Time**: ~6 hours  
**Starting**: 110 failures  
**Current**: 103 failures  
**Fixed**: 7 tests  

## Completed Fixes

1. ✅ **Registry tests** (4 tests)
   - Fixed tool name assertions (camelCase vs snake_case)
   - Fixed tool count (expected 19, not 20)
   - Fixed permissions field access

2. ✅ **read_file_v2 size limit** (1 test)
   - Fixed maxSize parsing to handle string values from XML

3. ✅ **insert_content start** (1 test)
   - Added newline handling for start position

## Current Focus

**insert_content tests** (2 remaining):
- test_insert_at_line - Content not separated by newline
- test_insert_at_end - Similar newline issue

## Failure Breakdown (103 total)

| Category | Count | Priority |
|----------|-------|----------|
| core::tools | 38 | High |
| streaming_pipeline::sse_parser | 6 | High |
| mcp_tools | 10 | Medium |
| ipc::config_validation | 5 | Medium |
| task_orchestration_loop | 3 | High |
| Other | 41 | Medium |

## Strategy

Fixing core::tools tests systematically - many likely have similar root causes (XML parsing, string vs typed values, newline handling).

Next: Fix remaining insert_content tests, then batch-fix similar core::tools failures.
