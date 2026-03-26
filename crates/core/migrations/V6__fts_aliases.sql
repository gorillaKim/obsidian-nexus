-- Add aliases column to chunks table (Option A: first chunk stores aliases, rest NULL)
ALTER TABLE chunks ADD COLUMN aliases TEXT;

-- Rebuild chunks_fts with aliases column
-- FTS5 does not support ALTER TABLE ADD COLUMN, so DROP & CREATE is required.
DROP TABLE IF EXISTS chunks_fts;

CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
    content,
    heading_path,
    aliases,
    content=chunks,
    content_rowid=rowid,
    tokenize='unicode61'
);

-- Recreate triggers to include aliases column
DROP TRIGGER IF EXISTS chunks_ai;
DROP TRIGGER IF EXISTS chunks_ad;
DROP TRIGGER IF EXISTS chunks_au;

CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
    INSERT INTO chunks_fts(rowid, content, heading_path, aliases)
    VALUES (new.rowid, new.content, new.heading_path, new.aliases);
END;

CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, content, heading_path, aliases)
    VALUES ('delete', old.rowid, old.content, old.heading_path, old.aliases);
END;

CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, content, heading_path, aliases)
    VALUES ('delete', old.rowid, old.content, old.heading_path, old.aliases);
    INSERT INTO chunks_fts(rowid, content, heading_path, aliases)
    VALUES (new.rowid, new.content, new.heading_path, new.aliases);
END;

-- Repopulate FTS index from existing chunks (aliases will be NULL until re-indexed, but content/heading_path remain searchable)
INSERT INTO chunks_fts(chunks_fts) VALUES('rebuild');
