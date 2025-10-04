#!/bin/bash
set -e

# Validate required environment variables
if [ -z "$AWS_ACCESS_KEY_ID" ]; then
    echo "Error: AWS_ACCESS_KEY_ID not set"
    exit 1
fi

if [ -z "$AWS_SECRET_ACCESS_KEY" ]; then
    echo "Error: AWS_SECRET_ACCESS_KEY not set"
    exit 1
fi

# Create data directory if needed
mkdir -p "$LANCE_DB_PATH"

# Run the service
case "$1" in
    serve)
        echo "Starting Titan Semantic Search API on port $PORT..."
        exec /usr/local/bin/lance_search serve \
            --port "$PORT" \
            --db-path "$LANCE_DB_PATH"
        ;;
    index)
        echo "Indexing files..."
        exec /usr/local/bin/lance_search index \
            --db-path "$LANCE_DB_PATH" \
            --path "$2"
        ;;
    search)
        echo "Searching..."
        exec /usr/local/bin/lance_search search \
            --db-path "$LANCE_DB_PATH" \
            --query "$2"
        ;;
    *)
        echo "Usage: $0 {serve|index|search}"
        exit 1
        ;;
esac
