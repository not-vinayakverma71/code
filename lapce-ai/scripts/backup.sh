#!/bin/bash
# Backup script for semantic search database

set -e

BACKUP_DIR="backups/$(date +%Y%m%d_%H%M%S)"
DB_PATH="./semantic_search.db"

echo "🔒 Creating backup..."

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup database
if [ -f "$DB_PATH" ]; then
    cp "$DB_PATH" "$BACKUP_DIR/semantic_search.db"
    echo "✅ Database backed up"
else
    echo "⚠️  Database not found"
fi

# Backup configurations
if [ -f ".env" ]; then
    cp .env "$BACKUP_DIR/.env"
    echo "✅ Config backed up"
fi

# Create metadata
cat > "$BACKUP_DIR/metadata.json" << EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "version": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "size_mb": $(du -m "$DB_PATH" | cut -f1),
  "chunks": $(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM code_embeddings" 2>/dev/null || echo 0)
}
EOF

# Compress backup
tar -czf "$BACKUP_DIR.tar.gz" -C "$(dirname "$BACKUP_DIR")" "$(basename "$BACKUP_DIR")"
rm -rf "$BACKUP_DIR"

echo "✅ Backup complete: $BACKUP_DIR.tar.gz"
