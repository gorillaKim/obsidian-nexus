use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Config;
use crate::db::sqlite::DbPool;
use crate::error::{NexusError, Result};
use crate::index_engine;

/// Debounced file event
#[derive(Debug, Clone)]
struct FileEvent {
    path: PathBuf,
    kind: FileEventKind,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
enum FileEventKind {
    CreateOrModify,
    Remove,
}

/// Watch a project's vault directory and re-index on changes.
/// This blocks the current thread until the stop signal is received.
pub fn watch_project(
    pool: &DbPool,
    project_id: &str,
    config: &Config,
    stop_rx: mpsc::Receiver<()>,
) -> Result<()> {
    let project = crate::project::get_project(pool, project_id)?;
    let vault_path = PathBuf::from(&project.path);

    if !vault_path.is_dir() {
        return Err(NexusError::PathNotFound(project.path));
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
        .map_err(|e| NexusError::Watcher(format!("Failed to create watcher: {}", e)))?;

    watcher
        .watch(&vault_path, RecursiveMode::Recursive)
        .map_err(|e| NexusError::Watcher(format!("Failed to watch {}: {}", vault_path.display(), e)))?;

    tracing::info!("Watching {} for changes...", vault_path.display());

    let debounce = Duration::from_millis(config.watcher.debounce_ms);
    let mut pending: Vec<FileEvent> = Vec::new();
    let mut last_flush = Instant::now();

    loop {
        // Check stop signal (non-blocking)
        if stop_rx.try_recv().is_ok() {
            tracing::info!("Stop signal received, shutting down watcher");
            break;
        }

        // Receive file events with timeout
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(event)) => {
                for path in event.paths {
                    if !is_markdown(&path) || config.is_excluded(&path) {
                        continue;
                    }

                    let kind = match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => FileEventKind::CreateOrModify,
                        EventKind::Remove(_) => FileEventKind::Remove,
                        _ => continue,
                    };

                    // Deduplicate: replace existing event for same path
                    pending.retain(|e| e.path != path);
                    pending.push(FileEvent {
                        path,
                        kind,
                        timestamp: Instant::now(),
                    });
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Watch error: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                tracing::info!("Watcher channel disconnected");
                break;
            }
        }

        // Flush debounced events
        if !pending.is_empty() && last_flush.elapsed() >= debounce {
            let ready: Vec<FileEvent> = pending
                .drain(..)
                .filter(|e| e.timestamp.elapsed() >= debounce)
                .collect();

            // Put back events that aren't ready yet
            // (already drained, so ready contains filtered ones)

            for event in &ready {
                process_event(pool, project_id, &vault_path, event, config);
            }

            if !ready.is_empty() {
                last_flush = Instant::now();
            }
        }
    }

    Ok(())
}

/// Watch all registered projects
pub fn watch_all(
    pool: &DbPool,
    config: &Config,
    stop_rx: mpsc::Receiver<()>,
) -> Result<()> {
    let projects = crate::project::list_projects(pool)?;

    if projects.is_empty() {
        return Err(NexusError::Watcher("No projects registered".to_string()));
    }

    // For simplicity, watch all vaults with a single watcher
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
        .map_err(|e| NexusError::Watcher(format!("Failed to create watcher: {}", e)))?;

    // Map vault paths to project IDs
    let mut path_to_project: Vec<(PathBuf, String)> = Vec::new();
    for proj in &projects {
        let vault_path = PathBuf::from(&proj.path);
        if vault_path.is_dir() {
            watcher
                .watch(&vault_path, RecursiveMode::Recursive)
                .map_err(|e| NexusError::Watcher(format!("Failed to watch {}: {}", vault_path.display(), e)))?;
            tracing::info!("Watching {} ({})", proj.name, vault_path.display());
            path_to_project.push((vault_path, proj.id.clone()));
        } else {
            tracing::warn!("Skipping {} — path not found: {}", proj.name, proj.path);
        }
    }

    let debounce = Duration::from_millis(config.watcher.debounce_ms);
    let mut pending: Vec<(String, PathBuf, FileEvent)> = Vec::new();

    loop {
        if stop_rx.try_recv().is_ok() {
            tracing::info!("Stop signal received");
            break;
        }

        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(event)) => {
                for path in event.paths {
                    if !is_markdown(&path) || config.is_excluded(&path) {
                        continue;
                    }

                    let kind = match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => FileEventKind::CreateOrModify,
                        EventKind::Remove(_) => FileEventKind::Remove,
                        _ => continue,
                    };

                    // Find which project this file belongs to
                    if let Some((vault_path, project_id)) = path_to_project.iter().find(|(vp, _)| path.starts_with(vp)) {
                        pending.retain(|(_, p, _)| p != &path);
                        pending.push((
                            project_id.clone(),
                            vault_path.clone(),
                            FileEvent {
                                path: path.clone(),
                                kind,
                                timestamp: Instant::now(),
                            },
                        ));
                    }
                }
            }
            Ok(Err(e)) => tracing::warn!("Watch error: {}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        // Flush debounced events
        let now = Instant::now();
        let mut i = 0;
        while i < pending.len() {
            if now.duration_since(pending[i].2.timestamp) >= debounce {
                let (project_id, vault_path, event) = pending.remove(i);
                process_event(pool, &project_id, &vault_path, &event, config);
            } else {
                i += 1;
            }
        }
    }

    Ok(())
}

fn process_event(
    pool: &DbPool,
    project_id: &str,
    vault_path: &Path,
    event: &FileEvent,
    _config: &Config,
) {
    let rel_path = event
        .path
        .strip_prefix(vault_path)
        .unwrap_or(&event.path)
        .to_string_lossy()
        .to_string();

    match event.kind {
        FileEventKind::CreateOrModify => {
            tracing::info!("File changed: {} — re-indexing", rel_path);
            // Re-index the entire project (incremental — only changed files)
            match index_engine::index_project(pool, project_id, false) {
                Ok(report) => {
                    if report.indexed > 0 {
                        tracing::info!(
                            "Indexed {} file(s), {} unchanged, {} errors",
                            report.indexed, report.unchanged, report.errors
                        );
                    }
                }
                Err(e) => tracing::error!("Re-indexing failed: {}", e),
            }
        }
        FileEventKind::Remove => {
            tracing::info!("File removed: {} — cleaning up index", rel_path);
            if let Err(e) = remove_document_index(pool, project_id, &rel_path) {
                tracing::error!("Failed to clean up index for {}: {}", rel_path, e);
            }
        }
    }
}

fn remove_document_index(pool: &DbPool, project_id: &str, file_path: &str) -> Result<()> {
    let conn = pool.get()?;
    conn.execute(
        "DELETE FROM documents WHERE project_id = ?1 AND file_path = ?2",
        rusqlite::params![project_id, file_path],
    )?;
    Ok(())
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .map_or(false, |ext| ext == "md" || ext == "markdown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_markdown() {
        assert!(is_markdown(Path::new("note.md")));
        assert!(is_markdown(Path::new("path/to/note.markdown")));
        assert!(!is_markdown(Path::new("image.png")));
        assert!(!is_markdown(Path::new("config.toml")));
    }

    #[test]
    fn test_watcher_stop_signal() {
        let pool = crate::test_helpers::helpers::test_pool();
        let vault = crate::test_helpers::helpers::create_test_vault();
        let proj = crate::project::add_project(&pool, "watch-test", vault.path().to_str().unwrap(), None).unwrap();
        let config = Config::default();

        let (stop_tx, stop_rx) = mpsc::channel();

        // Send stop immediately
        stop_tx.send(()).unwrap();

        // Should exit cleanly
        let result = watch_project(&pool, &proj.id, &config, stop_rx);
        assert!(result.is_ok());
    }
}
