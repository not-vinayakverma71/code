#!/bin/bash

# Simple script to test Gemini streaming with proper model names
# Uses curl to directly test the API

API_KEY="AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU"

echo "🔍 Testing Gemini Models for Streaming"
echo "======================================="

# Test different model names
MODELS=("gemini-2.5-flash-exp" "gemini-2.0-flash-exp" "gemini-1.5-flash" "gemini-1.5-pro" "models/gemini-pro")

for MODEL in "${MODELS[@]}"; do
    echo ""
    echo "Testing: $MODEL"
    
    RESPONSE=$(curl -s -X POST \
        "https://generativelanguage.googleapis.com/v1beta/$MODEL:generateContent?key=$API_KEY" \
        -H 'Content-Type: application/json' \
        -d '{
            "contents": [{
                "parts":[{"text":"Say OK"}]
            }],
            "generationConfig": {
                "temperature": 0.0,
                "maxOutputTokens": 5
            }
        }' 2>&1)
    
    if echo "$RESPONSE" | grep -q '"text"'; then
        echo "✅ Model $MODEL works!"
        TEXT=$(echo "$RESPONSE" | grep -o '"text":"[^"]*"' | head -1 | cut -d'"' -f4)
        echo "   Response: $TEXT"
        WORKING_MODEL="$MODEL"
        break
    elif echo "$RESPONSE" | grep -q "not found"; then
        echo "❌ Model not found"
    else
        echo "❌ Error: $(echo "$RESPONSE" | head -1)"
    fi
done

if [ -z "$WORKING_MODEL" ]; then
    echo ""
    echo "❌ No working model found!"
    exit 1
fi

echo ""
echo "======================================="
echo "✅ Using model: $WORKING_MODEL"
echo ""
echo "Now testing streaming..."
echo ""

# Test streaming with the working model
curl -X POST \
    "https://generativelanguage.googleapis.com/v1beta/$WORKING_MODEL:streamGenerateContent?alt=sse&key=$API_KEY" \
    -H 'Content-Type: application/json' \
    -d '{
        "contents": [{
            "parts":[{"text":"Generate 100 words about AI"}]
        }],
        "generationConfig": {
            "temperature": 0.7,
            "maxOutputTokens": 200
        }
    }' 2>/dev/null | while IFS= read -r line; do
    if [[ $line == data:* ]]; then
        # Extract just the data portion
        DATA="${line#data: }"
        if [ ! -z "$DATA" ]; then
            # Try to extract text from the JSON
            TEXT=$(echo "$DATA" | grep -o '"text":"[^"]*"' | cut -d'"' -f4)
            if [ ! -z "$TEXT" ]; then
                echo -n "$TEXT"
            fi
        fi
    fi
done

echo ""
echo ""
echo "======================================="
echo "✅ Streaming test complete!"
echo ""
echo "Model to use in Rust code: $WORKING_MODEL"
