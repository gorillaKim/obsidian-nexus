---
title: Tauri 사이드카 MCP 바이너리 업데이트 누락 문제
aliases:
  - tauri-sidecar-mcp-binary-update
  - sidecar-binary-cache-issue
  - Tauri 사이드카 바이너리 누락
  - MCP 바이너리 업데이트 누락
created: 2026-03-26
updated: 2026-03-26
tags:
  - devlog
  - troubleshooting
  - tauri
  - mcp
---

<!-- docsmith: auto-generated 2026-03-26 -->

## 문제 요약

`nexus_get_toc` 도구가 MCP 서버에 등록되어 있지 않다는 오류가 반복 발생.
소스코드에는 구현되어 있었지만 실제 실행 바이너리에는 반영되지 않았음.

## 근본 원인

### 원인 1: Tauri 사이드카 바이너리가 구버전 캐시 사용

- `apps/desktop/src-tauri/binaries/nexus-mcp-server-aarch64-apple-darwin` 파일이 Mar 20 빌드
- `pnpm tauri:build` 시 이 파일을 그대로 앱 번들에 포함 → 새 기능 미반영
- **교훈**: Tauri 빌드 전 반드시 `apps/desktop/src-tauri/binaries/` 사이드카 업데이트 필요

### 원인 2: `~/.local/bin/obs-nexus`가 구버전 (0.3.7)

- `update.rs`에서 설치 대상 바이너리명이 `"nexus"`로 하드코딩 → 릴리즈 타르볼의 `obs-nexus`와 불일치
- `obs-nexus update`를 실행해도 CLI가 업데이트되지 않았음
- **교훈**: 업데이트 스크립트의 바이너리명과 실제 릴리즈 아티팩트명이 일치해야 함

### 원인 3: `which_nexus_mcp()` 함수가 `~/.local/bin/` 우선 선택

- Agent SDK용 MCP 서버로 `~/.local/bin/nexus-mcp-server`를 선택
- 해당 바이너리는 코드 사인이 없어 macOS가 SIGKILL (exit 137)
- 앱 번들 내 사이드카는 코드 사인되어 있어 정상 동작
- **교훈**: Agent SDK subprocess로 실행되는 바이너리는 반드시 코드 사인된 앱 번들 경로 사용

### 원인 4: `find_sidecar()` mtime 비교의 함정

- `~/.local/bin/`의 수동 복사본이 앱 번들보다 mtime이 최신 → 잘못된 바이너리 선택
- **교훈**: mtime 비교는 "어느 것이 더 최신인가"를 판별하지만 "어느 것이 더 신뢰할 수 있는가"는 판별 못함

## 해결 방법

1. **`update.rs`**: `"nexus"` → `"obs-nexus"` 수정 (GitHub Actions 릴리즈 타르볼명과 일치)
2. **`find_sidecar()`**: mtime 기반 최신 바이너리 선택 로직 추가 (CLI/Claude Code용)
3. **`which_nexus_mcp()`**: 항상 앱 번들 경로 반환하도록 고정 (Agent SDK용)
4. **사이드카 업데이트**: `cargo build --release --target aarch64-apple-darwin -p nexus-mcp-server` 후 `binaries/` 복사 → Tauri 재빌드

## 재발 방지 체크리스트

- [ ] 새 MCP 도구 추가 시: `crates/mcp-server/src/main.rs` 핸들러 등록 확인
- [ ] 버전 bump 시: `apps/desktop/src-tauri/binaries/` 사이드카도 반드시 갱신
- [ ] 릴리즈 전: 앱 번들 MCP 서버에 새 도구가 포함됐는지 `tools/list` 로 검증
- [ ] `update.rs` 바이너리명 ↔ GitHub Actions 아티팩트명 일치 여부 확인

## 관련 커밋

- `8476aaf` fix(update): prefer newer binary via mtime, fix CLI binary name
- `55d4b27` fix(agent): always use app bundle MCP server for Agent SDK sessions

## 관련 문서

- [[2026-03-26-tauri-signing-key-troubleshooting]]
- [[2026-03-26-mcp-binary-path-confusion]]
