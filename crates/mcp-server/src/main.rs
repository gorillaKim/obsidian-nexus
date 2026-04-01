use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
const NEXUS_HELP_TEXT: &str = r#"# Obsidian Nexus MCP — Available Tools

## Search & Discovery
- **nexus_search** — Hybrid/keyword/vector search across vaults (supports tags filter, popularity boost)
- **nexus_resolve_alias** — Find a document by its alias

## Document Access
- **nexus_get_document** — Get full document content by path
- **nexus_get_section** — Get a specific section by heading (token-efficient!)
- **nexus_get_sections** — Get multiple sections from a document in one call (returns success/errors map)
- **nexus_get_metadata** — Get frontmatter, tags, indexing status

## Graph Navigation
- **nexus_get_backlinks** — Documents linking TO this document
- **nexus_get_links** — Documents this document links TO
- **nexus_get_cluster** — All documents within N hops (forward + backward). Replaces multiple get_links/get_backlinks calls.
- **nexus_find_path** — Shortest forward-link path between two documents
- **nexus_find_related** — Related documents via RRF (link + tag + vector signals)

## Project Management
- **nexus_list_projects** — List all registered vaults
- **nexus_list_documents** — List documents in a project (optional tag filter)
- **nexus_index_project** — Trigger incremental or full reindex
- **nexus_sync_config** — Sync project name from on-config.json
- **nexus_status** — Check system health (Ollama, DB, config)

## Setup
- **nexus_onboard** — Set up librarian skill & subagent in any project

## Recommended Workflow
1. `nexus_onboard` → set up librarian in your project
2. `nexus_search` → find relevant documents
3. `nexus_get_section` → read only the section you need (saves tokens!)
4. `nexus_get_backlinks` → discover related documents via graph
5. `nexus_get_metadata` → check tags and popularity
"#;

fn main() -> Result<()> {
    // Initialize database
    let pool = nexus_core::db::sqlite::create_pool()?;
    nexus_core::db::sqlite::run_migrations(&pool)?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let response = match method {
            "initialize" => handle_initialize(&id),
            "tools/list" => handle_tools_list(&id),
            "tools/call" => handle_tools_call(&id, &params, &pool),
            "notifications/initialized" => continue, // No response needed
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            }),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_initialize(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "nexus-mcp-server",
                "version": "0.1.0"
            }
        }
    })
}

fn handle_tools_list(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "nexus_search",
                    "description": "Search across indexed Obsidian documents (hybrid/keyword/vector modes with metadata reranking). query is optional — omit it to use filter-only mode (date + tag).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query (optional — omit for filter-only mode using date/tag)" },
                            "project": { "type": "string", "description": "Project name or ID (optional, searches all if omitted)" },
                            "limit": { "type": "integer", "description": "Max results (default: 20)", "default": 20 },
                            "offset": { "type": "integer", "description": "Pagination offset (default: 0). Use offset += limit to get the next page.", "default": 0 },
                            "mode": { "type": "string", "description": "Search mode: hybrid (default), keyword, vector", "default": "hybrid" },
                            "enrich": { "type": "boolean", "description": "Include metadata (tags, backlink_count, view_count, last_modified) in results (default: true)", "default": true },
                            "use_popularity": { "type": "boolean", "description": "Boost results by popularity. Default: true if project specified, false otherwise" },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "Filter results by tags (optional)" },
                            "tag_match_all": { "type": "boolean", "description": "If true, require ALL tags to match (AND). Default false (OR).", "default": false },
                            "date_from": { "type": "string", "description": "Filter: date range start (ISO 8601, e.g. '2025-01-01' or '2025-01-01T00:00:00')" },
                            "date_to": { "type": "string", "description": "Filter: date range end inclusive (ISO 8601, e.g. '2025-12-31')" },
                            "date_field": { "type": "string", "description": "Date field to filter on: last_modified (default) | created_at", "default": "last_modified" },
                            "sort_by": { "type": "string", "description": "Sort order: relevance (default) | date_desc | date_asc. Use date_desc when date filter is active.", "default": "relevance" },
                            "rewrite_query": { "type": "boolean", "description": "LLM으로 쿼리를 재작성하여 도메인 용어 매칭 향상 (Ollama 필요, config.llm.enabled 또는 이 파라미터로 활성화)", "default": false }
                        },
                        "required": []
                    }
                },
                {
                    "name": "nexus_get_documents",
                    "description": "Read multiple documents (up to 5) in one call. Use file_path values from nexus_search results directly.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "paths": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "List of document paths (max 5). Accepts file_path from search results."
                            },
                            "project": { "type": "string", "description": "Project name or ID (required when paths are relative)" }
                        },
                        "required": ["paths"]
                    }
                },
                {
                    "name": "nexus_list_projects",
                    "description": "List all registered Obsidian vault projects",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "nexus_get_document",
                    "description": "Get the full content of a document by project and file path",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" }
                        },
                        "required": ["project", "path"]
                    }
                },
                {
                    "name": "nexus_get_metadata",
                    "description": "Get document metadata (frontmatter, tags, indexing status)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" }
                        },
                        "required": ["project", "path"]
                    }
                },
                {
                    "name": "nexus_list_documents",
                    "description": "List all documents in a project, optionally filtered by tag",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "tag": { "type": "string", "description": "Filter by tag (optional)" }
                        },
                        "required": ["project"]
                    }
                },
                {
                    "name": "nexus_index_project",
                    "description": "Trigger indexing for a project (incremental by default)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "full": { "type": "boolean", "description": "Full re-index (default: false)", "default": false }
                        },
                        "required": ["project"]
                    }
                },
                {
                    "name": "nexus_get_section",
                    "description": "Get a specific section of a document by heading name",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" },
                            "heading": { "type": "string", "description": "Heading text to extract (e.g. 'Introduction')" },
                            "heading_path": { "type": "string", "description": "Full heading path from TOC (e.g. 'Parent > Child'), used to disambiguate duplicate headings" }
                        },
                        "required": ["project", "path", "heading"]
                    }
                },
                {
                    "name": "nexus_get_sections",
                    "description": "Get multiple sections from a document in one call. Returns {success: {heading: content}, errors: {heading: error}}.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" },
                            "headings": {
                                "type": "array",
                                "description": "List of sections to extract (max 20)",
                                "maxItems": 20,
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "heading": { "type": "string", "description": "Heading text to extract" },
                                        "heading_path": { "type": "string", "description": "Full heading path from TOC (e.g. 'Parent > Child') to disambiguate duplicate headings" }
                                    },
                                    "required": ["heading"]
                                }
                            }
                        },
                        "required": ["project", "path", "headings"]
                    }
                },
                {
                    "name": "nexus_get_toc",
                    "description": "Return the table of contents for a document. Each entry contains heading, level, and heading_path. Use heading_path when calling nexus_get_section to disambiguate duplicate headings.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" }
                        },
                        "required": ["project", "path"]
                    }
                },
                {
                    "name": "nexus_sync_config",
                    "description": "Sync project name from on-config.json in vault root. Call after editing on-config.json.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID to sync" }
                        },
                        "required": ["project"]
                    }
                },
                {
                    "name": "nexus_get_backlinks",
                    "description": "Get documents that link to this document (backlinks)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" }
                        },
                        "required": ["project", "path"]
                    }
                },
                {
                    "name": "nexus_get_links",
                    "description": "Get documents that this document links to (forward links)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" }
                        },
                        "required": ["project", "path"]
                    }
                },
                {
                    "name": "nexus_get_cluster",
                    "description": "Get all documents reachable from a document within N hops (forward + backlinks). Returns nodes with distance, tags, and snippet. Replaces multiple get_backlinks/get_links calls.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" },
                            "depth": { "type": "integer", "description": "Traversal depth (default: 2, max: 5)", "default": 2 }
                        },
                        "required": ["project", "path"]
                    }
                },
                {
                    "name": "nexus_find_path",
                    "description": "Find the shortest forward-link path between two documents (max 6 hops). Returns null if no path exists.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "from": { "type": "string", "description": "Source file path" },
                            "to": { "type": "string", "description": "Target file path" }
                        },
                        "required": ["project", "from", "to"]
                    }
                },
                {
                    "name": "nexus_find_related",
                    "description": "Find related documents using RRF over link-distance and tag overlap. Returns scored results with signals indicating why each document was included.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "path": { "type": "string", "description": "File path relative to vault root" },
                            "k": { "type": "integer", "description": "Max results (default: 10)", "default": 10 }
                        },
                        "required": ["project", "path"]
                    }
                },
                {
                    "name": "nexus_resolve_alias",
                    "description": "Find a document by its alias",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project name or ID" },
                            "alias": { "type": "string", "description": "Alias to look up" }
                        },
                        "required": ["project", "alias"]
                    }
                },
                {
                    "name": "nexus_status",
                    "description": "Check system health: Ollama server, embedding model, database, and config status",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "nexus_help",
                    "description": "Show available Obsidian Nexus MCP tools and recommended workflows",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "nexus_get_ranking",
                    "description": "Get popular document rankings by view_count and backlink_count. Use for 'what are the most viewed/linked docs?' queries. Supports global or per-project ranking.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string", "description": "Project ID or name (omit for global ranking)" },
                            "limit": { "type": "integer", "description": "Max results (default: 10)", "default": 10 }
                        }
                    }
                },
                {
                    "name": "nexus_onboard",
                    "description": "Set up librarian skill and subagent in a project. Creates .mcp.json, .claude/agents/librarian.md, and .claude/skills/librarian/SKILL.md",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": { "type": "string", "description": "Target project root path (default: current working directory)" },
                            "force": { "type": "boolean", "description": "Overwrite existing files (default: false)", "default": false }
                        }
                    }
                }
            ]
        }
    })
}

fn handle_tools_call(id: &Value, params: &Value, pool: &nexus_core::db::sqlite::DbPool) -> Value {
    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match tool_name {
        "nexus_search" => tool_search(&args, pool),
        "nexus_list_projects" => tool_list_projects(pool),
        "nexus_get_document" => tool_get_document(&args, pool),
        "nexus_get_documents" => tool_get_documents(&args, pool),
        "nexus_get_metadata" => tool_get_metadata(&args, pool),
        "nexus_list_documents" => tool_list_documents(&args, pool),
        "nexus_index_project" => tool_index_project(&args, pool),
        "nexus_sync_config" => tool_sync_config(&args, pool),
        "nexus_get_section" => tool_get_section(&args, pool),
        "nexus_get_sections" => tool_get_sections(&args, pool),
        "nexus_get_toc" => tool_get_toc(&args, pool),
        "nexus_get_backlinks" => tool_get_backlinks(&args, pool),
        "nexus_get_links" => tool_get_links(&args, pool),
        "nexus_get_cluster" => tool_get_cluster(&args, pool),
        "nexus_find_path" => tool_find_path(&args, pool),
        "nexus_find_related" => tool_find_related(&args, pool),
        "nexus_resolve_alias" => tool_resolve_alias(&args, pool),
        "nexus_status" => Ok(nexus_core::status::get_status(pool)),
        "nexus_help" => Ok(NEXUS_HELP_TEXT.to_string()),
        "nexus_get_ranking" => tool_get_ranking(&args, pool),
        "nexus_onboard" => tool_onboard(&args),
        _ => Err(format!("Unknown tool: {}", tool_name)),
    };

    match result {
        Ok(content) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": content }]
            }
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                "isError": true
            }
        }),
    }
}

fn tool_search(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let query = args.get("query").and_then(|q| q.as_str());
    let project = args.get("project").and_then(|p| p.as_str());
    let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(20) as usize;
    let offset = args.get("offset").and_then(|o| o.as_u64()).unwrap_or(0) as usize;
    let mode = args.get("mode").and_then(|m| m.as_str()).unwrap_or("hybrid");
    let enrich = args.get("enrich").and_then(|e| e.as_bool()).unwrap_or(true);
    let use_popularity = args.get("use_popularity").and_then(|u| u.as_bool())
        .unwrap_or(project.is_some());

    let resolved_pid = if let Some(p) = project {
        let proj = nexus_core::project::get_project(pool, p).map_err(|e| e.to_string())?;
        Some(proj.id)
    } else {
        None
    };

    // 태그 필터
    let tag_strings: Vec<String> = args.get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let match_all = args.get("tag_match_all").and_then(|v| v.as_bool()).unwrap_or(false);
    let tag_filter = if tag_strings.is_empty() {
        None
    } else {
        Some(nexus_core::search::TagFilter::new(tag_strings, match_all))
    };

    // 날짜 필터
    let date_from = args.get("date_from").and_then(|v| v.as_str()).map(str::to_string);
    let date_to = args.get("date_to").and_then(|v| v.as_str()).map(str::to_string);
    let date_field = match args.get("date_field").and_then(|v| v.as_str()).unwrap_or("last_modified") {
        "created_at" => nexus_core::search::DateField::CreatedAt,
        _ => nexus_core::search::DateField::LastModified,
    };
    let date_filter = if date_from.is_some() || date_to.is_some() {
        Some(nexus_core::search::DateFilter { date_from, date_to, field: date_field })
    } else {
        None
    };

    // sort_by
    let sort_by = match args.get("sort_by").and_then(|v| v.as_str()).unwrap_or("relevance") {
        "date_desc" => nexus_core::search::SortBy::DateDesc,
        "date_asc" => nexus_core::search::SortBy::DateAsc,
        _ => nexus_core::search::SortBy::Relevance,
    };

    // date filter 사용 시 last_modified 자동 포함을 위해 enrich 강제
    let force_enrich_lm = date_filter.is_some();

    let mut config = nexus_core::Config::load().unwrap_or_default();
    let rewrite_query_param = args.get("rewrite_query").and_then(|v| v.as_bool()).unwrap_or(false);
    if rewrite_query_param {
        config.llm.enabled = true;
    }

    let mut results = if let Some(q) = query {
        // query 있음 — 일반 검색
        let effective_query: String = if config.llm.enabled {
            nexus_core::llm::rewrite_query(&config, q).unwrap_or_else(|_| q.to_string())
        } else {
            q.to_string()
        };
        let search_query = effective_query.as_str();

        match mode {
            "keyword" => nexus_core::search::fts_search(pool, search_query, resolved_pid.as_deref(), limit, offset, tag_filter.as_ref(), date_filter.as_ref())
                .map_err(|e| e.to_string())?,
            "vector" => nexus_core::search::vector_search(pool, search_query, resolved_pid.as_deref(), limit, offset, &config, tag_filter.as_ref(), date_filter.as_ref())
                .map_err(|e| e.to_string())?,
            _ => nexus_core::search::hybrid_search(pool, search_query, resolved_pid.as_deref(), limit, offset, &config, tag_filter.as_ref(), date_filter.as_ref())
                .map_err(|e| e.to_string())?,
        }
    } else {
        // filter-only 모드: date + tag만으로 검색
        nexus_core::search::filter_search(pool, resolved_pid.as_deref(), limit, offset, tag_filter.as_ref(), date_filter.as_ref(), sort_by)
            .map_err(|e| e.to_string())?
    };

    if enrich || force_enrich_lm {
        nexus_core::search::enrich_results(pool, &mut results, use_popularity)
            .map_err(|e| e.to_string())?;
    }

    // has_more: limit+1개 조회해서 판단하는 대신, limit개 반환 시 has_more=true로 추정
    let has_more = results.len() == limit;
    let response = serde_json::json!({
        "results": results,
        "meta": {
            "total_returned": results.len(),
            "offset": offset,
            "has_more": has_more
        }
    });
    serde_json::to_string_pretty(&response).map_err(|e| e.to_string())
}

fn tool_get_documents(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let paths: Vec<&str> = args.get("paths")
        .and_then(|p| p.as_array())
        .ok_or("Missing 'paths'")?
        .iter()
        .filter_map(|v| v.as_str())
        .take(5)
        .collect();

    if paths.is_empty() {
        return Err("'paths' must be a non-empty array".to_string());
    }

    let project = args.get("project").and_then(|p| p.as_str());

    let mut success: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut errors: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for path in paths {
        // path 형식: "project_name/relative/path.md" 또는 절대경로
        let (proj_id, file_path) = if let Some(p) = project {
            // project 파라미터 제공됨 — path를 그대로 file_path로 사용
            match nexus_core::project::get_project(pool, p) {
                Ok(proj) => (proj.id, path.to_string()),
                Err(e) => { errors.insert(path.to_string(), e.to_string()); continue; }
            }
        } else {
            // project 없음 — path에서 첫 세그먼트를 project 이름으로 해석
            let parts: Vec<&str> = path.splitn(2, '/').collect();
            if parts.len() < 2 {
                errors.insert(path.to_string(), "Cannot resolve project: provide 'project' param or use 'project/path' format".to_string());
                continue;
            }
            match nexus_core::project::get_project(pool, parts[0]) {
                Ok(proj) => (proj.id, parts[1].to_string()),
                Err(e) => { errors.insert(path.to_string(), e.to_string()); continue; }
            }
        };

        match nexus_core::search::get_document_content(pool, &proj_id, &file_path) {
            Ok(content) => { success.insert(path.to_string(), content); }
            Err(e) => { errors.insert(path.to_string(), e.to_string()); }
        }
    }

    let response = serde_json::json!({ "success": success, "errors": errors });
    serde_json::to_string_pretty(&response).map_err(|e| e.to_string())
}

fn tool_list_projects(pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let projects = nexus_core::project::list_projects(pool).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&projects).map_err(|e| e.to_string())
}

fn tool_get_document(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;

    // Record view for popularity tracking
    if let Ok(meta) = nexus_core::search::get_document_meta(pool, &proj.id, path) {
        let _ = nexus_core::search::record_view(pool, &meta.id);
    }

    nexus_core::search::get_document_content(pool, &proj.id, path).map_err(|e| e.to_string())
}

fn tool_sync_config(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let updated = nexus_core::project::sync_vault_config(pool, project).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())
}

fn tool_get_section(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let heading = args.get("heading").and_then(|h| h.as_str()).ok_or("Missing 'heading'")?;
    let heading_path = args.get("heading_path").and_then(|h| h.as_str());
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    nexus_core::search::get_section(pool, &proj.id, path, heading, heading_path).map_err(|e| e.to_string())
}

fn tool_get_sections(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|v| v.as_str()).ok_or("Missing 'project'")?;
    let path    = args.get("path").and_then(|v| v.as_str()).ok_or("Missing 'path'")?;
    let raw = args.get("headings").and_then(|v| v.as_array()).ok_or("Missing 'headings'")?;
    let requests: Vec<nexus_core::search::SectionRequest> = raw.iter()
        .filter_map(|v| v.get("heading").and_then(|h| h.as_str()).map(|h| nexus_core::search::SectionRequest {
            heading: h,
            heading_path: v.get("heading_path").and_then(|p| p.as_str()),
        }))
        .collect();
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let (success, errors) = nexus_core::search::get_sections(pool, &proj.id, path, &requests)
        .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&serde_json::json!({ "success": success, "errors": errors }))
        .map_err(|e| e.to_string())
}

fn tool_get_toc(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let entries = nexus_core::search::get_toc(pool, &proj.id, path).map_err(|e| e.to_string())?;
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

fn tool_get_metadata(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let meta = nexus_core::search::get_document_meta(pool, &proj.id, path).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())
}

fn tool_list_documents(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let tag = args.get("tag").and_then(|t| t.as_str());
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let docs = nexus_core::search::list_documents(pool, &proj.id, tag).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&docs).map_err(|e| e.to_string())
}

fn tool_index_project(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let full = args.get("full").and_then(|f| f.as_bool()).unwrap_or(false);
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let report = nexus_core::index_engine::index_project(pool, &proj.id, full).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn tool_get_backlinks(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let backlinks = nexus_core::search::get_backlinks(pool, &proj.id, path).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&backlinks).map_err(|e| e.to_string())
}

fn tool_get_links(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let links = nexus_core::search::get_forward_links(pool, &proj.id, path).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&links).map_err(|e| e.to_string())
}

fn tool_get_cluster(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let depth = args.get("depth").and_then(|d| d.as_i64()).unwrap_or(2).min(5);
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let cluster = nexus_core::search::get_cluster(pool, &proj.id, path, depth).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&cluster).map_err(|e| e.to_string())
}

fn tool_find_path(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let from = args.get("from").and_then(|p| p.as_str()).ok_or("Missing 'from'")?;
    let to = args.get("to").and_then(|p| p.as_str()).ok_or("Missing 'to'")?;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let result = nexus_core::search::find_path(pool, &proj.id, from, to).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
}

fn tool_find_related(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let path = args.get("path").and_then(|p| p.as_str()).ok_or("Missing 'path'")?;
    let k = args.get("k").and_then(|k| k.as_u64()).unwrap_or(10) as usize;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let related = nexus_core::search::find_related(pool, &proj.id, path, k).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&related).map_err(|e| e.to_string())
}

fn tool_resolve_alias(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let project = args.get("project").and_then(|p| p.as_str()).ok_or("Missing 'project'")?;
    let alias = args.get("alias").and_then(|a| a.as_str()).ok_or("Missing 'alias'")?;
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    let result = nexus_core::search::resolve_by_alias(pool, &proj.id, alias).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
}

fn tool_get_ranking(args: &Value, pool: &nexus_core::db::sqlite::DbPool) -> std::result::Result<String, String> {
    let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;
    let project_id = if let Some(p) = args.get("project").and_then(|p| p.as_str()) {
        let proj = nexus_core::project::get_project(pool, p).map_err(|e| e.to_string())?;
        Some(proj.id)
    } else {
        None
    };
    let docs = nexus_core::search::get_popular_documents(pool, project_id.as_deref(), limit)
        .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&docs).map_err(|e| e.to_string())
}

fn tool_onboard(args: &Value) -> std::result::Result<String, String> {
    let force = args.get("force").and_then(|f| f.as_bool()).unwrap_or(false);
    let project_path = args.get("project_path").and_then(|p| p.as_str());
    let result = nexus_core::onboard::onboard(project_path, force).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())
}
