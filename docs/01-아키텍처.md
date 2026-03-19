---
title: 아키텍처
tags:
  - architecture
  - design
aliases:
  - Architecture
---

# 아키텍처

## 3-Layer 구조

### Layer 1: Core Engine (`crates/core/`)

모든 비즈니스 로직이 여기에 집중.

| 모듈 | 파일 | 역할 |
|------|------|------|
| indexer | `indexer.rs` | 마크다운 파싱, 청킹, 위키링크/태그/aliases 추출 |
| index_engine | `index_engine.rs` | 파일 워킹, 증분 인덱싱, 임베딩 생성 |
| search | `search.rs` | FTS5, 벡터, 하이브리드 검색, 백링크, 메타데이터 리랭킹 |
| embedding | `embedding.rs` | Ollama API 호출, 벡터 정규화, 직렬화 |
| db | `db/sqlite.rs` | 커넥션 풀, 마이그레이션, sqlite-vec 확장 로드 |
| config | `config.rs` | TOML 설정 파일 관리 |
| watcher | `watcher.rs` | 파일 시스템 감시 (notify 크레이트) |
| project | `project.rs` | 프로젝트(볼트) CRUD |

### Layer 2: Interface

| 인터페이스 | 파일 | 역할 |
|-----------|------|------|
| CLI | `crates/cli/` | `nexus` 바이너리, 터미널 사용 |
| MCP Server | `crates/mcp-server/` | AI 에이전트용 JSON-RPC 서버 |
| Tauri IPC | `apps/desktop/src-tauri/` | 데스크톱 앱 프론트엔드 연동 |

### Layer 3: Presentation

| 화면 | 설명 |
|------|------|
| 대시보드 | 프로젝트 통계, 인덱싱 상태 |
| 검색 | 하이브리드 검색 + 모드 선택 + 설정 패널 |
| 프로젝트 관리 | 볼트 추가/삭제/인덱싱 |
| 가이드 | 사용 안내 |

## 데이터 흐름

```
마크다운 파일
    ↓ [parse_markdown]
ParsedDocument { title, frontmatter, chunks, tags, wiki_links, aliases }
    ↓ [embed_text → normalize]
embeddings: Vec<Vec<f32>>
    ↓ [atomic transaction]
    ├→ documents 테이블
    ├→ chunks 테이블 → chunks_fts (FTS5 트리거)
    ├→ vec_chunks (sqlite-vec)
    ├→ tags + document_tags
    ├→ wiki_links (forward link resolve)
    └→ document_aliases
```

## 관련 문서

- [[00-프로젝트-개요]]
- [[02-검색-시스템]]
- [[04-데이터베이스-스키마]]
