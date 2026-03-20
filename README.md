# Obsidian Nexus

Agent-friendly knowledge search engine for Obsidian vaults.

여러 Obsidian 볼트의 문서를 인덱싱하고, AI 에이전트가 MCP 프로토콜로 검색·탐색할 수 있는 로컬 지식 검색 엔진입니다.

## Features

- **하이브리드 검색** — FTS5 키워드 + sqlite-vec 벡터 시맨틱 검색 + RRF 리랭킹
- **멀티 볼트** — 여러 Obsidian 볼트를 등록하고 통합 검색
- **MCP 서버** — Claude, Gemini 등 AI 에이전트가 문서를 직접 검색/읽기
- **Desktop 앱** — Tauri v2 기반 GUI (검색, 프로젝트 관리, 자동 업데이트)
- **CLI** — 터미널에서 인덱싱, 검색, 볼트 관리
- **Alias 검색** — 한글 별칭으로 영문 문서 검색 (예: "데이터독" → datadog-setup.md)
- **자동 업데이트** — Desktop은 Tauri updater, CLI는 `nexus update`

## Architecture

```
apps/desktop          # Tauri v2 + React Desktop 앱
crates/cli            # CLI (nexus 명령어)
crates/mcp-server     # MCP 서버 (stdin/stdout JSON-RPC)
crates/core           # 핵심 엔진 (검색, 인덱싱, DB)
```

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- [Node.js](https://nodejs.org/) 22+ & [pnpm](https://pnpm.io/)
- [Ollama](https://ollama.ai/) (벡터 검색용 임베딩)

### Quick Setup

```bash
# 1. 소스 클론
git clone https://github.com/gorillaKim/obsidian-nexus.git
cd obsidian-nexus

# 2. CLI + MCP 서버 빌드 & 설치
cargo build --release -p nexus-cli -p nexus-mcp-server
cp target/release/nexus ~/.local/bin/
cp target/release/nexus-mcp-server ~/.local/bin/

# 3. 의존성 설치 (Obsidian, Ollama, 임베딩 모델, DB 초기화)
nexus setup

# 4. 볼트 등록 & 인덱싱
nexus project add --name "my-vault" --path /path/to/obsidian/vault
nexus index my-vault
```

### Desktop 앱 설치

```bash
# 방법 1: GitHub Releases에서 DMG 다운로드
# https://github.com/gorillaKim/obsidian-nexus/releases

# 방법 2: 소스에서 빌드
cd apps/desktop
pnpm install
pnpm tauri:build
```

### MCP 서버 등록 (AI 에이전트 연동)

```bash
# 자동 설정 (프로젝트에 .mcp.json + librarian 에이전트 생성)
nexus onboard /path/to/my-project

# 수동 설정: .mcp.json 생성
cat > .mcp.json << 'EOF'
{
  "mcpServers": {
    "nexus": {
      "type": "stdio",
      "command": "nexus-mcp-server",
      "args": []
    }
  }
}
EOF
```

## Usage

### CLI

```bash
# 검색
nexus search "검색어" --mode hybrid --limit 10
nexus search "query" --project my-vault --mode keyword

# 프로젝트 관리
nexus project add --name "vault" --path /path/to/vault
nexus project list
nexus index --all

# 파일 감시 (실시간 인덱싱)
nexus watch

# 업데이트
nexus update           # 확인 + 설치
nexus update --check   # 확인만
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

### Desktop App

```bash
# 개발 모드
cargo tauri dev

# 프로덕션 빌드
cd apps/desktop && pnpm tauri:build
```

## Search Modes

| 모드 | 설명 | 적합한 상황 |
|------|------|-------------|
| `keyword` | FTS5 전문 검색 (한국어/영어) | 정확한 키워드 매칭 |
| `vector` | Ollama 임베딩 + KNN 유사도 | 의미적 유사 문서 탐색 |
| `hybrid` | keyword + vector + RRF 리랭킹 | 일반 검색 (기본값) |

## Release

```bash
./scripts/bump-version.sh        # 패치 자동 증가
./scripts/bump-version.sh 1.0.0  # 특정 버전 지정
```

GitHub Actions가 자동으로 빌드 + Release 생성:
- macOS universal Desktop 앱 (.dmg)
- CLI 바이너리 (aarch64 + x86_64)
- 자동 업데이트 번들 (latest.json + 서명)

## Tech Stack

| 영역 | 기술 |
|------|------|
| Core Engine | Rust, SQLite (FTS5 + sqlite-vec), Ollama |
| Desktop | Tauri v2, React 19, TypeScript, Tailwind CSS 4 |
| CLI | Rust (clap) |
| MCP Server | Rust (stdin/stdout JSON-RPC 2.0) |
| CI/CD | GitHub Actions |
| Embedding | nomic-embed-text (768D, Ollama) |

## License

MIT
