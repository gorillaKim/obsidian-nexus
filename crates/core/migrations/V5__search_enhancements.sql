-- Document view tracking for popularity-based ranking
CREATE TABLE IF NOT EXISTS document_views (
    document_id TEXT NOT NULL,
    viewed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_views_doc ON document_views(document_id);

-- Add created_at to documents (file creation time)
ALTER TABLE documents ADD COLUMN created_at DATETIME;
