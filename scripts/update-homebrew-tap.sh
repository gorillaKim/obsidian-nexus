#!/usr/bin/env bash
# Homebrew tap 자동 업데이트 스크립트
# CI에서 릴리즈 후 호출: ./scripts/update-homebrew-tap.sh v0.3.2
#
# 필요 환경변수:
#   TAP_REPO_TOKEN  - homebrew-nexus 레포에 쓰기 권한이 있는 GitHub PAT

set -euo pipefail

VERSION="${1:-}"
[[ -z "$VERSION" ]] && { echo "Usage: $0 <version>  (e.g. v0.3.2)"; exit 1; }
VERSION="${VERSION#v}"   # strip leading 'v'
TAG="v${VERSION}"

REPO="gorillaKim/obsidian-nexus"
TAP_REPO="gorillaKim/homebrew-nexus"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"

echo "→ Fetching release artifacts for ${TAG}..."

# ── SHA256 수집 ───────────────────────────────────────────────────────────────
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "${BASE_URL}/nexus-cli-darwin-aarch64.tar.gz" -o "${TMP}/aarch64.tar.gz"
curl -fsSL "${BASE_URL}/nexus-cli-darwin-x86_64.tar.gz"  -o "${TMP}/x86_64.tar.gz"
curl -fsSL "${BASE_URL}/Obsidian-Nexus.dmg"              -o "${TMP}/Obsidian-Nexus.dmg"

SHA_AARCH64=$(shasum -a 256 "${TMP}/aarch64.tar.gz"      | awk '{print $1}')
SHA_X86_64=$(shasum  -a 256 "${TMP}/x86_64.tar.gz"       | awk '{print $1}')
SHA_DMG=$(shasum     -a 256 "${TMP}/Obsidian-Nexus.dmg"  | awk '{print $1}')

echo "  aarch64:  $SHA_AARCH64"
echo "  x86_64:   $SHA_X86_64"
echo "  dmg:      $SHA_DMG"

# ── 탭 레포 clone ─────────────────────────────────────────────────────────────
TAP_DIR="${TMP}/homebrew-nexus"
git clone "https://x-access-token:${TAP_REPO_TOKEN}@github.com/${TAP_REPO}.git" "$TAP_DIR"
mkdir -p "${TAP_DIR}/Formula" "${TAP_DIR}/Casks"

# ── Formula/obsidian-nexus.rb 생성 ───────────────────────────────────────────
cat > "${TAP_DIR}/Formula/obsidian-nexus.rb" <<FORMULA
class ObsidianNexus < Formula
  desc "Agent-friendly knowledge search engine for Obsidian vaults (CLI)"
  homepage "https://github.com/gorillaKim/obsidian-nexus"
  version "${VERSION}"

  on_macos do
    on_arm do
      url "https://github.com/gorillaKim/obsidian-nexus/releases/download/v#{version}/nexus-cli-darwin-aarch64.tar.gz"
      sha256 "${SHA_AARCH64}"
    end
    on_intel do
      url "https://github.com/gorillaKim/obsidian-nexus/releases/download/v#{version}/nexus-cli-darwin-x86_64.tar.gz"
      sha256 "${SHA_X86_64}"
    end
  end

  def install
    bin.install "obs-nexus"
    bin.install "nexus-mcp-server"
  end

  def post_install
    ohai "Obsidian Nexus 설치 완료!"
    ohai "다음 단계:"
    ohai "  1. nexus setup          # 초기화 (Ollama 확인, DB 생성)"
    ohai "  2. nexus project add --name 'my-vault' --path /path/to/vault"
    ohai "  3. nexus index my-vault # 문서 인덱싱"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/nexus --version")
  end
end
FORMULA

# ── Casks/obsidian-nexus.rb 생성 ─────────────────────────────────────────────
cat > "${TAP_DIR}/Casks/obsidian-nexus.rb" <<CASK
cask "obsidian-nexus" do
  version "${VERSION}"
  sha256 "${SHA_DMG}"

  url "https://github.com/gorillaKim/obsidian-nexus/releases/download/v#{version}/Obsidian-Nexus.dmg"
  name "Obsidian Nexus"
  desc "Agent-friendly knowledge search engine for Obsidian vaults"
  homepage "https://github.com/gorillaKim/obsidian-nexus"

  app "Obsidian Nexus.app"

  zap trash: [
    "~/.nexus",
    "~/Library/Application Support/com.obsidian-nexus.app",
    "~/Library/Caches/com.obsidian-nexus.app",
    "~/Library/Preferences/com.obsidian-nexus.app.plist",
    "~/Library/Logs/com.obsidian-nexus.app",
  ]
end
CASK

# ── 커밋 + 푸시 ───────────────────────────────────────────────────────────────
cd "$TAP_DIR"
git config user.name  "github-actions[bot]"
git config user.email "github-actions[bot]@users.noreply.github.com"
git add Formula/obsidian-nexus.rb Casks/obsidian-nexus.rb
git commit -m "chore: bump to ${TAG}"
git push

echo "✓ homebrew-nexus tap updated to ${TAG}"
