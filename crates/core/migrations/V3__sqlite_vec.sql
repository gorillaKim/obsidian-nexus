-- sqlite-vec virtual table for vector KNN search (768 dimensions, nomic-embed-text)
-- Note: vec0 table creation is handled in Rust code after sqlite-vec extension is loaded,
-- because the extension must be loaded before CREATE VIRTUAL TABLE can reference vec0.
-- This migration only drops the old chunk_embeddings data dependency.

-- We keep chunk_embeddings table for backward compatibility during migration.
-- New embeddings will be stored in vec_chunks (created in Rust after extension load).
