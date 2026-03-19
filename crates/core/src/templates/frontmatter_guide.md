# Obsidian Nexus Frontmatter Guide

This file defines the frontmatter conventions for documents managed by Obsidian Nexus.
The librarian subagent MUST follow these rules when creating or improving documents.

## Required Frontmatter Template

```yaml
---
title: Document Title
aliases:
  - english-alias
  - korean-alias (한글 별칭)
tags:
  - document-type-tag    # REQUIRED: at least one (guide, spec, overview, etc.)
  - domain-tag           # technical domain
  - tech-stack-tag       # specific technology (optional)
created: YYYY-MM-DD
updated: YYYY-MM-DD
---
```

## Field Rules

### title (required)
- Clear, descriptive title
- Used by Nexus for title-matching boost in search ranking

### aliases (recommended)
- English aliases: lowercase, hyphen-separated (e.g. `datadog-setup`)
- Korean aliases: natural language form (e.g. `데이터독 셋업`)
- Include abbreviations, synonyms, and common search terms
- Aliases make documents discoverable via `nexus_resolve_alias`
- **Key rule**: If a user searched for something and couldn't find the document, add that search term as an alias

### tags (required, max 5)
- Lowercase English, hyphens for compound words
- At least 1 document type tag required
- Reuse existing tags before creating new ones
- No Korean tags (use aliases for Korean terms)
- No synonym tags (e.g. don't have both `config` and `configuration`)

#### Standard Tag Categories

| Category | Tags | Purpose |
|----------|------|---------|
| Document type | `overview`, `guide`, `spec`, `tutorial`, `reference`, `troubleshooting`, `devlog` | Nature of document |
| Domain | `architecture`, `database`, `search`, `mcp`, `api`, `config`, `ui`, `logging`, `auth`, `deploy` | Technical domain |
| Tech stack | `sqlite`, `sqlite-vec`, `fts5`, `vector`, `rust`, `tauri`, `react`, `datadog`, `ollama` | Specific technology |
| Activity | `development`, `test`, `benchmark`, `evaluation`, `setup`, `design`, `migration` | Type of work |
| Audience | `agent`, `user`, `admin` | Primary consumer |

### created (required)
- ISO date format: `YYYY-MM-DD`
- Set once at document creation, never change

### updated (recommended)
- ISO date format: `YYYY-MM-DD`
- Update whenever document content is modified
- Used by librarian Phase C to detect stale documents

## Examples

### Technical Guide
```yaml
---
title: Datadog Browser Logs Setup Guide
aliases:
  - datadog-setup
  - datadog-logger
  - 데이터독 셋업
tags:
  - guide
  - logging
  - datadog
  - setup
created: 2026-03-19
updated: 2026-03-19
---
```

### Architecture Document
```yaml
---
title: Search System Architecture
aliases:
  - search-architecture
  - 검색 시스템 구조
tags:
  - architecture
  - search
  - spec
created: 2025-12-01
updated: 2026-03-15
---
```

### Troubleshooting Document
```yaml
---
title: SQLite WAL Mode Locking Issues
aliases:
  - sqlite-wal-lock
  - WAL 잠금 문제
tags:
  - troubleshooting
  - sqlite
  - database
created: 2026-02-10
updated: 2026-02-10
---
```

## Document Body Structure

After frontmatter, follow this structure:

```markdown
# {title}

Brief description or summary paragraph.

## Section 1

Content...

## Section 2

Content...

## Related Documents

- [[Related Document 1]]
- [[Related Document 2]]
```

- Use `#` heading matching the title
- Use `##` for major sections (these are extractable via `nexus_get_section`)
- End with "Related Documents" section using wiki-links (`[[...]]`) for Obsidian graph connectivity
- Wiki-links enable Nexus backlink tracking via `nexus_get_backlinks` / `nexus_get_links`
