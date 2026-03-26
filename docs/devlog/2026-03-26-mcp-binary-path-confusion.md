---
title: "MCP 바이너리 배포 경로 혼선 및 해결"
aliases:
  - mcp-binary-path-confusion
  - mcp binary path
  - mcp 경로 혼선
  - nexus 배포
created: "2026-03-26"
updated: "2026-03-26"
tags:
  - devlog
  - troubleshooting
  - mcp
  - deployment
  - onboard
---

<!-- docsmith: auto-generated 2026-03-26 -->

## 개요

새 MCP 도구(`nexus_get_toc`)를 추가하고 빌드·릴리즈했는데 데스크톱 앱 에이전트가 "도구가 없다"고 계속 답변하는 문제가 발생했다. 원인은 MCP 바이너리가 여러 경로에 독립적으로 존재하는 구조였다.

## 발생한 문제

새 MCP 도구(`nexus_get_toc`)를 추가하고 빌드·릴리즈했는데 데스크톱 앱 에이전트가 "도구가 없다"고 계속 답변.

## 원인 분석

MCP 바이너리가 여러 경로에 독립적으로 존재함:

| 경로 | 관리 주체 | 용도 |
|------|----------|------|
| `/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server` | 앱 업데이트 버튼 | 데스크톱 앱 내부 사이드카 |
| `~/.local/bin/nexus-mcp-server` | nexus_onboard 설치 | Claude Code MCP 등록 경로 |
| `target/release/nexus-mcp-server` | cargo build | 로컬 빌드 산출물 |

앱 업데이트 버튼은 앱 번들만 교체하므로, Claude Code가 바라보는 `~/.local/bin`은 구버전이 그대로 유지된다.

## 해결 과정

1. `ps aux`로 실행 중인 nexus-mcp-server 프로세스 경로 확인
2. `~/.claude/claude_desktop_config.json`이 `~/.local/bin/nexus-mcp-server`를 바라보고 있음 확인
3. 설정을 앱 번들 경로로 변경: `/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server`
4. 이후 앱 업데이트 버튼 → Claude Code까지 자동 갱신되는 구조 완성

## 개선 사항

- `nexus_onboard`가 `current_exe()` 기반으로 경로 결정 → 앱 번들에서 실행 시 올바른 경로 자동 등록 (이미 올바른 구조)
- `NEXUS_TOOLS` 목록에 `nexus_get_toc` 추가 누락 → 수정 완료
- Claude Code MCP config를 앱 번들 바이너리로 통일 → 이후 앱 업데이트 버튼 한 번으로 모든 경로 갱신

## 교훈

새 도구 추가 시 체크리스트:

1. `crates/mcp-server/src/main.rs` — 스키마 + 디스패치 + 핸들러
2. `crates/core/src/search.rs` — 코어 함수
3. `crates/core/src/onboard.rs` — `NEXUS_TOOLS` 목록
4. 사서 프롬프트 (`app-guide.md`, `system.md`)
5. 빌드 후 tools/list로 실제 노출 검증

배포 후 검증 명령어:

```bash
# 바이너리 직접 검증
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | /Applications/Obsidian\ Nexus.app/Contents/MacOS/nexus-mcp-server

# Claude Code가 바라보는 바이너리 확인
cat ~/.claude/claude_desktop_config.json | grep mcp-server
```

앱 업데이트 버튼은 앱 번들만 교체하므로 Claude Code MCP 설정은 반드시 앱 번들 경로를 직접 지정해야 한다.

## 관련 문서

- [[MCP 경로 불일치 수정]]
- [[온보딩 버튼]]
- [[Tauri 서명키 관리 및 빌드 트러블슈팅]]
