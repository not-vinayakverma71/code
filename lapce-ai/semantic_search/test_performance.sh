#!/bin/bash
# AWS credentials should be set via environment variables or AWS config
# Do NOT hardcode credentials in this file
# 
# Set credentials before running:
#   export AWS_ACCESS_KEY_ID=your_key_id
#   export AWS_SECRET_ACCESS_KEY=your_secret_key
#   export AWS_DEFAULT_REGION=us-east-1
# 
# Or use AWS CLI config: ~/.aws/credentials

if [ -z "$AWS_ACCESS_KEY_ID" ] || [ -z "$AWS_SECRET_ACCESS_KEY" ]; then
    echo "ERROR: AWS credentials not set. Please configure AWS credentials via:"
    echo "  1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)"
    echo "  2. AWS CLI config (~/.aws/credentials)"
    echo "  3. IAM role (for EC2/ECS environments)"
    exit 1
fi

export AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-us-east-1}"
timeout 90 cargo test --test final_real_performance_test -- --nocapture 2>&1 | \
  grep -E "(╔|║|╚|REAL|PHASE|Index|Cache|Query|P50|P95|✅|✨|HIT|MISS|speedup|Speedup|test result)"
