---
title: MCP 경로 불일치 및 CLI 감지 실패
date: 2026-03-23
created: 2026-03-23
updated: 2026-03-23
tags:
  - troubleshooting
  - bugfix
  - mcp
  - cli-detection
  - homebrew
aliases:
  - mcp 경로 문제
  - cli 감지 실패
  - agent detecting
  - mcp-path-mismatch
  - cli-detection-failure
---

<!-- docsmith: auto-generated 2026-03-23 -->

## 증상

다른 컴퓨터에서 `brew install --cask gorillaKim/nexus/obsidian-nexus`로 설치 후 다음 증상이 발생했다.

1. 앱 채팅창이 "에이전트 감지 중" 상태에서 멈춤
2. "CLI 에이전트를 찾을 수 없습니다" 메시지 표시
3. `claude mcp list` 실행 결과: `No MCP servers configured` — settings.json에는 분명히 등록돼 있음

## 원인 분석

### 원인 1: MCP 경로 불일치 (Race Condition)

앱 최초 실행 시 초기화 순서가 문제였다.

```
register_mcp_server()  // 번들 경로로 등록
install_cli_symlinks() // 그 다음 ~/.local/bin 심링크 생성
```

`register_mcp_server()`가 먼저 실행되어 번들 내부 경로
`/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server`가
settings.json에 등록됐다. Claude Code는 **공백 포함 경로를 인식하지 못해**
`claude mcp list`에 등록 항목이 나타나지 않았다.

### 원인 2: GUI 앱의 PATH 제한

Tauri GUI 앱은 shell을 상속하지 않아 `which claude`가 실패했다.
nvm으로 설치된 Claude CLI(`~/.nvm/versions/node/v24.11.1/bin/claude`)를
찾지 못하는 것이 근본 원인이었다.

## 해결책

### Fix 1: `main.rs` 실행 순서 변경

심링크를 먼저 생성한 뒤 MCP 서버를 등록하도록 순서를 바꿨다.

```rust
// 변경 전
register_mcp_server();
install_cli_symlinks();

// 변경 후
install_cli_symlinks();  // 먼저 심링크 생성
register_mcp_server();   // ~/.local/bin 경로로 등록
```

### Fix 2: `register_in_config()` 경로 유효성 검사

기존에 등록된 경로가 실제로 존재하지 않으면 덮어쓰도록 수정했다.
이미 잘못된 경로로 등록된 기존 사용자도 앱 재실행 시 자동으로 수정된다.

### Fix 3: `try_which()` login shell 사용

```rust
// 변경 전
Command::new("which").arg(name)

// 변경 후
Command::new(&shell).args(["-l", "-c", &format!("which {}", name)])
```

`-l` 플래그로 `.zshrc`를 로드하여 nvm / volta / fnm 등의 PATH가 포함되도록 했다.

## 수정 파일

- `apps/desktop/src-tauri/src/main.rs`
- `crates/agent/src/cli_detector.rs`

## 교훈

- GUI 앱 초기화 순서는 의존 관계를 명시적으로 정의해야 한다. 암묵적 순서 의존은 다른 환경에서 쉽게 깨진다.
- macOS Tauri 앱은 shell 환경을 상속하지 않는다. nvm/volta/fnm 같은 PATH 확장 도구를 사용하는 CLI를 탐지할 때는 login shell(`-l`)을 통해 실행해야 한다.
- 공백 포함 경로는 외부 도구(Claude Code 등)가 파싱하지 못할 수 있다. 가능하면 공백 없는 경로(심링크 등)를 등록하는 것이 안전하다.

## 관련 문서

- [[getting-started]]
- [[2026-03-19]]
