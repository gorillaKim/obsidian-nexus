---
title: "ADR-011: Agent SDK용 MCP 서버는 항상 앱 번들 경로 사용"
aliases:
  - ADR-011
  - agent-mcp-bundle-path
  - Agent SDK MCP 경로
  - mcp-bundle-path
  - 에이전트 MCP 번들 경로
created: 2026-03-26
updated: 2026-03-26
status: Accepted
tags:
  - architecture
  - decision
  - agent
  - macos
  - security
---

<!-- docsmith: auto-generated 2026-03-26 -->

# ADR-011: Agent SDK용 MCP 서버는 항상 앱 번들 경로 사용

## 상태

채택 (Accepted)

## 배경

`crates/agent`의 `which_nexus_mcp()` 함수는 Agent SDK 세션에서 사용할 MCP 서버 바이너리 경로를 반환한다.

`find_sidecar()` 함수에 mtime 기반 최신 바이너리 선택 로직을 추가한 이후, Agent SDK용 MCP 경로 선택에도 이 로직이 적용되어 `~/.local/bin/nexus-mcp-server`가 선택되는 상황이 발생했다.

macOS의 보안 정책상 서명된 앱이 서명 없는 바이너리를 subprocess로 실행할 경우 SIGKILL (exit 137)로 강제 종료된다. `~/.local/bin/` 복사본은 코드 사인이 없기 때문에 Agent SDK에서 실행이 불가능하다.

## 결정

`which_nexus_mcp()` 함수는 Agent SDK 세션의 MCP 서버 경로를 결정할 때, 항상 앱 번들 내 사이드카 경로(`/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server`)를 우선 탐색한다. `~/.local/bin/nexus-mcp-server`는 사용하지 않는다.

## 고려한 대안

| 옵션 | 문제 |
|------|------|
| `~/.local/bin/nexus-mcp-server` 사용 | 코드 사인 없음 → macOS SIGKILL (exit 137) |
| mtime 기반 최신 바이너리 선택 적용 | `~/.local/bin/` 복사본이 더 최신일 경우 서명 없는 바이너리 선택 위험 |
| 앱 번들 사이드카 고정 사용 | Tauri 빌드 시 코드 사인 포함 → macOS 보안 정책 통과 |

## 결정 이유

앱 번들 내 사이드카(`Contents/MacOS/nexus-mcp-server`)는 Tauri 빌드 및 서명 과정에서 함께 서명된다. 따라서 서명된 앱 컨텍스트에서 subprocess로 실행할 때 macOS Gatekeeper 및 코드 서명 검사를 통과한다.

`find_sidecar()`의 mtime 로직은 CLI 및 Claude Code Extension용 경로 탐색(서명 불필요)에만 적용되며, Agent SDK 경로에는 별도 함수(`which_nexus_mcp()`)를 통해 번들 경로를 고정한다.

## 결과

- `which_nexus_mcp()`: 앱 번들 경로(`/Applications/Obsidian Nexus.app/...`) 직접 탐색 → 없을 경우 `~/.local/bin/` fallback
- `find_sidecar()` mtime 로직: CLI / Claude Code Extension용 경로 탐색에만 적용
- Agent SDK MCP 서버는 앱 업데이트 시 자동으로 최신 서명 바이너리로 갱신됨

## 참조

- 커밋 `55d4b27` fix(agent): always use app bundle MCP server for Agent SDK sessions
- [[2026-03-26-mcp-binary-path-confusion]]

## 관련 문서

- [[2026-03-26-mcp-binary-path-confusion]]
- [[ADR-010: 임베딩 컨텍스트 Prefix — 프로젝트명+태그로 Hub Vector 차단]]
