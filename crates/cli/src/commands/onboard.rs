use anyhow::Result;
use nexus_core::onboard::StepStatus;

pub fn handle_onboard(project_path: Option<&str>, force: bool) -> Result<()> {
    let steps = nexus_core::onboard::onboard(project_path, force)?;

    for step in &steps {
        match step.status {
            StepStatus::Created => eprintln!("✅ {}: {}", step.name, step.message),
            StepStatus::Skipped => eprintln!("⏭️  {}: {}", step.name, step.message),
            StepStatus::Error   => eprintln!("❌ {}: {}", step.name, step.message),
        }
    }

    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  1. Claude Code 세션을 재시작하면 적용됩니다.");
    eprintln!("  2. nexus_search 도구로 문서를 검색하세요.");

    Ok(())
}
