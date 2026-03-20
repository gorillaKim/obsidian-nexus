#!/usr/bin/env bash
# Obsidian Nexus installer
# Usage: curl -fsSL https://raw.githubusercontent.com/gorillaKim/obsidian-nexus/master/install.sh | bash

set -euo pipefail

REPO="gorillaKim/obsidian-nexus"
INSTALL_DIR="${NEXUS_INSTALL_DIR:-$HOME/.local/bin}"
BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
RESET='\033[0m'

info()    { echo -e "${BOLD}[nexus]${RESET} $*"; }
success() { echo -e "${GREEN}✓${RESET} $*"; }
warn()    { echo -e "${YELLOW}⚠${RESET}  $*"; }
error()   { echo -e "${RED}✗${RESET} $*" >&2; exit 1; }

# ── OS / arch detection ──────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

[[ "$OS" != "Darwin" ]] && error "현재 macOS만 지원합니다. (검출된 OS: $OS)"

case "$ARCH" in
  arm64)   ARCH_TAG="aarch64" ;;
  x86_64)  ARCH_TAG="x86_64" ;;
  *)       error "지원하지 않는 아키텍처: $ARCH" ;;
esac

TARBALL="nexus-cli-darwin-${ARCH_TAG}.tar.gz"
CHECKSUM="${TARBALL}.sha256"

# ── Latest version detection ─────────────────────────────────────────────────
info "최신 버전 확인 중..."
if command -v curl &>/dev/null; then
  FETCH="curl -fsSL"
elif command -v wget &>/dev/null; then
  FETCH="wget -qO-"
else
  error "curl 또는 wget이 필요합니다."
fi

LATEST=$($FETCH "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

[[ -z "$LATEST" ]] && error "버전 정보를 가져올 수 없습니다. 네트워크 연결을 확인하세요."

info "설치 버전: ${BOLD}${LATEST}${RESET} (aarch64: $ARCH_TAG)"

BASE_URL="https://github.com/${REPO}/releases/download/${LATEST}"

# ── Download ─────────────────────────────────────────────────────────────────
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

info "다운로드 중: $TARBALL"
$FETCH "${BASE_URL}/${TARBALL}"    > "${TMP_DIR}/${TARBALL}"
$FETCH "${BASE_URL}/${CHECKSUM}"   > "${TMP_DIR}/${CHECKSUM}"

# ── SHA256 verification ───────────────────────────────────────────────────────
info "체크섬 검증 중..."
pushd "$TMP_DIR" > /dev/null
if command -v shasum &>/dev/null; then
  shasum -a 256 -c "$CHECKSUM" --status || error "체크섬 불일치! 파일이 손상되었을 수 있습니다."
elif command -v sha256sum &>/dev/null; then
  sha256sum -c "$CHECKSUM" --status || error "체크섬 불일치! 파일이 손상되었을 수 있습니다."
else
  warn "shasum/sha256sum 없음 — 체크섬 검증을 건너뜁니다."
fi
popd > /dev/null
success "체크섬 검증 완료"

# ── Install ───────────────────────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR"
tar xzf "${TMP_DIR}/${TARBALL}" -C "$TMP_DIR"

for BIN in obs-nexus nexus-mcp-server; do
  if [[ -f "${TMP_DIR}/${BIN}" ]]; then
    install -m 755 "${TMP_DIR}/${BIN}" "${INSTALL_DIR}/${BIN}"
    success "${BIN} → ${INSTALL_DIR}/${BIN}"
  else
    warn "${BIN} 바이너리를 찾을 수 없습니다."
  fi
done

# ── PATH check ────────────────────────────────────────────────────────────────
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  warn "${INSTALL_DIR} 가 PATH에 없습니다. 아래 줄을 쉘 설정 파일에 추가하세요:"
  echo ""
  echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
  echo ""

  SHELL_RC=""
  case "${SHELL:-}" in
    */zsh)  SHELL_RC="$HOME/.zshrc" ;;
    */bash) SHELL_RC="$HOME/.bashrc" ;;
  esac

  if [[ -n "$SHELL_RC" ]]; then
    read -r -p "  지금 바로 ${SHELL_RC}에 추가할까요? [Y/n] " REPLY
    REPLY="${REPLY:-Y}"
    if [[ "$REPLY" =~ ^[Yy]$ ]]; then
      echo '' >> "$SHELL_RC"
      echo '# Obsidian Nexus' >> "$SHELL_RC"
      echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$SHELL_RC"
      success "${SHELL_RC} 에 PATH 추가 완료 (새 터미널에서 적용됨)"
    fi
  fi
fi

# ── Next steps ────────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}━━━ 설치 완료! 다음 단계를 따라주세요 ━━━${RESET}"
echo ""
echo "  1. 초기 설정 (Ollama 확인, DB 초기화)"
echo "     ${BOLD}nexus setup${RESET}"
echo ""
echo "  2. Obsidian 볼트 등록"
echo "     ${BOLD}nexus project add --name \"my-vault\" --path /path/to/vault${RESET}"
echo ""
echo "  3. 문서 인덱싱"
echo "     ${BOLD}nexus index my-vault${RESET}"
echo ""
echo "  4. AI 에이전트(MCP) 연동"
echo "     ${BOLD}nexus onboard /path/to/project${RESET}"
echo ""
echo "  📱 Desktop 앱:"
echo "     https://github.com/${REPO}/releases/latest"
echo "     → Obsidian-Nexus.dmg 다운로드 후 설치"
echo ""
echo -e "${GREEN}${BOLD}nexus ${LATEST} 설치 완료!${RESET}"
