---
title: "nexus_get_toc MCP 미노출 및 업데이트 파이프라인 전체 수정"
aliases:
  - nexus-get-toc-mcp-update-fix
  - mcp-update-pipeline-fix
  - nexus_get_toc 미노출
  - 업데이트 파이프라인 수정
  - sidecar-binary-cache-fix
created: "2026-03-26"
updated: "2026-03-26"
tags:
  - devlog
  - troubleshooting
  - mcp
  - tauri
  - build
---

<!-- docsmith: auto-generated 2026-03-26 -->

# nexus_get_toc MCP 미노출 및 업데이트 파이프라인 전체 수정

`nexus_get_toc` 도구가 MCP에 노출되지 않는 문제를 추적하다가 업데이트 파이프라인 전반의 구조적 문제 4가지를 발견하고 전부 수정했다.

---

## 문제 1: Tauri 사이드카 바이너리 구버전 캐시

### 증상

MCP 클라이언트(Claude Desktop 등)에서 `nexus_get_toc` 도구가 보이지 않는다. 코드에는 이미 구현되어 있음.

### 원인

`apps/desktop/src-tauri/binaries/nexus-mcp-server-aarch64-apple-darwin` 파일이 Mar 20 구버전으로 남아 있었다. `pnpm tauri:build` 시 이 파일을 그대로 앱 번들에 포함하므로 `nexus_get_toc` 구현이 반영되지 않았다.

Tauri 사이드카 바이너리는 빌드 과정에서 **자동으로 갱신되지 않는다.** `cargo build` 를 아무리 해도 `binaries/` 디렉토리의 파일은 그대로다.

### 해결

```bash
# aarch64 타깃으로 MCP 서버 빌드
cargo build --release --target aarch64-apple-darwin -p nexus-mcp-server

# 사이드카 디렉토리에 수동 복사
cp target/aarch64-apple-darwin/release/nexus-mcp-server \
   apps/desktop/src-tauri/binaries/nexus-mcp-server-aarch64-apple-darwin
```

이후 `pnpm tauri:build` 로 앱 번들 재생성.

### 검증

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | apps/desktop/src-tauri/binaries/nexus-mcp-server-aarch64-apple-darwin
```

출력에서 `nexus_get_toc` 포함 여부 확인.

---

## 문제 2: `update.rs` 바이너리명 하드코딩 불일치

### 증상

앱 내 `nexus update` 명령 실행 시 CLI가 업데이트되지 않는다.

### 원인

`crates/cli/src/commands/update.rs:181` 에서 설치 대상 바이너리명을 `"nexus"` 로 하드코딩하고 있었다. 그런데 GitHub Actions 릴리즈 타르볼은 `obs-nexus` 로 패키징한다. 이름이 달라서 업데이트 시 CLI 교체가 일어나지 않았다.

### 해결

```rust
// before
let binary_name = "nexus";

// after
let binary_name = "obs-nexus";
```

---

## 문제 3: `which_nexus_mcp()` 잘못된 경로 선택

### 증상

Agent SDK 세션에서 MCP 서버가 exit 137(SIGKILL)로 즉시 종료된다.

### 원인

`which_nexus_mcp()` 함수가 `~/.local/bin/nexus-mcp-server` 를 Agent SDK 서버로 선택하고 있었다. 이 경로의 바이너리는 코드 서명이 없으므로 macOS Gatekeeper가 SIGKILL로 강제 종료한다.

앱 번들 경로(`find_sidecar()`)의 바이너리는 코드 서명이 되어 있어 정상 실행된다.

### 해결

`which_nexus_mcp()` 가 항상 앱 번들 경로를 반환하도록 수정. `~/.local/bin/` 경로 선택 로직 제거.

---

## 문제 4: `find_sidecar()` mtime 비교의 함정

### 증상

mtime 기준으로 최신 바이너리를 선택하는 로직이 오히려 서명 없는 바이너리를 우선 선택한다.

### 원인

`~/.local/bin/` 의 복사본은 나중에 복사되므로 mtime이 더 최신이다. mtime 비교 로직은 이 파일을 우선 선택하게 되어 있어 코드 서명 없는 바이너리가 선택된다.

### 해결

`which_nexus_mcp()` 에서 mtime 비교를 제거하고 앱 번들 경로를 직접 탐색하도록 고정.

---

## 문제 5: GitHub Actions 서명 키 base64 오류

### 증상

v0.5.7 릴리즈 GitHub Actions 빌드 실패:

```
failed to decode base64 secret key: Invalid padding
```

### 원인

`TAURI_SIGNING_PRIVATE_KEY` 시크릿이 base64 패딩이 깨진 상태로 등록되어 있었다.

### 해결

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/obsidian-nexus.key
```

파일을 직접 stdin으로 전달하여 줄바꿈/패딩 손상 없이 등록.

---

## 수정된 파일

| 파일 | 변경 내용 |
|------|-----------|
| `crates/cli/src/commands/update.rs` | `"nexus"` → `"obs-nexus"` 바이너리명 수정 |
| `apps/desktop/src-tauri/src/main.rs` | `find_sidecar()` mtime 로직 제거, `which_nexus_mcp()` 앱 번들 경로 고정 |
| `.claude/rules/build.md` | 사이드카 갱신 절차, DMG 설치 규칙 추가 |

## 관련 커밋

| 커밋 | 내용 |
|------|------|
| `8476aaf` | fix(update): prefer newer binary via mtime, fix CLI binary name |
| `55d4b27` | fix(agent): always use app bundle MCP server for Agent SDK sessions |
| `fd75f58` | chore(release): bump version to 0.5.8 |

---

## 사이드카 바이너리 갱신 체크리스트

MCP 서버 코드 변경 후 배포 전 반드시 수행:

- [ ] `cargo build --release --target aarch64-apple-darwin -p nexus-mcp-server`
- [ ] `binaries/` 디렉토리 수동 교체
- [ ] `tools/list` 응답으로 새 도구 포함 여부 검증
- [ ] `pnpm tauri:build` 로 앱 번들 재생성

---

## 교훈

- Tauri 사이드카 바이너리는 `cargo build` 와 무관하게 `binaries/` 디렉토리를 수동으로 갱신해야 한다
- macOS Agent SDK subprocess는 코드 서명된 바이너리만 허용 — `~/.local/bin/` 복사본은 서명이 깨짐
- `cp -R` 로 macOS 앱 설치 시 코드 서명 깨짐 — 반드시 DMG 마운트 후 Applications 드래그 설치 사용
- mtime 비교로 "최신" 바이너리를 선택하는 방식은 코드 서명을 고려하지 않아 역효과를 낼 수 있다
- GitHub Secrets에 멀티라인 값 등록 시 stdin 직접 전달 방식을 사용할 것 (`< file` 리다이렉션)

## 관련 문서

- [[2026-03-26-tauri-signing-key-troubleshooting]]
- [[2026-03-26-mcp-binary-path-confusion]]
- [[2026-03-25-mcp-update-asset-name-fix]]
