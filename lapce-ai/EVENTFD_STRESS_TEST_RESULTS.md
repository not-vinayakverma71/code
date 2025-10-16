# EventFD Stress Test - CRITICAL ISSUES IDENTIFIED

## Problem: Handlers Stuck After Reading Data

### Symptom
- Handlers read data successfully (`read=152`)
- Read position updates to 152
- But then handlers continue polling in infinite loop
- Buffer shows `read=152 write=152 avail_read=0` (correct state after consuming all data)
- Handlers don't exit or process next message properly

### Root Cause Analysis

**The handler loop is broken:**
1. Handler reads message ✅
2. Handler decodes message ✅  
3. Handler calls user handler ✅
4. Handler writes response ✅
5. **Handler loops back and tries to read AGAIN from same buffer** ❌
6. No data available (read==write), so returns Ok(0)
7. Handler continues looping forever checking for more data
8. With 1000 concurrent clients = 1000 handlers in tight poll loops = CPU exhaustion

### The Core Issue

**The server handler is designed for a PERSISTENT connection but our test creates TRANSIENT connections.**

Each stress test client:
- Connects
- Sends ONE message
- Waits for ONE response
- Should disconnect

But server handler:
- Processes ONE message
- Then loops FOREVER waiting for more messages from same client
- Never exits

This causes resource leakage - every client connection leaves a handler task running forever.

## Solution Needed

### Option 1: Fix Handler Lifecycle (RECOMMENDED)
- Handler should exit after N milliseconds of inactivity
- Or detect when client disconnects (control socket closed)
- Free resources when client is done

### Option 2: Make Test Match Design
- Clients send multiple messages per connection
- Keep connections alive
- Test sustained load vs burst load

### Option 3: Hybrid
- Support both transient and persistent connections
- Detect connection mode from handshake
- Handle accordingly

## Current Status

❌ **BLOCKED** - Cannot proceed with stress test until handler lifecycle fixed
- Single message works fine (145µs latency!)
- Multiple concurrent transient connections cause handler pile-up
- Need to fix before memory/load testing makes sense

## Next Steps

1. Add connection timeout to handlers
2. Detect client disconnect 
3. Clean up resources properly
4. Then re-run stress test
5. Then measure memory baseline
6. Then full load testing

## Performance Data (What Works)

✅ **Single Client Test:**
- Round-trip latency: **145 microseconds** (vs 11.6ms with polling!)
- EventFD integration: WORKING
- FD passing: WORKING  
- Volatile buffers: WORKING
- Message encode/decode: WORKING

**80x latency improvement from eventfd!**
