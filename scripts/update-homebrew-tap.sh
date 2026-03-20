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

curl -fsSL "${BASE_URL}/nexus-cli-darwin-aarch64.tar.gz"        -o "${TMP}/aarch64.tar.gz"
curl -fsSL "${BASE_URL}/nexus-cli-darwin-x86_64.tar.gz"         -o "${TMP}/x86_64.tar.gz"
curl -fsSL "${BASE_URL}/Obsidian-Nexus.dmg"                     -o "${TMP}/Obsidian-Nexus.dmg"

SHA_AARCH64=$(shasum -a 256 "${TMP}/aarch64.tar.gz" | awk '{print $1}')
SHA_X86_64=$(shasum  -a 256 "${TMP}/x86_64.tar.gz"  | awk '{print $1}')
SHA_DMG=$(shasum     -a 256 "${TMP}/Obsidian-Nexus.dmg" | awk '{print $1}')

echo "  aarch64:  $SHA_AARCH64"
echo "  x86_64:   $SHA_X86_64"
echo "  dmg:      $SHA_DMG"

# ── 탭 레포 clone ─────────────────────────────────────────────────────────────
TAP_DIR="${TMP}/homebrew-nexus"
git clone "https://x-access-token:${TAP_REPO_TOKEN}@github.com/${TAP_REPO}.git" "$TAP_DIR"

# ── Formula/nexus.rb 업데이트 ─────────────────────────────────────────────────
FORMULA="${TAP_DIR}/Formula/nexus.rb"
sed -i '' "s/version \".*\"/version \"${VERSION}\"/"                    "$FORMULA"
sed -i '' "s/PLACEHOLDER_SHA256_AARCH64/${SHA_AARCH64}/"                "$FORMULA"
sed -i '' "s/PLACEHOLDER_SHA256_X86_64/${SHA_X86_64}/"                  "$FORMULA"
# 두 번째 실행 이후(이미 실제 값) → 정규식으로 교체
perl -i -0pe "s/(on_arm do.*?sha256 \")[^\"]+(\")/${\"\$1\"}${SHA_AARCH64}\$2/s"  "$FORMULA" 2>/dev/null || true
perl -i -0pe "s/(on_intel do.*?sha256 \")[^\"]+(\")/${\"\$1\"}${SHA_X86_64}\$2/s" "$FORMULA" 2>/dev/null || true

# ── Casks/obsidian-nexus.rb 업데이트 ─────────────────────────────────────────
CASK="${TAP_DIR}/Casks/obsidian-nexus.rb"
sed -i '' "s/version \".*\"/version \"${VERSION}\"/" "$CASK"
sed -i '' "s/sha256 \".*\"/sha256 \"${SHA_DMG}\"/"   "$CASK"

# ── 커밋 + 푸시 ───────────────────────────────────────────────────────────────
cd "$TAP_DIR"
git config user.name  "github-actions[bot]"
git config user.email "github-actions[bot]@users.noreply.github.com"
git add Formula/nexus.rb Casks/obsidian-nexus.rb
git commit -m "chore: bump to ${TAG}"
git push

echo "✓ homebrew-nexus tap updated to ${TAG}"
