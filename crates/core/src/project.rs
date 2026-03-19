use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::sqlite::DbPool;
use crate::error::{NexusError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub vault_name: Option<String>,
    pub path: String,
    pub created_at: Option<String>,
    pub last_indexed_at: Option<String>,
}

/// Add a new project (register a vault path)
pub fn add_project(pool: &DbPool, name: &str, path: &str, vault_name: Option<&str>) -> Result<Project> {
    let abs_path = std::fs::canonicalize(path)
        .map_err(|_| NexusError::PathNotFound(path.to_string()))?;
    let abs_path_str = abs_path.to_string_lossy().to_string();

    if !abs_path.is_dir() {
        return Err(NexusError::PathNotFound(format!("{} is not a directory", path)));
    }

    let conn = pool.get()?;
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO projects (id, name, vault_name, path) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, vault_name, abs_path_str],
    ).map_err(|e| match e {
        rusqlite::Error::SqliteFailure(_, _) => {
            NexusError::ProjectAlreadyExists(name.to_string())
        }
        other => NexusError::Database(other),
    })?;

    Ok(Project {
        id,
        name: name.to_string(),
        vault_name: vault_name.map(|s| s.to_string()),
        path: abs_path_str,
        created_at: None,
        last_indexed_at: None,
    })
}

/// List all projects
pub fn list_projects(pool: &DbPool) -> Result<Vec<Project>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, vault_name, path, created_at, last_indexed_at FROM projects ORDER BY name"
    )?;

    let projects = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            vault_name: row.get(2)?,
            path: row.get(3)?,
            created_at: row.get(4)?,
            last_indexed_at: row.get(5)?,
        })
    })?.collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(projects)
}

/// Get a project by ID or name
pub fn get_project(pool: &DbPool, id_or_name: &str) -> Result<Project> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, vault_name, path, created_at, last_indexed_at
         FROM projects WHERE id = ?1 OR name = ?1"
    )?;

    stmt.query_row(params![id_or_name], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            vault_name: row.get(2)?,
            path: row.get(3)?,
            created_at: row.get(4)?,
            last_indexed_at: row.get(5)?,
        })
    }).map_err(|_| NexusError::ProjectNotFound(id_or_name.to_string()))
}

/// Remove a project by ID or name (only removes index, not files)
pub fn remove_project(pool: &DbPool, id_or_name: &str) -> Result<()> {
    let project = get_project(pool, id_or_name)?;
    let conn = pool.get()?;
    conn.execute("DELETE FROM projects WHERE id = ?1", params![project.id])?;

    // Clean up LanceDB directory
    let lance_dir = crate::config::Config::lance_dir(&project.id);
    if lance_dir.exists() {
        let _ = std::fs::remove_dir_all(&lance_dir);
    }

    Ok(())
}

/// Update project path (for vault relocation)
pub fn update_project_path(pool: &DbPool, id_or_name: &str, new_path: &str) -> Result<Project> {
    let project = get_project(pool, id_or_name)?;
    let abs_path = std::fs::canonicalize(new_path)
        .map_err(|_| NexusError::PathNotFound(new_path.to_string()))?;

    if !abs_path.is_dir() {
        return Err(NexusError::PathNotFound(format!("{} is not a directory", new_path)));
    }

    let abs_path_str = abs_path.to_string_lossy().to_string();
    let conn = pool.get()?;
    conn.execute(
        "UPDATE projects SET path = ?1 WHERE id = ?2",
        params![abs_path_str, project.id],
    )?;

    Ok(Project {
        path: abs_path_str,
        ..project
    })
}

/// Get project document count and stats
pub fn project_info(pool: &DbPool, id_or_name: &str) -> Result<(Project, ProjectStats)> {
    let project = get_project(pool, id_or_name)?;
    let conn = pool.get()?;

    let doc_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE project_id = ?1",
        params![project.id],
        |row| row.get(0),
    )?;

    let chunk_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM chunks c JOIN documents d ON c.document_id = d.id WHERE d.project_id = ?1",
        params![project.id],
        |row| row.get(0),
    )?;

    let pending_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE project_id = ?1 AND indexing_status != 'done'",
        params![project.id],
        |row| row.get(0),
    )?;

    Ok((project, ProjectStats { doc_count, chunk_count, pending_count }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectStats {
    pub doc_count: i64,
    pub chunk_count: i64,
    pub pending_count: i64,
}

/// Detect Obsidian vaults under a directory (folders containing .obsidian/)
/// Returns list of (vault_name, vault_path) pairs
pub fn detect_vaults(root_path: &str) -> Result<Vec<(String, String)>> {
    let root = std::path::Path::new(root_path);
    if !root.is_dir() {
        return Err(NexusError::PathNotFound(root_path.to_string()));
    }

    let mut vaults = Vec::new();

    // Check if root itself is a vault
    if root.join(".obsidian").is_dir() {
        let name = root.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "vault".to_string());
        let abs = std::fs::canonicalize(root)
            .map_err(|_| NexusError::PathNotFound(root_path.to_string()))?;
        vaults.push((name, abs.to_string_lossy().to_string()));
        return Ok(vaults); // Don't recurse into sub-vaults
    }

    // Recurse into subdirectories (max depth 3 to avoid deep traversal)
    fn scan(dir: &std::path::Path, vaults: &mut Vec<(String, String)>, depth: usize) {
        if depth > 3 { return; }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // Skip hidden directories and common non-vault dirs
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            if path.join(".obsidian").is_dir() {
                let abs = std::fs::canonicalize(&path).unwrap_or(path.clone());
                vaults.push((name, abs.to_string_lossy().to_string()));
                // Don't recurse into a vault's subdirectories
            } else {
                scan(&path, vaults, depth + 1);
            }
        }
    }

    scan(root, &mut vaults, 0);
    vaults.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(vaults)
}

/// Auto-detect and register all vaults under a directory
pub fn auto_add_vaults(pool: &DbPool, root_path: &str) -> Result<Vec<Project>> {
    let vaults = detect_vaults(root_path)?;
    let mut added = Vec::new();
    for (name, path) in vaults {
        match add_project(pool, &name, &path, Some(&name)) {
            Ok(proj) => added.push(proj),
            Err(NexusError::ProjectAlreadyExists(_)) => {
                // Already registered, skip
                if let Ok(proj) = get_project(pool, &name) {
                    added.push(proj);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to add vault {}: {}", name, e);
            }
        }
    }
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::helpers::{test_pool, create_test_vault};

    #[test]
    fn test_add_and_get_project() {
        let pool = test_pool();
        let vault = create_test_vault();
        let path = vault.path().to_str().unwrap();

        let project = add_project(&pool, "test-project", path, Some("MyVault")).unwrap();
        assert_eq!(project.name, "test-project");
        assert_eq!(project.vault_name, Some("MyVault".to_string()));

        let fetched = get_project(&pool, "test-project").unwrap();
        assert_eq!(fetched.id, project.id);
        assert_eq!(fetched.name, "test-project");
    }

    #[test]
    fn test_get_project_by_id() {
        let pool = test_pool();
        let vault = create_test_vault();
        let project = add_project(&pool, "by-id", vault.path().to_str().unwrap(), None).unwrap();

        let fetched = get_project(&pool, &project.id).unwrap();
        assert_eq!(fetched.name, "by-id");
    }

    #[test]
    fn test_list_projects() {
        let pool = test_pool();
        let v1 = tempfile::tempdir().unwrap();
        let v2 = tempfile::tempdir().unwrap();

        add_project(&pool, "alpha", v1.path().to_str().unwrap(), None).unwrap();
        add_project(&pool, "beta", v2.path().to_str().unwrap(), None).unwrap();

        let list = list_projects(&pool).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "alpha"); // sorted by name
        assert_eq!(list[1].name, "beta");
    }

    #[test]
    fn test_remove_project() {
        let pool = test_pool();
        let vault = create_test_vault();
        let project = add_project(&pool, "to-remove", vault.path().to_str().unwrap(), None).unwrap();

        remove_project(&pool, "to-remove").unwrap();
        assert!(get_project(&pool, &project.id).is_err());
    }

    #[test]
    fn test_duplicate_name_rejected() {
        let pool = test_pool();
        let v1 = tempfile::tempdir().unwrap();
        let v2 = tempfile::tempdir().unwrap();

        add_project(&pool, "dup", v1.path().to_str().unwrap(), None).unwrap();
        let result = add_project(&pool, "dup", v2.path().to_str().unwrap(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_path_rejected() {
        let pool = test_pool();
        let vault = create_test_vault();
        let path = vault.path().to_str().unwrap();

        add_project(&pool, "first", path, None).unwrap();
        let result = add_project(&pool, "second", path, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_path_rejected() {
        let pool = test_pool();
        let result = add_project(&pool, "bad", "/nonexistent/path/abc123", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_not_found() {
        let pool = test_pool();
        let result = get_project(&pool, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_update_project_path() {
        let pool = test_pool();
        let v1 = create_test_vault();
        let v2 = tempfile::tempdir().unwrap();

        let project = add_project(&pool, "movable", v1.path().to_str().unwrap(), None).unwrap();
        let updated = update_project_path(&pool, "movable", v2.path().to_str().unwrap()).unwrap();

        assert_ne!(project.path, updated.path);
        assert!(updated.path.contains(v2.path().to_str().unwrap().split('/').last().unwrap()));
    }

    #[test]
    fn test_project_info() {
        let pool = test_pool();
        let vault = create_test_vault();
        let project = add_project(&pool, "info-test", vault.path().to_str().unwrap(), None).unwrap();

        let (proj, stats) = project_info(&pool, "info-test").unwrap();
        assert_eq!(proj.id, project.id);
        assert_eq!(stats.doc_count, 0);
        assert_eq!(stats.chunk_count, 0);
    }
}
