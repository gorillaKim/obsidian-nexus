use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;

const GITHUB_API_URL: &str = "https://api.github.com/repos/gorillaKim/obsidian-nexus/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

fn bin_dir() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("Cannot find home directory")?
        .join(".local")
        .join("bin"))
}

fn cache_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("Cannot find home directory")?
        .join(".nexus")
        .join("update-check"))
}

fn should_skip_check() -> bool {
    let path = match cache_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    if let Ok(meta) = fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                return elapsed.as_secs() < 86400; // 24 hours
            }
        }
    }
    false
}

fn touch_cache() {
    let path = match cache_path() {
        Ok(p) => p,
        Err(_) => return,
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, "");
}

fn asset_name() -> String {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        other => other,
    };
    format!("nexus-cli-{}-{}.tar.gz", std::env::consts::OS, arch)
}

fn checksum_asset_name() -> String {
    format!("{}.sha256", asset_name())
}

pub fn handle_update(check_only: bool, force: bool, format: &str) -> Result<()> {
    if !force && !check_only && should_skip_check() {
        eprintln!("최근 24시간 내 확인됨. --force로 강제 확인 가능.");
        return Ok(());
    }
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { do_update(check_only, format).await })
}

async fn do_update(check_only: bool, format: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("nexus-cli")
        .build()?;

    let release: GithubRelease = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .context("Failed to reach GitHub API")?
        .error_for_status()
        .context("GitHub API returned an error")?
        .json()
        .await
        .context("Failed to parse release info")?;

    let latest = release.tag_name.trim_start_matches('v');
    let current = CURRENT_VERSION;

    let has_update = version_gt(latest, current);

    if format == "json" {
        let info = serde_json::json!({
            "current_version": current,
            "latest_version": latest,
            "has_update": has_update,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else if has_update {
        println!("새 버전 사용 가능: v{} (현재: v{})", latest, current);
    } else {
        println!("최신 버전입니다 (v{})", current);
    }

    if check_only || !has_update {
        touch_cache();
        return Ok(());
    }

    // Find assets
    let target = asset_name();
    let checksum_name = checksum_asset_name();

    let binary_asset = release
        .assets
        .iter()
        .find(|a| a.name == target)
        .context(format!("Release에 {} 에셋이 없습니다", target))?;

    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name == checksum_name)
        .context(format!("Release에 {} 체크섬 파일이 없습니다. 무결성 검증 불가.", checksum_name))?;

    // Download binary
    eprintln!("다운로드 중: {}...", target);
    let bytes = client
        .get(&binary_asset.browser_download_url)
        .send()
        .await?
        .error_for_status()
        .context("바이너리 다운로드 실패")?
        .bytes()
        .await?;

    // Verify checksum (mandatory)
    eprintln!("체크섬 검증 중...");
    let cs_text = client
        .get(&checksum_asset.browser_download_url)
        .send()
        .await?
        .error_for_status()
        .context("체크섬 파일 다운로드 실패")?
        .text()
        .await?;

    let expected = cs_text.split_whitespace().next().unwrap_or("");
    let actual = sha256_hex(&bytes);

    if expected != actual {
        bail!(
            "체크섬 불일치!\n  예상: {}\n  실제: {}",
            expected,
            actual
        );
    }
    eprintln!("체크섬 검증 완료");

    // Extract to temp dir
    let tmp = tempfile::tempdir()?;
    let decoder = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(tmp.path())?;

    let bin = bin_dir()?;
    fs::create_dir_all(&bin)?;

    // Atomic replace: .new → .bak → rename
    for name in &["nexus", "nexus-mcp-server"] {
        let src = tmp.path().join(name);
        if !src.exists() {
            continue;
        }

        let dest = bin.join(name);
        let new_path = bin.join(format!("{}.new", name));
        let bak_path = bin.join(format!("{}.bak", name));

        // Copy to .new
        fs::copy(&src, &new_path)
            .context(format!("{}.new 생성 실패", name))?;

        // Set executable permission
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&new_path, fs::Permissions::from_mode(0o755))?;
        }

        // Backup existing
        if dest.exists() {
            let _ = fs::remove_file(&bak_path);
            if let Err(e) = fs::rename(&dest, &bak_path) {
                let _ = fs::remove_file(&new_path);
                bail!("{} 백업 실패: {}", name, e);
            }
        }

        // Atomic rename .new → final
        if let Err(e) = fs::rename(&new_path, &dest) {
            if bak_path.exists() {
                let _ = fs::rename(&bak_path, &dest);
            }
            bail!("{} 교체 실패: {}", name, e);
        }

        eprintln!("  ✓ {} 업데이트 완료", name);
    }

    touch_cache();
    eprintln!("\nv{} → v{} 업데이트 완료!", current, latest);
    Ok(())
}

fn version_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>())
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    parse(a) > parse(b)
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
