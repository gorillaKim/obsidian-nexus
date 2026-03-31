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

    // If desktop app is installed, prefer symlink so app updates propagate automatically
    let app_macos_dir = std::path::PathBuf::from(
        "/Applications/Obsidian Nexus.app/Contents/MacOS",
    );

    for name in &["obs-nexus", "nexus-mcp-server"] {
        let src = tmp.path().join(name);
        if !src.exists() {
            continue;
        }

        let dest = bin.join(name);
        let app_binary = app_macos_dir.join(name);

        if app_binary.exists() {
            // Compare versions: use whichever binary is newer
            let downloaded_ver = binary_version(&src);
            let app_ver = binary_version(&app_binary);
            let use_symlink = match (&downloaded_ver, &app_ver) {
                (Some(dv), Some(av)) => !version_gt(dv, av),
                _ => true, // version unknown, prefer symlink (safe default)
            };

            if !use_symlink {
                // Downloaded binary is newer than app bundle — install it directly
                eprintln!(
                    "  ℹ {}: 다운로드 버전({}) > 앱 번들 버전({}) — 직접 설치",
                    name,
                    downloaded_ver.as_deref().unwrap_or("?"),
                    app_ver.as_deref().unwrap_or("?")
                );
                install_binary(&src, &dest, &bin, name)?;
            } else {
                // App bundle is up-to-date — symlink so future app updates propagate
                if let (Some(dv), Some(av)) = (&downloaded_ver, &app_ver) {
                    if version_gt(av, dv) {
                        eprintln!(
                            "  ℹ {}: 앱 번들 버전({}) > 다운로드 버전({}) — 심볼릭 링크 사용",
                            name, av, dv
                        );
                    }
                }
                if dest.exists() || dest.is_symlink() {
                    let _ = fs::remove_file(&dest);
                }
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(&app_binary, &dest)
                        .context(format!("{} 심볼릭 링크 생성 실패", name))?;
                }
                eprintln!("  ✓ {} → 앱 번들 심볼릭 링크", name);
            }
        } else {
            // No desktop app — atomic replace
            install_binary(&src, &dest, &bin, name)?;
        }
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

/// Run `binary --version` and return the version string (e.g. "0.5.9").
fn binary_version(path: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new(path)
        .arg("--version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Expected format: "obs-nexus 0.5.9" or just "0.5.9"
    text.split_whitespace()
        .find(|s| s.chars().next().map_or(false, |c| c.is_ascii_digit()))
        .map(|s| s.trim().to_string())
}

/// Atomic replace: src → dest (.new/.bak pattern).
fn install_binary(
    src: &std::path::Path,
    dest: &std::path::Path,
    bin: &std::path::Path,
    name: &str,
) -> anyhow::Result<()> {
    let new_path = bin.join(format!("{}.new", name));
    let bak_path = bin.join(format!("{}.bak", name));

    fs::copy(src, &new_path).context(format!("{}.new 생성 실패", name))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&new_path, fs::Permissions::from_mode(0o755))?;
    }

    if dest.exists() || dest.is_symlink() {
        let _ = fs::remove_file(&bak_path);
        if let Err(e) = fs::rename(dest, &bak_path) {
            let _ = fs::remove_file(&new_path);
            bail!("{} 백업 실패: {}", name, e);
        }
    }

    if let Err(e) = fs::rename(&new_path, dest) {
        if bak_path.exists() {
            let _ = fs::rename(&bak_path, dest);
        }
        bail!("{} 교체 실패: {}", name, e);
    }

    eprintln!("  ✓ {} 업데이트 완료", name);
    Ok(())
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    // ── version_gt ────────────────────────────────────────────────────────────

    #[test]
    fn version_gt_basic() {
        assert!(version_gt("0.5.10", "0.5.9"));
        assert!(version_gt("1.0.0", "0.9.9"));
        assert!(!version_gt("0.5.9", "0.5.9"));
        assert!(!version_gt("0.5.8", "0.5.9"));
    }

    #[test]
    fn version_gt_with_suffix() {
        // binary_version strips non-digit suffixes; version_gt should handle clean strings
        assert!(version_gt("0.6.0", "0.5.9"));
        assert!(!version_gt("0.5.9", "0.6.0"));
    }

    // ── binary_version ────────────────────────────────────────────────────────

    /// Create a tiny shell script that prints "<name> <ver>" to stdout.
    fn make_fake_binary(dir: &std::path::Path, name: &str, version: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\necho \"{} {}\"\n", name, version)).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn binary_version_parses_output() {
        let dir = tempdir().unwrap();
        let bin = make_fake_binary(dir.path(), "obs-nexus", "0.5.9");
        assert_eq!(binary_version(&bin), Some("0.5.9".to_string()));
    }

    #[test]
    fn binary_version_returns_none_for_nonexistent() {
        let path = std::path::PathBuf::from("/nonexistent/binary");
        assert_eq!(binary_version(&path), None);
    }

    #[test]
    fn binary_version_handles_version_only_output() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("plain");
        std::fs::write(&path, "#!/bin/sh\necho \"0.6.1\"\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(binary_version(&path), Some("0.6.1".to_string()));
    }

    // ── install_binary ────────────────────────────────────────────────────────

    #[test]
    fn install_binary_creates_dest() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::write(&src, b"binary content").unwrap();
        let dest = dir.path().join("dest");

        install_binary(&src, &dest, dir.path(), "dest").unwrap();

        assert!(dest.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"binary content");
        // executable
        let mode = std::fs::metadata(&dest).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "dest should be executable");
    }

    #[test]
    fn install_binary_replaces_existing_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::write(&src, b"new content").unwrap();
        let dest = dir.path().join("dest");
        std::fs::write(&dest, b"old content").unwrap();

        install_binary(&src, &dest, dir.path(), "dest").unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"new content");
        // .bak should exist with old content
        let bak = dir.path().join("dest.bak");
        assert_eq!(std::fs::read(&bak).unwrap(), b"old content");
    }

    #[test]
    fn install_binary_replaces_existing_symlink() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("app_binary");
        std::fs::write(&target, b"app content").unwrap();
        let src = dir.path().join("src");
        std::fs::write(&src, b"newer content").unwrap();
        let dest = dir.path().join("dest");
        std::os::unix::fs::symlink(&target, &dest).unwrap();

        install_binary(&src, &dest, dir.path(), "dest").unwrap();

        // dest should now be a regular file, not a symlink
        assert!(!dest.is_symlink());
        assert_eq!(std::fs::read(&dest).unwrap(), b"newer content");
    }

    // ── version-based symlink decision ────────────────────────────────────────

    #[test]
    fn prefers_symlink_when_app_bundle_is_newer() {
        let dir = tempdir().unwrap();
        let downloaded = make_fake_binary(dir.path(), "downloaded", "0.5.9");
        let app_bin = make_fake_binary(dir.path(), "app_bin", "0.6.0");

        let dv = binary_version(&downloaded);
        let av = binary_version(&app_bin);
        let use_symlink = match (&dv, &av) {
            (Some(d), Some(a)) => !version_gt(d, a),
            _ => true,
        };
        assert!(use_symlink, "app bundle newer → should symlink");
    }

    #[test]
    fn prefers_direct_install_when_downloaded_is_newer() {
        let dir = tempdir().unwrap();
        let downloaded = make_fake_binary(dir.path(), "downloaded", "0.6.1");
        let app_bin = make_fake_binary(dir.path(), "app_bin", "0.6.0");

        let dv = binary_version(&downloaded);
        let av = binary_version(&app_bin);
        let use_symlink = match (&dv, &av) {
            (Some(d), Some(a)) => !version_gt(d, a),
            _ => true,
        };
        assert!(!use_symlink, "downloaded newer → should install directly");
    }

    #[test]
    fn prefers_symlink_on_equal_versions() {
        let dir = tempdir().unwrap();
        let downloaded = make_fake_binary(dir.path(), "downloaded", "0.6.0");
        let app_bin = make_fake_binary(dir.path(), "app_bin", "0.6.0");

        let dv = binary_version(&downloaded);
        let av = binary_version(&app_bin);
        let use_symlink = match (&dv, &av) {
            (Some(d), Some(a)) => !version_gt(d, a),
            _ => true,
        };
        assert!(use_symlink, "equal versions → should symlink");
    }

    #[test]
    fn falls_back_to_symlink_on_unknown_version() {
        let dir = tempdir().unwrap();
        // binary that prints nothing useful
        let path = dir.path().join("silent");
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let dv = binary_version(&path);
        let av: Option<String> = None;
        let use_symlink = match (&dv, &av) {
            (Some(d), Some(a)) => !version_gt(d, a),
            _ => true,
        };
        assert!(use_symlink, "unknown version → safe default is symlink");
    }
}
