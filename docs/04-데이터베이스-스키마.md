---
title: 데이터베이스 스키마
tags:
  - database
  - schema
  - sqlite
aliases:
  - DB Schema
---

# 데이터베이스 스키마

SQLite 단일 파일 DB. 5단계 마이그레이션.

## 테이블 구조

### projects

볼트(프로젝트) 메타데이터.

| 컬럼 | 타입 | 설명 |
|------|------|------|
| id | TEXT PK | UUID |
| name | TEXT UNIQUE | 프로젝트명 |
| vault_name | TEXT | Obsidian 볼트명 |
| path | TEXT UNIQUE | 볼트 경로 |
| created_at | DATETIME | 등록 시간 |
| last_indexed_at | DATETIME | 마지막 인덱싱 |

### documents

인덱싱된 마크다운 파일.

| 컬럼 | 타입 | 설명 |
|------|------|------|
| id | TEXT PK | UUID |
| project_id | TEXT FK | 소속 프로젝트 |
| file_path | TEXT | 볼트 내 상대 경로 |
| title | TEXT | 첫 H1 헤딩 |
| content_hash | TEXT | SHA-256 (증분 인덱싱용) |
| frontmatter | TEXT | JSON 직렬화된 YAML |
| indexing_status | TEXT | pending/indexing/done/error |
| created_at | DATETIME | 파일 생성 시간 (V5) |
| last_modified | DATETIME | 파일 수정 시간 |
| last_indexed | DATETIME | 인덱싱 완료 시간 |

### chunks

문서를 heading 기준으로 분할한 청크.

| 컬럼 | 타입 | 설명 |
|------|------|------|
| id | TEXT PK | UUID |
| document_id | TEXT FK | 소속 문서 |
| chunk_index | INTEGER | 순서 |
| content | TEXT | 텍스트 내용 |
| heading_path | TEXT | "H1 > H2 > H3" 형태 |
| start_line | INTEGER | 시작 줄 번호 |
| end_line | INTEGER | 끝 줄 번호 |

### chunks_fts (V1, FTS5 가상 테이블)

청크의 전문 검색 인덱스. `unicode61` 토크나이저.
INSERT/UPDATE/DELETE 트리거로 chunks와 자동 동기화.

### vec_chunks (V3, sqlite-vec 가상 테이블)

```sql
CREATE VIRTUAL TABLE vec_chunks USING vec0(
    chunk_id TEXT PRIMARY KEY,
    embedding float[768]
);
```

정규화된 768차원 벡터 저장. KNN 쿼리 지원.

### tags / document_tags (V1)

정규화된 태그 저장. frontmatter + 인라인 `#태그` 모두 포함.

### wiki_links (V4)

문서 간 `[[위키링크]]` 관계.

| 컬럼 | 타입 | 설명 |
|------|------|------|
| source_doc_id | TEXT FK | 링크 출발 문서 |
| target_path | TEXT | `[[target]]`의 target |
| display_text | TEXT | `[[target\|display]]`의 display |
| target_doc_id | TEXT FK | resolve된 대상 문서 (NULL=미해결) |

### document_aliases (V4)

frontmatter `aliases` 필드 저장.

### document_views (V5)

문서 조회 기록 (인기도 랭킹용).

| 컬럼 | 타입 | 설명 |
|------|------|------|
| document_id | TEXT FK | 조회된 문서 |
| viewed_at | DATETIME | 조회 시간 |

## 마이그레이션 히스토리

| 버전 | 파일 | 내용 |
|------|------|------|
| V1 | `V1__initial.sql` | projects, documents, chunks, chunks_fts, tags, indexing_queue |
| V2 | `V2__embeddings.sql` | chunk_embeddings (레거시 BLOB) |
| V3 | `V3__sqlite_vec.sql` | vec_chunks (sqlite-vec 가상 테이블) |
| V4 | `V4__links.sql` | wiki_links, document_aliases |
| V5 | `V5__search_enhancements.sql` | document_views (인기도 추적), documents.created_at |

## 볼트 설정 파일 (on-config.json)

DB 외부에 볼트 루트에 저장되는 설정 파일:

```json
{"name": "프로젝트 표시명"}
```

- 볼트 등록 시 자동 생성
- `nexus_sync_config` MCP 도구로 DB와 동기화

## 관련 문서

- [[01-아키텍처]]
- [[02-검색-시스템]]
