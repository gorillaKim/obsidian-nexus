use anyhow::Result;

pub fn handle_onboard(project_path: Option<&str>, force: bool) -> Result<()> {
    let result = nexus_core::onboard::onboard(project_path, force)?;

    if !result.created.is_empty() {
        eprintln!("✅ Created:");
        for f in &result.created {
            eprintln!("  - {}", f);
        }
    }

    if !result.skipped.is_empty() {
        eprintln!("⏭️  Skipped:");
        for f in &result.skipped {
            eprintln!("  - {}", f);
        }
    }

    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  1. Restart Claude Code session");
    eprintln!("  2. Use /librarian <query> to search documents");

    Ok(())
}
