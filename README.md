# Obsidian Nexus

> v0.5.4 · macOS (Apple Silicon / Intel)

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

> **어떤 방법을 선택할까?**
> - AI 에이전트(Claude Code 등)와 연동이 목적 → **방법 1 (curl)** 또는 **방법 2 (Homebrew)**
> - GUI로 편하게 쓰고 싶다 → **방법 3 (Desktop 앱)**
> - 직접 빌드하고 싶다 → **방법 5 (소스)**

---

### 방법 1: curl 스크립트 (권장 — CLI + MCP 서버)

```bash
curl -fsSL https://raw.githubusercontent.com/gorillaKim/obsidian-nexus/master/install.sh | bash
```

설치 완료 후 터미널을 **새로 열거나** 아래 명령어로 PATH를 즉시 적용하세요:

```bash
source ~/.zshrc    # zsh 사용 시
source ~/.bashrc   # bash 사용 시
```

설치 확인:

```bash
obs-nexus --version
```

**설치 내용:**
- 아키텍처 자동 감지 (Apple Silicon / Intel)
- `obs-nexus` + `nexus-mcp-server` → `~/.local/bin` 설치
- SHA256 체크섬 검증
- `~/.zshrc` / `~/.bashrc`에 PATH 자동 추가

> 설치 디렉토리를 변경하려면: `NEXUS_INSTALL_DIR=/usr/local/bin curl -fsSL ... | bash`

---

### 방법 2: Homebrew

```bash
brew tap gorillaKim/nexus
brew install gorillaKim/nexus/obsidian-nexus      # CLI + MCP 서버
```

Desktop 앱도 함께 설치하려면:

```bash
brew install --cask gorillaKim/nexus/obsidian-nexus
```

업데이트:
```bash
brew upgrade obsidian-nexus           # CLI + MCP 서버
brew upgrade --cask obsidian-nexus    # Desktop 앱
```

> **사전 요구사항:** macOS Command Line Tools 필요. 없으면 `xcode-select --install` 실행.

---

### 방법 3: Desktop 앱 수동 설치

1. [Releases 페이지](https://github.com/gorillaKim/obsidian-nexus/releases/latest)에서 `Obsidian-Nexus.dmg` 다운로드
2. DMG 열기 → `Obsidian Nexus.app`을 `/Applications`로 드래그
3. 앱 실행

> Desktop 앱 안에 CLI와 MCP 서버가 내장되어 있습니다. 앱을 통해 실행하면 항상 최신 버전이 사용됩니다.

**최초 실행 시 자동 처리:**

| 항목 | 동작 | 비고 |
|------|------|------|
| Obsidian 설치 | 미설치 시 `brew install --cask obsidian` 자동 실행 | |
| CLI 심볼릭 링크 | `~/.local/bin/obs-nexus`, `~/.local/bin/nexus-mcp-server` 생성 | 앱 내장 바이너리를 가리킴 |
| PATH 추가 | `~/.zshrc`에 `export PATH="$HOME/.local/bin:$PATH"` 추가 | `.local/bin`이 없는 경우에만 / **bash 사용자는 수동 추가 필요** |
| MCP 서버 등록 | Claude Desktop / Claude Code / Gemini CLI 설정에 자동 등록 | 해당 앱이 설치된 경우에만 |
| DB 초기화 | `~/.nexus/` 데이터베이스 생성 + 스키마 마이그레이션 적용 | |

**자동으로 처리되지 않는 항목:**
- **Ollama** — 벡터/하이브리드 검색을 원하면 [Setup 0단계](#0-ollama-설치-벡터-검색)를 참고하세요. 키워드 검색만 쓴다면 불필요합니다.
- **볼트 등록 및 인덱싱** — 앱 UI 또는 CLI에서 직접 진행해야 합니다.

**"앱이 손상되었습니다" 오류 해결:**

macOS가 App Store 외부 앱에 격리(quarantine) 속성을 부여해 발생합니다.

```bash
xattr -cr /Applications/Obsidian\ Nexus.app
```

위 명령이 동작하지 않으면 (macOS 15 Sequoia 이상):

```bash
sudo xattr -d com.apple.quarantine /Applications/Obsidian\ Nexus.app
```

또는 **시스템 설정 → 개인정보 보호 및 보안 → 보안** 섹션에서 "확인 없이 열기" 버튼을 클릭하세요.

---

### 방법 4: CLI 수동 설치

`~/.local/bin` 디렉토리가 없으면 먼저 생성하세요:

```bash
mkdir -p ~/.local/bin
```

CLI와 MCP 서버를 함께 설치합니다:

```bash
# Apple Silicon (M1/M2/M3/M4)
BASE=https://github.com/gorillaKim/obsidian-nexus/releases/latest/download
curl -fsSL $BASE/nexus-cli-darwin-aarch64.tar.gz    | tar xz -C ~/.local/bin
curl -fsSL $BASE/nexus-mcp-server-darwin-aarch64.tar.gz | tar xz -C ~/.local/bin

# Intel Mac
BASE=https://github.com/gorillaKim/obsidian-nexus/releases/latest/download
curl -fsSL $BASE/nexus-cli-darwin-x86_64.tar.gz    | tar xz -C ~/.local/bin
curl -fsSL $BASE/nexus-mcp-server-darwin-x86_64.tar.gz | tar xz -C ~/.local/bin
```

PATH에 추가 (아직 없는 경우):

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

---

### 방법 5: 소스에서 빌드

**사전 요구사항:**
- Rust 1.75+ (`rustup` 권장)
- Node.js 22+, pnpm (Desktop 앱 빌드 시에만 필요)
- Xcode Command Line Tools (`xcode-select --install`)

```bash
git clone https://github.com/gorillaKim/obsidian-nexus.git
cd obsidian-nexus

# CLI + MCP 서버 빌드 및 설치
cargo build --release -p nexus-cli -p nexus-mcp-server
cp target/release/obs-nexus ~/.local/bin/
cp target/release/nexus-mcp-server ~/.local/bin/

# Desktop 앱 빌드 (선택)
cd apps/desktop && pnpm install && cargo tauri build
```

---

## Setup (설치 후 초기 설정)

설치가 완료됐으면 아래 순서대로 초기 설정을 진행하세요.

### 0. Ollama 설치 (벡터 검색)

하이브리드/벡터 검색을 사용하려면 Ollama가 필요합니다. **키워드 검색만 사용한다면 이 단계를 건너뛰세요.**

```bash
# Ollama 설치
brew install ollama

# 백그라운드 서비스로 시작
brew services start ollama

# 임베딩 모델 다운로드 (~274MB, 최초 1회)
ollama pull nomic-embed-text
```

정상 설치 확인:

```bash
ollama list   # nomic-embed-text 가 목록에 보이면 완료
```

### 1. 초기화

```bash
obs-nexus setup
```

- Ollama 연결 및 `nomic-embed-text` 모델 설치 여부 확인
- 로컬 데이터베이스 초기화 (`~/.nexus/`)

Ollama 없이 키워드 검색만 사용하는 경우에도 이 단계는 필요합니다.

### 2. Obsidian 볼트 등록

```bash
obs-nexus project add --name my-notes --path ~/Documents/MyObsidianVault

# 등록 확인
obs-nexus project list
```

`--path`는 Obsidian이 열 때 지정하는 볼트 루트 디렉토리입니다.

### 3. 문서 인덱싱

```bash
obs-nexus index my-notes          # 증분 인덱싱 (변경분만)
obs-nexus index my-notes --full   # 전체 재인덱싱
obs-nexus index --all             # 모든 볼트 인덱싱
```

인덱싱이 완료되면 검색이 가능합니다:

```bash
obs-nexus search "찾고싶은 내용"
```

### 4. AI 에이전트(MCP) 연동

Claude Code 등 AI 에이전트가 문서를 직접 검색하게 하려면 MCP 서버를 연동합니다.

**자동 설정 (권장):**

```bash
# 프로젝트 디렉토리에서 실행 — .mcp.json 자동 생성
obs-nexus onboard /path/to/my-project
```

연동 확인:

```bash
obs-nexus status   # Ollama 연결 + 인덱스 상태 출력
```

**수동 설정:** 프로젝트 루트에 `.mcp.json` 파일 생성:

```json
{
  "mcpServers": {
    "nexus": {
      "type": "stdio",
      "command": "/Users/yourname/.local/bin/nexus-mcp-server",
      "args": []
    }
  }
}
```

> `command` 경로는 `which nexus-mcp-server` 출력값으로 대체하세요.

---

## Claude Code 플러그인으로 활용하기 (jake-marketplace)

[jake-marketplace](https://github.com/gorillaKim/jake-marketplace)는 Claude Code용 개인 플러그인 마켓플레이스입니다. **obsidian-nexus 플러그인**을 설치하면 Claude Code 에이전트가 문서를 자동 생성·관리하고, obs-nexus CLI와 연동하여 지식베이스를 직접 탐색할 수 있습니다.

### 마켓플레이스 + 플러그인 설치

```bash
# 1. 마켓플레이스 등록
/plugin marketplace add gorillaProject/jake-marketplace

# 2. obsidian-nexus 플러그인 설치
/plugin install obsidian-nexus@jake-plugins
```

> **사전 요구사항:** `obs-nexus` CLI가 설치되어 있어야 합니다 — [Installation 방법 1](#방법-1-curl-스크립트-권장--cli--mcp-서버) 참고.

### 플러그인 제공 스킬

| 스킬 | 명령어 | 동작 |
|------|--------|------|
| **onboard** | `/obsidian-nexus:onboard` | 프로젝트 코드베이스 분석 → 인터뷰 → `docs/` 문서 세트 자동 생성 |
| **librarian** | `/obsidian-nexus:librarian` | obs-nexus CLI 기반 문서 검색, 발견성 개선, 문서 최신화 |
| **add** | `/obsidian-nexus:add` | devlog, ADR, troubleshooting 등 개별 문서 추가 |
| **session-devlog** | `/obsidian-nexus:session-devlog` | 현재 세션 대화 내용을 devlog로 자동 정리 |
| **doctor** | `/obsidian-nexus:doctor` | 문서 상태 진단 (누락, 오래됨, 코드-문서 불일치) |

### 활용 흐름 예시

```
1. obs-nexus 설치 + 볼트 인덱싱 (본 README Setup 참고)
2. Claude Code에서 /obsidian-nexus:onboard 실행
   → 프로젝트 분석 후 docs/ 문서 자동 생성
3. 개발 중 /obsidian-nexus:librarian 으로 관련 문서 즉시 검색
4. 작업 완료 후 /obsidian-nexus:session-devlog 로 개발 일지 기록
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
| `nexus_get_backlinks` | 역방향 링크 탐색 (1-hop) |
| `nexus_get_links` | 정방향 링크 탐색 (1-hop) |
| `nexus_get_cluster` | 멀티홉 그래프 탐색 (앞+뒤 방향, depth 파라미터) |
| `nexus_find_path` | 두 문서 간 최단 경로 탐색 |
| `nexus_find_related` | 링크+태그 기반 유사 문서 추천 |
| `nexus_list_projects` | 등록된 볼트 목록 |
| `nexus_list_documents` | 볼트 내 문서 목록 |
| `nexus_index_project` | 인덱싱 트리거 |
| `nexus_status` | 시스템 상태 확인 |
| `nexus_onboard` | 프로젝트에 librarian 스킬 자동 설정 |

---

## Q&A (자주 묻는 문제)

### 설치

**Q. `obs-nexus: command not found` 오류가 납니다.**

`~/.local/bin`이 PATH에 없는 경우입니다.

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

터미널을 새로 열어도 해결됩니다.

---

**Q. curl 설치 스크립트가 "Permission denied"로 실패합니다.**

`~/.local/bin` 디렉토리가 없거나 쓰기 권한이 없는 경우입니다.

```bash
mkdir -p ~/.local/bin
# 이후 설치 스크립트 재실행
```

특정 디렉토리에 설치하려면:

```bash
NEXUS_INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/gorillaKim/obsidian-nexus/master/install.sh | bash
```

---

**Q. Desktop 앱을 열면 "앱이 손상되었습니다"라고 합니다.**

macOS Gatekeeper가 App Store 외부 앱을 차단하는 것입니다.

```bash
xattr -cr /Applications/Obsidian\ Nexus.app
```

macOS 15 Sequoia 이상에서 위 명령이 안 되면:

```bash
sudo xattr -d com.apple.quarantine /Applications/Obsidian\ Nexus.app
```

---

### 초기 설정

**Q. `obs-nexus setup` 실행 시 "Ollama connection failed" 오류가 납니다.**

Ollama가 실행 중이지 않은 경우입니다.

```bash
# 서비스로 실행 (재부팅 후에도 자동 시작)
brew services start ollama

# 또는 포그라운드로 실행 (터미널 별도 유지 필요)
ollama serve
```

실행 후 `obs-nexus setup`을 재시도하세요.

---

**Q. `obs-nexus setup` 실행 시 "model 'nomic-embed-text' not found" 오류가 납니다.**

임베딩 모델이 아직 다운로드되지 않은 상태입니다.

```bash
ollama pull nomic-embed-text   # ~274MB 다운로드
```

완료 후 `obs-nexus setup` 재실행.

---

**Q. 벡터 검색 없이 키워드 검색만 사용하고 싶습니다.**

Ollama 없이도 동작합니다. `obs-nexus setup` 시 Ollama 연결 오류가 나도 계속 진행하면 FTS5 키워드 검색 전용으로 사용할 수 있습니다.

검색 시 모드를 명시하세요:

```bash
obs-nexus search "query" --mode keyword
```

---

### 인덱싱

**Q. 인덱싱이 완료됐는데 검색 결과가 나오지 않습니다.**

1. 프로젝트 이름이 올바른지 확인:

```bash
obs-nexus project list
```

2. 인덱스된 문서 수 확인:

```bash
obs-nexus status
```

3. 문서 수가 0이면 전체 재인덱싱:

```bash
obs-nexus index my-notes --full
```

---

**Q. 볼트 경로를 잘못 등록했습니다.**

프로젝트를 제거하고 올바른 경로로 다시 추가하세요:

```bash
obs-nexus project remove my-notes
obs-nexus project add --name my-notes --path /correct/path/to/vault
obs-nexus index my-notes --full
```

---

### MCP 연동

**Q. Claude Code에서 `nexus_search` 도구가 보이지 않습니다.**

1. `.mcp.json`이 프로젝트 루트에 있는지 확인하세요.
2. `nexus-mcp-server` 경로가 올바른지 확인:

```bash
which nexus-mcp-server   # 경로 출력 확인
```

출력된 절대 경로를 `.mcp.json`의 `command`에 넣으세요.

3. Claude Code를 재시작하세요.

---

**Q. MCP 서버가 실행되다가 바로 종료됩니다.**

`obs-nexus status`로 시스템 상태를 확인하세요:

```bash
obs-nexus status
```

DB가 초기화되지 않은 경우 `obs-nexus setup`을 먼저 실행해야 합니다.

---

**Q. `obs-nexus onboard`를 실행했는데 `.mcp.json`이 생성되지 않습니다.**

`onboard` 명령에 프로젝트 디렉토리 경로를 인자로 전달해야 합니다:

```bash
obs-nexus onboard /path/to/your/project   # 절대 경로 사용
```

---

### 업데이트

**Q. 새 버전이 나왔는데 자동 업데이트가 안 됩니다.**

CLI는 자동 업데이트되지 않습니다. 수동으로 업데이트하세요:

```bash
obs-nexus update
```

Homebrew로 설치했다면:

```bash
brew upgrade obsidian-nexus
```

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
obs-nexus update --force  # 24시간 캐시 무시하고 강제 확인
```

GitHub Release API로 최신 버전을 확인하고, SHA256 체크섬을 검증한 뒤 atomic 교체합니다. MCP 서버(`nexus-mcp-server`)는 CLI 업데이트 시 함께 교체됩니다.

### 버전 확인

```bash
obs-nexus --version
```

---

## License

MIT
