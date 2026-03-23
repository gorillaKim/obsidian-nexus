---
title: 시작하기 (Getting Started)
aliases:
  - getting-started
  - quickstart
  - 시작하기
  - 설치
  - 온보딩
tags:
  - guide
  - getting-started
  - setup
  - onboarding
created: 2026-03-23
updated: 2026-03-23
---
<!-- docsmith: auto-generated 2026-03-23 -->

# 시작하기 (Getting Started)

Obsidian Nexus를 설치하고 첫 검색을 실행하기까지의 빠른 온보딩 가이드입니다.

---

## 사전 요구사항

### Ollama

벡터 검색(시맨틱 검색)을 사용하려면 [Ollama](https://ollama.com)가 필요합니다.

```bash
# Ollama 설치 (macOS)
brew install ollama

# nomic-embed-text 모델 다운로드 (768차원 임베딩)
ollama pull nomic-embed-text

# Ollama 서버 실행
ollama serve
```

> Ollama가 실행 중이지 않아도 키워드 검색은 동작합니다. 벡터/하이브리드 검색만 비활성화됩니다.

---

## 설치

### Homebrew (권장)

```bash
brew install gorilla-software/tap/obsidian-nexus
```

### Cargo (소스 빌드)

```bash
cargo install --path crates/cli
```

설치 후 확인:

```bash
nexus --version
```

---

## 초기 설정

데이터베이스 초기화 및 의존성 확인:

```bash
nexus setup
```

---

## 볼트 등록

Obsidian 볼트(또는 마크다운 디렉토리)를 프로젝트로 등록합니다.

```bash
nexus project add --name <이름> --path <경로>
```

예시:

```bash
nexus project add --name my-notes --path ~/Documents/my-obsidian-vault
```

등록된 프로젝트 목록 확인:

```bash
nexus project list
```

> 볼트 등록 시 macOS에서는 Obsidian URI 스킴을 통해 자동으로 볼트를 인식시킵니다.

---

## 인덱싱

등록한 프로젝트의 마크다운 파일을 인덱싱합니다.

```bash
nexus index <프로젝트명>
```

예시:

```bash
nexus index my-notes
```

전체 재인덱싱이 필요한 경우:

```bash
nexus index my-notes --full
```

모든 프로젝트를 한 번에 인덱싱:

```bash
nexus index --all
```

---

## 첫 검색

### 키워드 검색 (기본)

```bash
nexus search "쿼리" --project <프로젝트명>
```

예시:

```bash
nexus search "프로젝트 관리" --project my-notes
```

### 벡터 검색 (시맨틱)

```bash
nexus search "비슷한 의미의 문서 찾기" --project my-notes --mode vector
```

### 하이브리드 검색 (권장)

키워드와 벡터를 결합한 RRF 방식으로 가장 높은 품질의 결과를 제공합니다.

```bash
nexus search "쿼리" --project my-notes --mode hybrid
```

결과 수 제한:

```bash
nexus search "쿼리" --project my-notes --limit 5
```

---

## MCP 서버 설정 (Claude Desktop 연동)

Obsidian Nexus를 Claude Desktop 또는 Claude Code의 MCP 도구로 사용할 수 있습니다.

### 자동 설정 (권장)

프로젝트 디렉토리에서 실행하면 `.mcp.json`, `.claude/agents/librarian.md`, `.claude/skills/librarian/SKILL.md`를 자동 생성합니다.

```bash
nexus onboard
```

다른 경로를 지정하려면:

```bash
nexus onboard /path/to/my-project
```

세션을 재시작하면 MCP 서버가 활성화됩니다.

### 수동 설정

프로젝트 루트에 `.mcp.json` 파일을 생성합니다:

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

MCP 서버 바이너리 경로는 `which nexus-mcp-server`로 확인하세요.

---

## 변경 감지 (자동 재인덱싱)

볼트 파일 변경을 감지하여 자동으로 인덱싱합니다.

```bash
nexus watch <프로젝트명>
```

백그라운드 서비스로 등록하려면 [[05-설정-가이드]]를 참조하세요.

---

## 다음 단계

- [[03-MCP-도구-레퍼런스]] — Claude에서 사용 가능한 MCP 도구 전체 목록
- [[05-설정-가이드]] — 임베딩 모델, 검색 가중치, 인덱서 세부 설정
- [[08-서브에이전트-MCP-설정-가이드]] — Claude Code 서브에이전트에서 Nexus 도구 사용
- [[02-검색-시스템]] — 검색 모드(keyword/vector/hybrid) 상세 설명

## 관련 문서

- [[05-설정-가이드]]
- [[03-MCP-도구-레퍼런스]]
- [[08-서브에이전트-MCP-설정-가이드]]
- [[02-검색-시스템]]
