-- Initial schema for semantic search
CREATE TABLE IF NOT EXISTS code_embeddings (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    content TEXT NOT NULL,
    language TEXT,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    vector BLOB NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_embeddings_path ON code_embeddings(path);
CREATE INDEX idx_embeddings_language ON code_embeddings(language);
CREATE INDEX idx_embeddings_created ON code_embeddings(created_at);

CREATE TABLE IF NOT EXISTS search_history (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    results_count INTEGER,
    latency_ms REAL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS index_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    files_indexed INTEGER,
    chunks_created INTEGER,
    index_duration_ms REAL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
