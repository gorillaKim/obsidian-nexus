use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

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
                    "description": "Search across indexed Obsidian documents using FTS5 full-text search",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query" },
                            "project": { "type": "string", "description": "Project name or ID (optional, searches all if omitted)" },
                            "limit": { "type": "integer", "description": "Max results (default: 20)", "default": 20 }
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

    let resolved_pid = if let Some(p) = project {
        let proj = nexus_core::project::get_project(pool, p).map_err(|e| e.to_string())?;
        Some(proj.id)
    } else {
        None
    };

    let results = nexus_core::search::fts_search(pool, query, resolved_pid.as_deref(), limit)
        .map_err(|e| e.to_string())?;
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
    nexus_core::search::get_document_content(pool, &proj.id, path).map_err(|e| e.to_string())
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
