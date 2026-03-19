---
name: librarian
description: Obsidian document librarian. Search, update, improve discoverability, create docs (librarian, docs, search, guide, find document, document search)
---

# Librarian

Document search and management skill using obsidian-nexus MCP tools.
**2-tier structure**: Simple searches use native MCP tools directly, complex tasks spawn subagent.

## Input

- `args`: Search query or command
- `--team`: Team member mode (persists for session)

## MCP Tool Reference

nexus MCP server is registered in `.mcp.json` — available as native MCP tools.
Tool names follow `mcp__nexus__nexus_*` pattern.

| MCP Tool | Purpose |
|----------|---------|
| `nexus_search` | Hybrid/keyword/vector search |
| `nexus_resolve_alias` | Find document by alias |
| `nexus_get_document` | Full document content |
| `nexus_get_section` | Extract specific section (token-efficient!) |
| `nexus_get_metadata` | Frontmatter, tags, indexing status |
| `nexus_get_backlinks` | Documents linking TO this document |
| `nexus_get_links` | Documents this document links TO |
| `nexus_list_projects` | List registered vaults |
| `nexus_list_documents` | List documents (optional tag filter) |
| `nexus_index_project` | Trigger indexing |
| `nexus_help` | Tool help |

## Execution

### Step 1: Direct Search (no subagent needed)

Main agent calls native MCP tools directly:

```
nexus_search(query="query", mode="hybrid", limit=5)
```

If results found:
- Use `nexus_get_section` to extract relevant section
- Deliver to user directly → **Done**

### Step 2: Spawn Subagent (on search failure or document management)

Spawn librarian subagent only for:

**Search failure:**
```
Agent(subagent_type: "librarian", model: "haiku",
  prompt: "Search query: '{query}'. Direct search returned no results. Start from Phase A.",
  description: "librarian deep search")
```

**Document update request:**
```
Agent(subagent_type: "librarian", model: "haiku",
  prompt: "Document update request: '{path}'. Execute Phase C.",
  description: "librarian doc update")
```

**Document creation request:**
```
Agent(subagent_type: "librarian", model: "haiku",
  prompt: "Document creation request: '{topic}'. Execute Phase D. Follow the frontmatter guide in your agent definition. CWD: {cwd}",
  description: "librarian doc creation")
```

## Rules

- Simple searches use native MCP tool calls (no CLI bash)
- Subagent only for search failure, discoverability improvement, updates, creation
- Document modifications require user approval
- Tag standards defined in agent definition (`librarian.md`)
