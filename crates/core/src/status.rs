use serde_json::{json, Value};

use crate::config::Config;
use crate::db::sqlite::DbPool;

/// Check if Ollama is running and the configured model is available.
/// Returns status info instead of erroring.
fn check_ollama_status(config: &Config) -> Value {
    let url = format!("{}/api/tags", config.embedding.ollama_url);
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return json!({
                "running": false,
                "url": config.embedding.ollama_url,
                "model": config.embedding.model,
                "model_available": false,
                "error": "Failed to create HTTP client"
            });
        }
    };

    let resp = match client.get(&url).send() {
        Ok(r) => r,
        Err(_) => {
            return json!({
                "running": false,
                "url": config.embedding.ollama_url,
                "model": config.embedding.model,
                "model_available": false,
                "error": "Ollama is not running. Start with: ollama serve"
            });
        }
    };

    if !resp.status().is_success() {
        return json!({
            "running": false,
            "url": config.embedding.ollama_url,
            "model": config.embedding.model,
            "model_available": false,
            "error": "Ollama returned an error"
        });
    }

    let model_available = resp
        .json::<Value>()
        .ok()
        .and_then(|data| data.get("models")?.as_array().cloned())
        .map(|models| {
            models.iter().any(|m| {
                m.get("name")
                    .and_then(|n| n.as_str())
                    .map_or(false, |n| n.starts_with(&config.embedding.model))
            })
        })
        .unwrap_or(false);

    let mut result = json!({
        "running": true,
        "url": config.embedding.ollama_url,
        "model": config.embedding.model,
        "model_available": model_available
    });

    if !model_available {
        result["error"] = json!(format!(
            "Model '{}' not found. Install with: ollama pull {}",
            config.embedding.model, config.embedding.model
        ));
    }

    result
}

/// Check database status: existence, schema version, counts.
fn check_database_status(pool: &DbPool) -> Value {
    let db_path = Config::db_path();
    let exists = db_path.exists();
    let path_str = db_path.to_string_lossy().to_string();

    if !exists {
        return json!({
            "exists": false,
            "path": path_str,
            "schema_version": null,
            "project_count": 0,
            "document_count": 0,
            "error": "Database file does not exist"
        });
    }

    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            return json!({
                "exists": true,
                "path": path_str,
                "schema_version": null,
                "project_count": 0,
                "document_count": 0,
                "error": format!("Failed to get connection: {}", e)
            });
        }
    };

    let schema_version: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| row.get(0))
        .unwrap_or(0);

    let project_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .unwrap_or(0);

    let document_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap_or(0);

    json!({
        "exists": true,
        "path": path_str,
        "schema_version": schema_version,
        "project_count": project_count,
        "document_count": document_count
    })
}

/// Check config file status.
fn check_config_status(config: &Config) -> Value {
    let config_path = Config::config_path();
    json!({
        "exists": config_path.exists(),
        "path": config_path.to_string_lossy(),
        "embedding_provider": config.embedding.provider,
        "embedding_model": config.embedding.model,
        "embedding_dimensions": config.embedding.dimensions
    })
}

/// Get full system status as JSON.
pub fn get_status(pool: &DbPool) -> String {
    let config = Config::load().unwrap_or_default();

    let ollama = check_ollama_status(&config);
    let database = check_database_status(pool);
    let config_status = check_config_status(&config);

    let ollama_ok = ollama.get("running").and_then(|v| v.as_bool()).unwrap_or(false)
        && ollama.get("model_available").and_then(|v| v.as_bool()).unwrap_or(false);
    let db_ok = database.get("exists").and_then(|v| v.as_bool()).unwrap_or(false)
        && database.get("error").is_none();

    let overall = if ollama_ok && db_ok { "ready" } else { "not_ready" };

    let result = json!({
        "ollama": ollama,
        "database": database,
        "config": config_status,
        "overall": overall
    });

    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
}
