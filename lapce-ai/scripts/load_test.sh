#!/bin/bash
# Load testing script using vegeta

set -e

API_URL="http://localhost:8080"
DURATION="30s"
RATE="100"

echo "🚀 Starting load test..."
echo "   Target: $API_URL"
echo "   Duration: $DURATION"
echo "   Rate: $RATE req/s"

# Install vegeta if not present
if ! command -v vegeta &> /dev/null; then
    echo "Installing vegeta..."
    go install github.com/tsenart/vegeta@latest
fi

# Create test targets
cat > targets.txt << EOF
POST ${API_URL}/api/v1/search
Content-Type: application/json
@search_payload.json

GET ${API_URL}/health

GET ${API_URL}/api/v1/stats
EOF

# Create search payload
cat > search_payload.json << EOF
{
  "query": "function test async",
  "limit": 10
}
EOF

# Run load test
echo -e "\n📊 Running load test..."
vegeta attack -targets=targets.txt -rate=$RATE -duration=$DURATION | \
  vegeta report -type=text

# Cleanup
rm targets.txt search_payload.json

echo "✅ Load test complete!"
