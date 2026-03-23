---
title: Claude & Gemini CLI 자동 감지
aliases:
  - cli-detection
  - claude-gemini-cli-detection
  - cli 자동감지
  - claude gemini 연동
created: 2026-03-23
updated: 2026-03-23
tags:
  - integration
  - claude-cli
  - gemini-cli
  - cli-detection
  - agent
---

<!-- docsmith: auto-generated 2026-03-23 -->

# Claude & Gemini CLI 자동 감지

## 개요

obsidian-nexus 데스크탑 앱(Tauri)은 사서(librarian) 기능을 위해 Claude CLI 또는 Gemini CLI를 필요로 합니다. GUI 앱은 셸 환경을 상속하지 않으므로 일반 `which` 명령으로는 nvm, volta, fnm 등 Node.js 버전 매니저를 통해 설치된 CLI를 찾지 못하는 문제가 있습니다.

`crates/agent/src/cli_detector.rs`가 이 문제를 해결합니다. 로그인 셸을 통한 탐색과 하드코딩된 fallback 경로 목록을 결합하여 어떤 설치 방식으로도 CLI를 안정적으로 찾아냅니다.

---

## 탐색 흐름

```
detect_agents()
  ├── detect_claude()
  │     └── find_cli_path("claude")
  │           ├── try_which()       # 1순위: 로그인 셸 위임
  │           └── 하드코딩 경로 순회  # 2순위: fallback
  └── detect_gemini()
        └── find_cli_path("gemini")
              ├── try_which()
              └── 하드코딩 경로 순회
```

`detect_agents()`는 탐지에 성공한 CLI를 `Vec<DetectedAgent>` 형태로 반환합니다. 각 항목에는 `path`, `version`, `authenticated`, `models` 필드가 포함됩니다.

---

## 경로 탐색 전략

### 1순위: 로그인 셸 위임 (`try_which`)

`$SHELL -l -c "which <name>"` 형태로 실행합니다. `-l` 플래그가 `.zshrc` / `.bashrc`를 완전히 로드하므로 사용자 PATH가 반영됩니다. 출력 경로가 비어있거나 명령이 실패하면 다음 단계로 넘어갑니다.

### 2순위: 하드코딩 fallback 경로

로그인 셸 탐색이 실패하면 아래 경로를 순서대로 확인합니다.

| 설치 방식 | 경로 |
|-----------|------|
| npm global (기본) | `~/.npm-global/bin/<name>` |
| npm global (대체) | `~/.npm/bin/<name>` |
| nvm | `~/.nvm/versions/node/*/bin/<name>` (glob) |
| fnm | `~/.local/share/fnm/node-versions/*/installation/bin/<name>` (glob) |
| volta | `~/.volta/bin/<name>` |
| n | `~/n/bin/<name>` |
| Homebrew (Apple Silicon) | `/opt/homebrew/bin/<name>` |
| Homebrew (Intel) | `/usr/local/bin/<name>` |
| nexus 심볼릭 링크 | `~/.local/bin/<name>` |

glob 패턴(`*`)이 포함된 경로는 `glob_first()`로 처리되며, 여러 버전이 존재하면 정렬 후 마지막 항목(가장 높은 버전)을 선택합니다.

---

## 인증 확인

CLI를 찾은 뒤 인증 상태를 확인합니다. 자격 증명 파일을 JSON으로 파싱하여 키 존재 여부만 검사하고, 토큰 값은 메모리에 유지하지 않습니다.

| CLI | 파일 | 확인 키 |
|-----|------|---------|
| Claude | `~/.claude/.credentials.json` | `claudeAiOauthTokenData` |
| Gemini | `~/.gemini/oauth_creds.json` | `access_token` |

`DetectedAgent.authenticated` 필드가 `false`이면 CLI는 찾았지만 로그인이 되어있지 않은 상태입니다.

---

## 최소 지원 버전

| CLI | 최소 버전 |
|-----|-----------|
| Claude CLI | 2.0.0 |
| Gemini CLI | 1.0.0 |

버전이 최솟값보다 낮으면 경고 로그(`warn!`)를 남기지만 탐지 자체는 계속 진행됩니다.

버전 파싱은 `get_cli_version()`이 담당하며 `--version` 출력에서 첫 번째 숫자로 시작하는 토큰을 추출합니다. 예: `"2.1.78 (Claude Code)"` → `"2.1.78"`.

---

## 지원 모델 목록

탐지 성공 시 `DetectedAgent.models`에 아래 값이 설정됩니다.

**Claude**
- `sonnet`
- `opus`
- `haiku`

**Gemini**
- `gemini-2.5-pro`
- `gemini-2.5-flash`

---

## 주요 타입

```rust
pub struct DetectedAgent {
    pub cli: CliType,        // Claude | Gemini
    pub path: PathBuf,       // 실행 파일 절대 경로
    pub version: String,     // 파싱된 버전 문자열
    pub authenticated: bool, // 자격 증명 파일 유효 여부
    pub models: Vec<String>, // 사용 가능한 모델 목록
}
```

---

## 트러블슈팅

### CLI가 탐지되지 않는 경우

1. 터미널에서 `which claude` 또는 `which gemini`가 동작하는지 확인합니다.
2. 동작한다면 로그인 셸(`$SHELL -l -c "which claude"`)로도 확인합니다.
3. 여전히 실패하면 해당 CLI의 실제 경로를 `~/.local/bin/`에 심볼릭 링크로 추가합니다.

```bash
ln -s $(which claude) ~/.local/bin/claude
```

### `authenticated: false`인 경우

- **Claude**: `claude` 명령어로 로그인 후 `~/.claude/.credentials.json`이 생성되었는지 확인합니다.
- **Gemini**: `gemini` 명령어로 OAuth 인증 후 `~/.gemini/oauth_creds.json`이 생성되었는지 확인합니다.

### 버전 경고가 나타나는 경우

최소 버전 미만이면 일부 기능이 동작하지 않을 수 있습니다. 패키지 매니저로 업데이트합니다.

```bash
npm update -g @anthropic-ai/claude-code   # Claude CLI
npm update -g @google/gemini-cli          # Gemini CLI
```

---

## Shebang 기반 실행 가능 여부 검증 (`is_executable_script`)

nvm/volta로 설치된 CLI는 Node.js 스크립트다. 파일 자체는 존재하지만 shebang이 가리키는 interpreter가 없으면 실행 시 ENOENT(os error 2)가 발생한다.

`is_executable_script(path)` 함수는 프로세스를 실행하지 않고 shebang만 읽어 유효성을 판단한다:

| shebang 유형 | 판단 기준 |
|---|---|
| 없음 (네이티브 바이너리) | 항상 true |
| `#!/absolute/path` | interpreter 경로가 파일시스템에 존재하면 true |
| `#!/usr/bin/env node` | 스크립트와 같은 `bin/` 디렉토리에 `node`가 있으면 true (nvm 패턴) |
| `#!/usr/bin/env -S node --flags` | 위와 동일 (`-S` 플래그 처리 포함) |

```rust
// 핵심 로직 요약
if interpreter == "/usr/bin/env" {
    // 1. 스크립트 같은 디렉토리에 node가 있는가? (nvm/volta 패턴)
    if parent.join(cmd).exists() { return true; }
    // 2. 현재 PATH에 node가 있는가?
    for p in PATH { if p.join(cmd).exists() { return true; } }
}
```

이 필터는 `try_which`, `glob_first`, `find_cli_path`의 fallback 순회, 그리고 `test_cli` 핸들러 모두에 적용된다.

---

## PATH Enrichment (nvm/volta 실행 시 필수)

shebang 검증을 통과한 경로를 실제 실행할 때도 PATH 보강이 필요하다. `#!/usr/bin/env node`는 실행 시점에 PATH에서 `node`를 찾기 때문이다.

```rust
// get_cli_version 및 test_cli 핸들러에서 적용
let enriched_path = format!("{}:{}", binary_parent_dir, current_PATH);
Command::new(binary_path).env("PATH", enriched_path)
```

`binary_parent_dir`은 nvm의 경우 `~/.nvm/versions/node/v20.20.0/bin/`이며,
이 디렉토리에 `node`가 함께 설치되어 있어 interpreter 해석이 성공한다.

---

## GUI 앱 PATH 문제 요약

macOS GUI 앱(Tauri, Electron 등)은 로그인 셸 초기화 없이 실행되므로 `~/.zshrc`, `~/.bash_profile`에서 설정한 PATH가 반영되지 않는다.

```
터미널: /Users/madup/.nvm/versions/node/v20.20.0/bin:/opt/homebrew/bin:...
GUI 앱:  /usr/bin:/bin:/usr/sbin:/sbin  ← nvm, homebrew 모두 없음
```

**해결 전략 (우선순위 순)**:

1. `$SHELL -l -c "which <name>"` — 로그인 셸에 위임해 사용자 PATH 탐색 (느림, `.zshrc` 로딩 포함)
2. `~/.nvm/versions/node/*/bin/<name>` glob 탐색 — nvm 특화, 빠름
3. `~/.local/bin/<name>` 심볼릭 링크 — 수동 등록, 항상 동작

실행 시 항상 바이너리 parent dir를 PATH에 prepend할 것.

---

## 관련 파일

- `crates/agent/src/cli_detector.rs` — 탐지 로직 전체
- `apps/desktop/src-tauri/src/main.rs` — Tauri 앱에서 호출

---

## 관련 문서

- [[ollama-setup]]
- [[mcp-tools]]
- [[subagent-mcp-setup]]
