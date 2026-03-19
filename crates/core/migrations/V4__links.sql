-- Wiki links between documents
CREATE TABLE IF NOT EXISTS wiki_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_doc_id TEXT NOT NULL,
    target_path TEXT NOT NULL,
    display_text TEXT,
    target_doc_id TEXT,
    FOREIGN KEY(source_doc_id) REFERENCES documents(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_wl_source ON wiki_links(source_doc_id);
CREATE INDEX IF NOT EXISTS idx_wl_target ON wiki_links(target_path);
CREATE INDEX IF NOT EXISTS idx_wl_target_doc ON wiki_links(target_doc_id);

-- Document aliases (from frontmatter)
CREATE TABLE IF NOT EXISTS document_aliases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id TEXT NOT NULL,
    alias TEXT NOT NULL,
    FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_aliases_doc ON document_aliases(document_id);
CREATE INDEX IF NOT EXISTS idx_aliases_name ON document_aliases(alias);
