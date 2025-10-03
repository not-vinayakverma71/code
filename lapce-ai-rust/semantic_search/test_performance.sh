#!/bin/bash
export AWS_ACCESS_KEY_ID="AKIA2RCKMSFVZ72HLCXD"
export AWS_SECRET_ACCESS_KEY="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"
export AWS_DEFAULT_REGION="us-east-1"
timeout 90 cargo test --test final_real_performance_test -- --nocapture 2>&1 | \
  grep -E "(╔|║|╚|REAL|PHASE|Index|Cache|Query|P50|P95|✅|✨|HIT|MISS|speedup|Speedup|test result)"
