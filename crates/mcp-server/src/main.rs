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
- **nexus_get_metadata** — Get frontmatter, tags, indexing status

## Graph Navigation
- **nexus_get_backlinks** — Documents linking TO this document
- **nexus_get_links** — Documents this document links TO

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
                    "description": "Search across indexed Obsidian documents (hybrid/keyword/vector modes with metadata reranking)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query" },
                            "project": { "type": "string", "description": "Project name or ID (optional, searches all if omitted)" },
                            "limit": { "type": "integer", "description": "Max results (default: 20)", "default": 20 },
                            "mode": { "type": "string", "description": "Search mode: hybrid (default), keyword, vector", "default": "hybrid" },
                            "enrich": { "type": "boolean", "description": "Include metadata (tags, backlink_count, view_count, last_modified) in results (default: true)", "default": true },
                            "use_popularity": { "type": "boolean", "description": "Boost results by popularity. Default: true if project specified, false otherwise" },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "Filter results by tags (optional)" },
                            "tag_match_all": { "type": "boolean", "description": "If true, require ALL tags to match (AND). Default false (OR).", "default": false },
                            "rewrite_query": { "type": "boolean", "description": "LLM으로 쿼리를 재작성하여 도메인 용어 매칭 향상 (Ollama 필요, config.llm.enabled 또는 이 파라미터로 활성화)", "default": false }
                        },
                        "required": ["query"]
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
                            "heading": { "type": "string", "description": "Heading text to extract (e.g. 'Introduction')" }
                        },
                        "required": ["project", "path", "heading"]
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
        "nexus_get_metadata" => tool_get_metadata(&args, pool),
        "nexus_list_documents" => tool_list_documents(&args, pool),
        "nexus_index_project" => tool_index_project(&args, pool),
        "nexus_sync_config" => tool_sync_config(&args, pool),
        "nexus_get_section" => tool_get_section(&args, pool),
        "nexus_get_backlinks" => tool_get_backlinks(&args, pool),
        "nexus_get_links" => tool_get_links(&args, pool),
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
    let query = args.get("query").and_then(|q| q.as_str()).ok_or("Missing 'query'")?;
    let project = args.get("project").and_then(|p| p.as_str());
    let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(20) as usize;
    let mode = args.get("mode").and_then(|m| m.as_str()).unwrap_or("hybrid");
    let enrich = args.get("enrich").and_then(|e| e.as_bool()).unwrap_or(true);
    // Default use_popularity: true if project filter, false otherwise
    let use_popularity = args.get("use_popularity").and_then(|u| u.as_bool())
        .unwrap_or(project.is_some());

    let resolved_pid = if let Some(p) = project {
        let proj = nexus_core::project::get_project(pool, p).map_err(|e| e.to_string())?;
        Some(proj.id)
    } else {
        None
    };

    // 태그 필터 준비 (검색 전)
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

    let mut config = nexus_core::Config::load().unwrap_or_default();
    // rewrite_query 파라미터로 per-request LLM 재작성 활성화 가능
    let rewrite_query_param = args.get("rewrite_query").and_then(|v| v.as_bool()).unwrap_or(false);
    if rewrite_query_param {
        config.llm.enabled = true;
    }

    // LLM query rewriting — keyword/vector/hybrid 모든 모드에 공통 적용
    let effective_query: String = if config.llm.enabled {
        nexus_core::llm::rewrite_query(&config, query)
            .unwrap_or_else(|_| query.to_string())
    } else {
        query.to_string()
    };
    let search_query = effective_query.as_str();

    let mut results = match mode {
        "keyword" => nexus_core::search::fts_search(pool, search_query, resolved_pid.as_deref(), limit, tag_filter.as_ref())
            .map_err(|e| e.to_string())?,
        "vector" => nexus_core::search::vector_search(pool, search_query, resolved_pid.as_deref(), limit, &config, tag_filter.as_ref())
            .map_err(|e| e.to_string())?,
        _ => nexus_core::search::hybrid_search(pool, search_query, resolved_pid.as_deref(), limit, &config, tag_filter.as_ref())
            .map_err(|e| e.to_string())?,
    };

    if enrich {
        nexus_core::search::enrich_results(pool, &mut results, use_popularity)
            .map_err(|e| e.to_string())?;
    }

    serde_json::to_string_pretty(&results).map_err(|e| e.to_string())
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
    let proj = nexus_core::project::get_project(pool, project).map_err(|e| e.to_string())?;
    nexus_core::search::get_section(pool, &proj.id, path, heading).map_err(|e| e.to_string())
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
    Ok(result.report())
}
