# Obsidian Nexus

> v0.3.12 · macOS (Apple Silicon / Intel)

Agent-friendly knowledge search engine for Obsidian vaults.

여러 Obsidian 볼트의 문서를 인덱싱하고, AI 에이전트가 MCP 프로토콜로 검색·탐색할 수 있는 로컬 지식 검색 엔진입니다.

## Features

- **하이브리드 검색** — FTS5 키워드 + sqlite-vec 벡터 시맨틱 검색 + RRF 리랭킹
- **멀티 볼트** — 여러 Obsidian 볼트를 등록하고 통합 검색
- **MCP 서버** — Claude, Gemini 등 AI 에이전트가 문서를 직접 검색/읽기
- **AI 사서** — 앱 내장 AI 채팅으로 자연어 질문→문서 검색·요약
- **Desktop 앱** — Tauri v2 기반 GUI (검색, 프로젝트 관리, 자동 업데이트)
- **CLI** — 터미널에서 인덱싱, 검색, 볼트 관리
- **Alias 검색** — 한글 별칭으로 영문 문서 검색 (예: "데이터독" → datadog-setup.md)
- **자동 업데이트** — Desktop은 Tauri updater, CLI는 `obs-nexus update`

---

## Installation

### 방법 1: curl 스크립트 (권장 — CLI + MCP 서버)

```bash
curl -fsSL https://raw.githubusercontent.com/gorillaKim/obsidian-nexus/master/install.sh | bash
```

- 아키텍처 자동 감지 (Apple Silicon / Intel)
- `obs-nexus` + `nexus-mcp-server` → `~/.local/bin` 설치
- SHA256 체크섬 검증
- PATH 자동 설정

> 설치 디렉토리 변경: `NEXUS_INSTALL_DIR=/usr/local/bin curl -fsSL ... | bash`

---

### 방법 2: Homebrew

> macOS Command Line Tools 최신 버전 필요. `xcode-select --install` 으로 설치.

```bash
brew tap gorillakim/nexus
brew install --formula gorillakim/nexus/obsidian-nexus   # CLI + MCP 서버
brew install --cask gorillakim/nexus/obsidian-nexus      # Desktop 앱
```

업데이트:
```bash
brew upgrade --formula gorillakim/nexus/obsidian-nexus
brew upgrade --cask gorillakim/nexus/obsidian-nexus
```

---

### 방법 3: Desktop 앱 수동 설치

1. [Releases 페이지](https://github.com/gorillaKim/obsidian-nexus/releases/latest)에서 `Obsidian-Nexus.dmg` 다운로드
2. DMG 열기 → Applications 폴더로 드래그
3. 앱 실행

**"앱이 손상되었습니다" 오류가 뜨면** (macOS Gatekeeper 미서명 차단):

```bash
xattr -cr /Applications/Obsidian\ Nexus.app
```

> Desktop 앱 안에 CLI와 MCP 서버가 내장되어 있습니다. CLI만 필요하면 방법 1로 충분합니다.

#### 데스크톱 앱 최초 실행 시 자동 처리

앱을 처음 실행하면 다음을 자동으로 수행합니다:

| 단계 | 동작 |
|------|------|
| Obsidian 설치 | 미설치 시 `brew install --cask obsidian` 자동 실행 |
| MCP 서버 등록 | Claude Desktop / Claude Code / Gemini CLI 설정에 자동 등록 |
| CLI PATH 설정 | `~/.local/bin/nexus` 심볼릭 링크 생성 + shell rc에 PATH 추가 |

> Ollama는 자동 설치되지 않습니다. 벡터 검색을 사용하려면 별도 설치가 필요합니다 (`obs-nexus setup` 실행 시 안내).

---

### 방법 4: CLI 수동 설치

```bash
# Apple Silicon
curl -fsSL https://github.com/gorillaKim/obsidian-nexus/releases/latest/download/nexus-cli-darwin-aarch64.tar.gz \
  | tar xz -C ~/.local/bin

# Intel Mac
curl -fsSL https://github.com/gorillaKim/obsidian-nexus/releases/latest/download/nexus-cli-darwin-x86_64.tar.gz \
  | tar xz -C ~/.local/bin
```

---

### 방법 5: 소스에서 빌드

**Prerequisites:** Rust 1.75+, Node.js 22+, pnpm

```bash
git clone https://github.com/gorillaKim/obsidian-nexus.git
cd obsidian-nexus

# CLI + MCP 서버 빌드
cargo build --release -p nexus-cli -p nexus-mcp-server
cp target/release/obs-nexus ~/.local/bin/
cp target/release/nexus-mcp-server ~/.local/bin/

# Desktop 앱 빌드 (선택)
cd apps/desktop && pnpm install && cargo tauri build
```

---

## Setup (설치 후 초기 설정)

### 0. Ollama 설치 (벡터 검색 필수 의존성)

벡터/하이브리드 검색을 사용하려면 Ollama를 **직접 설치**해야 합니다.

```bash
# macOS
brew install ollama

# 서비스 시작
ollama serve

# 임베딩 모델 다운로드 (768차원, ~274MB)
ollama pull nomic-embed-text
```

> 키워드 검색만 사용한다면 Ollama 없이도 동작합니다.

### 1. 초기화

```bash
obs-nexus setup
```

- Ollama 연결 확인 및 `nomic-embed-text` 모델 설치 여부 검증
- 로컬 데이터베이스 초기화 (`~/.nexus/`)

### 2. Obsidian 볼트 등록

```bash
obs-nexus project add --name "my-vault" --path /path/to/obsidian/vault
```

### 3. 문서 인덱싱

```bash
obs-nexus index my-vault      # 특정 볼트 인덱싱
obs-nexus index --all         # 모든 볼트 인덱싱
```

### 4. AI 에이전트(MCP) 연동

```bash
# 자동 설정 — .mcp.json 생성 + 에이전트 프롬프트 주입
obs-nexus onboard /path/to/my-project
```

또는 수동으로 `.mcp.json` 생성:

```json
{
  "mcpServers": {
    "nexus": {
      "type": "stdio",
      "command": "nexus-mcp-server",
      "args": []
    }
  }
}
```

---

## Usage

### CLI

```bash
# 검색
obs-nexus search "검색어"                           # 하이브리드 검색 (기본)
obs-nexus search "query" --mode keyword            # 키워드 검색
obs-nexus search "query" --mode vector             # 벡터 검색
obs-nexus search "query" --project my-vault --limit 10

# 프로젝트 관리
obs-nexus project add --name "vault" --path /path/to/vault
obs-nexus project list
obs-nexus project remove my-vault

# 인덱싱
obs-nexus index my-vault      # 특정 볼트
obs-nexus index --all         # 전체
obs-nexus watch               # 실시간 파일 감시

# 업데이트
obs-nexus update              # 최신 버전 확인 + 설치
obs-nexus update --check      # 확인만
obs-nexus update --force      # 캐시 무시하고 강제 확인
```

### MCP Tools (AI 에이전트용)

| 도구 | 용도 |
|------|------|
| `nexus_search` | 하이브리드/키워드/벡터 검색 |
| `nexus_get_document` | 문서 전체 내용 |
| `nexus_get_section` | 특정 섹션만 추출 (토큰 절약) |
| `nexus_resolve_alias` | 별칭으로 문서 찾기 |
| `nexus_get_metadata` | 태그, frontmatter 조회 |
| `nexus_get_backlinks` | 역방향 링크 탐색 |
| `nexus_get_links` | 정방향 링크 탐색 |
| `nexus_list_projects` | 등록된 볼트 목록 |
| `nexus_list_documents` | 볼트 내 문서 목록 |
| `nexus_index_project` | 인덱싱 트리거 |
| `nexus_status` | 시스템 상태 확인 |
| `nexus_onboard` | 프로젝트에 librarian 스킬 자동 설정 |

---

## Search Modes

| 모드 | 설명 | 적합한 상황 |
|------|------|-------------|
| `keyword` | FTS5 전문 검색 (한국어/영어) | 정확한 키워드 매칭 |
| `vector` | Ollama 임베딩 + KNN 유사도 | 의미적 유사 문서 탐색 |
| `hybrid` | keyword + vector + RRF 리랭킹 | 일반 검색 (기본값) |

---

## Architecture

```
apps/desktop          # Tauri v2 + React Desktop 앱
crates/cli            # CLI (nexus 명령어)
crates/mcp-server     # MCP 서버 (stdin/stdout JSON-RPC)
crates/core           # 핵심 엔진 (검색, 인덱싱, DB)
crates/agent          # AI 사서 에이전트 (사이드카 관리)
```

## Tech Stack

| 영역 | 기술 |
|------|------|
| Core Engine | Rust, SQLite (FTS5 + sqlite-vec), Ollama |
| Desktop | Tauri v2, React 19, TypeScript, Tailwind CSS 4 |
| CLI | Rust (clap) |
| MCP Server | Rust (stdin/stdout JSON-RPC 2.0) |
| AI 사서 | Node.js sidecar + Claude SDK |
| CI/CD | GitHub Actions |
| Embedding | nomic-embed-text (768D, Ollama) |

## Release

```bash
./scripts/bump-version.sh        # 패치 자동 증가
./scripts/bump-version.sh 1.0.0  # 특정 버전 지정
```

GitHub Actions가 자동으로 빌드 + Release 생성:
- macOS universal Desktop 앱 (`.dmg`)
- CLI 바이너리 (aarch64 + x86_64)
- 자동 업데이트 번들 (`latest.json` + 서명)

## Update

### Desktop 앱 자동 업데이트

앱 실행 중 새 버전이 릴리즈되면 자동으로 업데이트 알림이 표시됩니다. 확인을 누르면 재시작 후 최신 버전으로 업데이트됩니다.

> Desktop 앱 내부에 CLI(`obs-nexus`)와 MCP 서버(`nexus-mcp-server`)가 내장되어 있어, 앱을 통해 실행하는 경우 항상 최신 버전을 사용합니다.

### CLI 업데이트

터미널에서 직접 `obs-nexus`를 사용하는 경우 별도로 업데이트해야 합니다:

```bash
obs-nexus update          # 최신 버전 확인 + 설치
obs-nexus update --check  # 확인만 (설치 안 함)
```

MCP 서버(`nexus-mcp-server`)는 CLI 업데이트 시 함께 교체됩니다.

### 버전 확인

```bash
obs-nexus --version
```

---

## License

MIT
