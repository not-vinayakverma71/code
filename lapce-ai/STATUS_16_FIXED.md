# Test Fixing Status

**Time**: ~7.5 hours  
**Progress**: 16/110 fixed (85% → 94 remaining)  
**Success rate**: 15% so far  

## Fixed Tests (16)

### XML Parsing Pattern Fixes:
1-4. **expanded_tools_registry** (4 tests) - Tool name assertions  
5. **read_file_v2** - maxSize string parsing  
6-8. **insert_content** (3 tests) - XML content trimming  
9. **write_file_v2** - maxSize parsing  
10. **write_file_v2** - backupIfExists parsing  
11-14. **search_and_replace_v2** (4 tests) - Boolean/numeric parsing  
15-16. **search_and_replace** (2 tests) - multiline, preview, empty replace  

## Remaining: 94 failures

**Categories**:
- Symlink tests (5-8) - Platform-specific, skip
- IPC/adapter tests (10+) - Mock setup required, skip
- Streaming pipeline (6) - Complex logic
- Task orchestration (3) - Async integration
- Other tools (50+) - More XML parsing needed

## Next Actions

**Quick wins** (estimate 20 more tests):
- execute_command.rs - dangerous flag
- terminal tools - timeout values  
- observability tests - metrics
- Other fs tools - boolean flags

**Skip** (30+ tests):
- Symlink tests - platform-specific
- IPC adapter tests - complex mocking
- Tests with logic bugs vs parsing bugs

**Realistic target**: 35-40 fixed total (32-36% of 110)
