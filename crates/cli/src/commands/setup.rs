use anyhow::Result;
use std::process::Command;

pub fn handle_setup() -> Result<()> {
    eprintln!("🔧 Obsidian Nexus Setup\n");

    // 1. Check/create data directory
    eprint!("[1/5] Data directory (~/.nexus/) ... ");
    nexus_core::Config::ensure_dirs()?;
    eprintln!("✓");

    // 2. Check/install Obsidian
    eprint!("[2/5] Obsidian ... ");
    if check_app_installed("Obsidian") {
        eprintln!("✓ (already installed)");
    } else {
        eprintln!("not found, installing...");
        let status = Command::new("brew")
            .args(["install", "--cask", "obsidian"])
            .status();
        match status {
            Ok(s) if s.success() => eprintln!("  ✓ Obsidian installed"),
            _ => {
                eprintln!("  ✗ Failed to install via brew. Install manually:");
                eprintln!("    brew install --cask obsidian");
                eprintln!("    OR visit https://obsidian.md/download");
            }
        }
    }

    // 3. Check/install Ollama
    eprint!("[3/5] Ollama ... ");
    if check_command("ollama", &["--version"]) {
        eprintln!("✓ (already installed)");
    } else {
        eprintln!("not found, installing...");
        let status = Command::new("brew")
            .args(["install", "ollama"])
            .status();
        match status {
            Ok(s) if s.success() => {
                eprintln!("  ✓ Ollama installed");
                // Start Ollama service
                let _ = Command::new("brew")
                    .args(["services", "start", "ollama"])
                    .status();
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
            _ => {
                eprintln!("  ✗ Failed to install via brew. Install manually:");
                eprintln!("    brew install ollama");
                eprintln!("    OR visit https://ollama.com/download");
            }
        }
    }

    // 4. Check/pull embedding model
    eprint!("[4/5] Embedding model (nomic-embed-text) ... ");
    let config = nexus_core::Config::load().unwrap_or_default();

    match nexus_core::embedding::check_ollama(&config) {
        Ok(()) => eprintln!("✓ (ready)"),
        Err(_) => {
            eprintln!("pulling model...");
            // Ensure Ollama is running
            let _ = Command::new("brew")
                .args(["services", "start", "ollama"])
                .status();
            std::thread::sleep(std::time::Duration::from_secs(3));

            let status = Command::new("ollama")
                .args(["pull", &config.embedding.model])
                .status();
            match status {
                Ok(s) if s.success() => eprintln!("  ✓ Model pulled"),
                _ => {
                    eprintln!("  ✗ Failed. Run manually:");
                    eprintln!("    ollama serve");
                    eprintln!("    ollama pull {}", config.embedding.model);
                }
            }
        }
    }

    // 5. Initialize database
    eprint!("[5/5] Database ... ");
    let pool = nexus_core::db::sqlite::create_pool()?;
    nexus_core::db::sqlite::run_migrations(&pool)?;
    eprintln!("✓");

    // Save default config if not exists
    if !nexus_core::Config::config_path().exists() {
        config.save()?;
        eprintln!("Default config saved to {:?}", nexus_core::Config::config_path());
    }

    eprintln!("\n✅ Setup complete! Next steps:");
    eprintln!("  1. Open Obsidian and note your vault path");
    eprintln!("  2. nexus project add --name \"my-vault\" --path /path/to/obsidian/vault");
    eprintln!("  3. nexus index my-vault");
    eprintln!("  4. nexus search \"query\" --mode hybrid");

    Ok(())
}

fn check_command(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if a macOS application is installed
fn check_app_installed(app_name: &str) -> bool {
    let app_path = format!("/Applications/{}.app", app_name);
    std::path::Path::new(&app_path).exists()
}
