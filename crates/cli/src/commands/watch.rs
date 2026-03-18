use anyhow::Result;
use std::sync::mpsc;

use nexus_core::db::sqlite::DbPool;

pub fn handle_watch(pool: &DbPool, project: Option<&str>) -> Result<()> {
    let config = nexus_core::Config::load().unwrap_or_default();

    // Setup Ctrl+C handler
    let (stop_tx, stop_rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = stop_tx.send(());
    })?;

    if let Some(pid) = project {
        // Resolve project by name or ID
        let proj = nexus_core::project::get_project(pool, pid)?;
        println!("Watching project '{}' at {}", proj.name, proj.path);
        println!("Press Ctrl+C to stop.");
        nexus_core::watcher::watch_project(pool, &proj.id, &config, stop_rx)?;
    } else {
        println!("Watching all projects...");
        println!("Press Ctrl+C to stop.");
        nexus_core::watcher::watch_all(pool, &config, stop_rx)?;
    }

    println!("\nWatcher stopped.");
    Ok(())
}
