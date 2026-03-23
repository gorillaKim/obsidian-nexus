---
title: 설치 가이드 (Installation Guide)
aliases:
  - installation-guide
  - install
  - 설치가이드
  - 설치방법
tags:
  - guide
  - installation
  - setup
  - homebrew
created: 2026-03-23
updated: 2026-03-23
---
<!-- docsmith: auto-generated 2026-03-23 -->

# 설치 가이드 (Installation Guide)

Obsidian Nexus v0.3.12 — 데스크톱 앱, CLI, MCP 서버의 설치 방법을 방법별로 상세히 정리한 레퍼런스 문서입니다.

빠른 온보딩이 목적이라면 [[시작하기 (Getting Started)]]를 먼저 확인하세요.

---

## 설치 방법 비교

| 방법 | 설치 대상 | 권장 대상 |
|------|-----------|-----------|
| **Homebrew (Formula)** | `obs-nexus` CLI + `nexus-mcp-server` | CLI/MCP만 사용하는 개발자 |
| **Homebrew (Cask)** | 데스크톱 앱 `.app` | GUI를 선호하는 일반 사용자 |
| **데스크톱 앱 (.dmg)** | 데스크톱 앱 + CLI + MCP 서버 (사이드카 번들) | 처음 사용하는 모든 사용자 |
| **cargo install** | `obs-nexus` CLI + `nexus-mcp-server` | Rust 개발자, 소스 빌드 선호 |
| **수동 빌드** | 전체 워크스페이스 | 기여자, 포크 사용자 |

---

## Homebrew로 설치 (권장)

### CLI + MCP 서버 (Formula)

```bash
brew tap gorillaKim/nexus
brew install gorillaKim/nexus/obsidian-nexus
```

설치되는 바이너리:

| 바이너리 | 역할 |
|----------|------|
| `obs-nexus` | Nexus CLI (`nexus` 명령어 진입점) |
| `nexus-mcp-server` | MCP 서버 (stdin/stdout JSON-RPC) |

설치 확인:

```bash
obs-nexus --version
nexus-mcp-server --version
```

### 데스크톱 앱 (Cask)

```bash
brew install --cask gorillaKim/nexus/obsidian-nexus
```

`Obsidian Nexus.app`이 `/Applications`에 설치됩니다.

### 업그레이드

```bash
brew upgrade obsidian-nexus          # Formula (CLI)
brew upgrade --cask obsidian-nexus   # Cask (데스크톱 앱)
```

릴리즈 시 Homebrew tap(`gorillaKim/nexus`)이 자동으로 업데이트됩니다.

---

## 데스크톱 앱 설치

### 다운로드

[GitHub Releases](https://github.com/gorillaKim/obsidian-nexus/releases/latest)에서 플랫폼에 맞는 파일을 다운로드합니다.

| 파일 | 플랫폼 |
|------|--------|
| `Obsidian-Nexus.dmg` | macOS (Universal — Apple Silicon + Intel) |

> Windows 및 Linux 패키지는 현재 지원 예정입니다. (`release.yml` CI matrix 확장 계획 중)

### macOS 설치 절차

1. `Obsidian-Nexus.dmg`를 열고 `Obsidian Nexus.app`을 `/Applications`로 드래그합니다.
2. 첫 실행 시 macOS Gatekeeper 경고가 나타날 수 있습니다. 아래 명령어로 격리 속성을 해제합니다.

```bash
xattr -cr "/Applications/Obsidian Nexus.app"
```

### 데스크톱 앱이 번들하는 항목

`tauri.conf.json`의 `bundle.externalBin` 설정에 따라 다음이 앱 번들 내에 포함됩니다.

| 항목 | 내용 |
|------|------|
| `obs-nexus` | Nexus CLI 바이너리 (사이드카) |
| `nexus-mcp-server` | MCP 서버 바이너리 (사이드카) |
| `claude-bridge.mjs` | Claude 브릿지 스크립트 (Node.js 사이드카) |

데스크톱 앱을 설치하면 별도의 CLI/MCP 서버 설치 없이 모든 기능을 사용할 수 있습니다.

### 앱 최초 실행 시 자동 처리 항목

데스크톱 앱을 **처음 실행하면** `main()` 에서 다음을 자동으로 수행합니다 (`main.rs` 확인):

| 단계 | 함수 | 동작 |
|------|------|------|
| 1 | `ensure_obsidian()` | Obsidian 미설치 시 `brew install --cask obsidian` 자동 실행 (macOS only) |
| 2 | `register_mcp_server()` | Claude Desktop / Claude Code / Gemini CLI 설정 파일에 MCP 서버 자동 등록 |
| 3 | `install_cli_symlinks()` | `~/.local/bin/nexus`, `~/.local/bin/nexus-mcp-server` 심볼릭 링크 생성 + shell rc에 PATH 추가 |

> **참고**: Ollama는 자동 설치되지 않습니다. 벡터 검색을 사용하려면 별도 설치가 필요합니다. → [[Ollama 설치 및 임베딩 설정]] 참고

**MCP 자동 등록 대상** (설정 디렉토리가 이미 존재하는 경우에만 등록):
- Claude Desktop (`~/Library/Application Support/Claude/`)
- Claude Code (`~/.claude/`)
- Gemini CLI (`~/.gemini/`)

### 자동 업데이트

앱 시작 시 백그라운드에서 업데이트를 확인합니다.

- 업데이트 확인 endpoint: `https://github.com/gorillaKim/obsidian-nexus/releases/latest/download/latest.json`
- 다운로드된 번들은 minisign 공개키로 서명이 검증됩니다.
- 업데이트 적용 후 자동 재시작됩니다.
- 설정 탭에서 수동으로 "업데이트 확인" 버튼을 눌러 즉시 확인할 수도 있습니다.

---

## CLI만 단독 설치

### cargo install (crates.io 미등록 — 소스에서 설치)

저장소를 클론한 뒤 설치합니다.

```bash
git clone https://github.com/gorillaKim/obsidian-nexus.git
cd obsidian-nexus
cargo install --path crates/cli
```

`obs-nexus` 바이너리가 `~/.cargo/bin/`에 설치됩니다.

### 수동 빌드

```bash
# 요구사항: Rust 1.75+
cargo build --release -p nexus-cli -p nexus-mcp-server

# 바이너리 위치
./target/release/obs-nexus
./target/release/nexus-mcp-server
```

빌드된 바이너리를 PATH에 추가합니다.

```bash
cp target/release/obs-nexus /usr/local/bin/
cp target/release/nexus-mcp-server /usr/local/bin/
```

---

## MCP 서버 설치 및 설정

### 자동 설치 (데스크톱 앱 또는 Homebrew)

데스크톱 앱 또는 Homebrew로 설치하면 `nexus-mcp-server`가 함께 설치됩니다. 별도 설치가 필요하지 않습니다.

MCP 서버 바이너리 경로 확인:

```bash
which nexus-mcp-server
```

### Claude Desktop / Claude Code에 등록

#### 자동 등록 (권장)

프로젝트 디렉토리에서 다음 명령어를 실행하면 `.mcp.json`과 librarian 에이전트 파일이 자동 생성됩니다.

```bash
nexus onboard
# 또는 경로 지정
nexus onboard /path/to/my-project
```

세션을 재시작하면 MCP 서버가 활성화됩니다.

#### 수동 등록

프로젝트 루트에 `.mcp.json` 파일을 생성합니다.

```json
{
  "mcpServers": {
    "nexus": {
      "type": "stdio",
      "command": "/usr/local/bin/nexus-mcp-server",
      "args": []
    }
  }
}
```

`command` 값을 `which nexus-mcp-server` 출력으로 대체하세요.

### MCP 서버 프로토콜

MCP 서버는 **stdin/stdout JSON-RPC** 프로토콜을 사용합니다. 외부 포트를 열지 않으며, MCP 클라이언트(Claude Desktop, Claude Code 등)가 프로세스를 직접 관리합니다.

제공하는 도구 목록:

| 도구 | 기능 |
|------|------|
| `nexus_search` | 하이브리드/키워드/벡터 검색 |
| `nexus_get_document` | 문서 전체 내용 조회 |
| `nexus_get_section` | 특정 섹션만 조회 (토큰 절약) |
| `nexus_get_metadata` | 프론트매터, 태그, 인덱싱 상태 |
| `nexus_get_backlinks` | 역방향 링크 조회 |
| `nexus_get_links` | 순방향 링크 조회 |
| `nexus_list_projects` | 등록된 볼트 목록 |
| `nexus_index_project` | 증분/전체 재인덱싱 트리거 |
| `nexus_onboard` | librarian 설정 자동 생성 |
| `nexus_status` | 시스템 상태 확인 (Ollama, DB) |

---

## 설치 후 초기 설정

### 1. Ollama 설치 (벡터 검색용)

벡터 검색 및 하이브리드 검색을 사용하려면 Ollama가 필요합니다. 키워드 검색만 사용할 경우 생략 가능합니다.

```bash
# Ollama 설치 (macOS)
brew install ollama

# nomic-embed-text 모델 다운로드 (768차원)
ollama pull nomic-embed-text

# Ollama 서버 실행
ollama serve
```

> Ollama가 실행 중이지 않으면 벡터/하이브리드 검색은 비활성화되고 키워드 검색만 동작합니다.

### 2. 초기 설정

데이터베이스 초기화 및 의존성 확인:

```bash
obs-nexus setup
```

### 3. 볼트 등록

Obsidian 볼트 또는 마크다운 디렉토리를 프로젝트로 등록합니다.

```bash
obs-nexus project add --name my-notes --path ~/Documents/my-obsidian-vault

# 등록 확인
obs-nexus project list
```

### 4. 인덱싱

```bash
obs-nexus index my-notes

# 전체 재인덱싱
obs-nexus index my-notes --full
```

---

## 업데이트

### Homebrew

```bash
brew upgrade obsidian-nexus
```

릴리즈 태그 푸시 시 CI가 tap을 자동으로 갱신하므로 별도 작업 없이 최신 버전을 설치할 수 있습니다.

### 데스크톱 앱

앱 시작 시 자동으로 업데이트를 확인합니다. 설정 탭에서 수동 확인도 가능합니다.

### CLI (자가 업데이트)

```bash
obs-nexus update          # 확인 + 설치
obs-nexus update --check  # 확인만 (설치하지 않음)
obs-nexus update --force  # 24시간 캐시 무시하고 강제 확인
```

GitHub Release API로 최신 버전을 확인하고, SHA256 체크섬을 검증한 뒤 atomic 교체합니다.

### cargo (소스 빌드)

```bash
cd obsidian-nexus
git pull
cargo install --path crates/cli --force
```

---

## 관련 문서

- [[시작하기 (Getting Started)]]
- [[배포 및 버전 관리]]
- [[05-설정-가이드]]
- [[03-MCP-도구-레퍼런스]]
