---
title: 설정 가이드
tags:
  - config
  - setup
aliases:
  - Configuration
  - 설정
---

# 설정 가이드

## 설정 파일 위치

`~/.config/obsidian-nexus/config.toml` (macOS/Linux)

## 설정 항목

### 임베딩 (embedding)

```toml
[embedding]
provider = "ollama"              # ollama 또는 openai
model = "nomic-embed-text"       # 임베딩 모델명
dimensions = 768                 # 벡터 차원 수
ollama_url = "http://localhost:11434"
```

### 인덱서 (indexer)

```toml
[indexer]
chunk_size = 512                 # 청크 최대 문자 수
chunk_overlap = 50               # 청크 간 겹침 문자 수
exclude_patterns = [".obsidian", ".trash", "node_modules", ".git"]
```

### 검색 (search)

```toml
[search]
default_limit = 20               # 기본 검색 결과 수
hybrid_weight = 0.7              # 벡터 비중 (0.0=키워드만, 1.0=벡터만)
min_vector_score = 0.65          # 최소 벡터 유사도 (노이즈 필터)
```

#### 권장값

| 항목 | 권장값 | 설명 |
|------|--------|------|
| hybrid_weight | 0.7 | 벡터 70%, 키워드 30% |
| min_vector_score | 0.65 | nomic-embed-text 기준 적정 임계값 |

### 감시 (watcher)

```toml
[watcher]
debounce_ms = 500                # 파일 변경 감지 디바운스
```

## 사전 요구 사항

### Ollama 설치 및 실행

```bash
# 설치
brew install ollama

# 모델 다운로드
ollama pull nomic-embed-text

# 서버 실행
ollama serve
```

Ollama가 실행 중이지 않으면 벡터 검색이 비활성화되고 키워드 검색만 동작합니다.

### MCP 서버 등록

데스크톱 앱 첫 실행 시 자동으로 Claude Desktop/Claude Code에 MCP 서버를 등록합니다.

수동 등록:
```json
// ~/.claude/settings.json
{
  "mcpServers": {
    "nexus": {
      "command": "/path/to/nexus-mcp-server",
      "args": []
    }
  }
}
```

## 볼트 설정 (on-config.json)

각 Obsidian 볼트 루트에 `on-config.json`을 생성하여 프로젝트 표시명을 관리합니다.

```json
{
  "name": "My Project Docs"
}
```

- **자동 생성**: 볼트 등록 시 파일이 없으면 폴더명으로 자동 생성
- **이름 변경**: `name` 필드 수정 후 동기화 필요
  - MCP: `nexus_sync_config`
  - 데스크톱: 동기화 버튼

### 볼트 자동 감지

폴더 선택 시 `.obsidian/` 디렉토리가 있는 하위 폴더를 자동 탐색합니다 (최대 depth 3).

```
~/Documents/ 선택
  → ~/Documents/work-vault/.obsidian/ 발견 → "work-vault" 등록
  → ~/Documents/personal/.obsidian/ 발견 → "personal" 등록
```

## 관련 문서

- [[00-프로젝트-개요]]
- [[06-개발-가이드]]
