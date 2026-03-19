---
title: 검색 시스템
tags:
  - search
  - fts5
  - vector
  - sqlite-vec
aliases:
  - Search System
  - 하이브리드 검색
---

# 검색 시스템

## 검색 모드 3가지

### 1. 키워드 검색 (FTS5)

SQLite FTS5 전문 검색. `unicode61` 토크나이저로 한국어/영어 모두 지원.

```sql
SELECT ... FROM chunks_fts WHERE chunks_fts MATCH ?1
```

- 짧은 쿼리(3자 이하): 프리픽스 매칭 (`"앱" OR 앱*`)
- 긴 쿼리: 정확 구문 매칭 (`"검색어"`)

### 2. 벡터 검색 (sqlite-vec + Ollama)

의미 기반 유사도 검색. Ollama `nomic-embed-text`로 768차원 벡터 생성.

```
검색어 → Ollama → 벡터 → 정규화 → sqlite-vec KNN 쿼리
```

```sql
SELECT ... FROM vec_chunks WHERE embedding MATCH ?1 AND k = ?2
```

- 임베딩 정규화: L2 거리 ≈ 코사인 유사도 (수학적 동치)
- `min_vector_score` (기본 0.65)로 노이즈 필터링

### 3. 하이브리드 검색 (기본)

FTS5 + 벡터 결과를 RRF(Reciprocal Rank Fusion)로 합산.

```
RRF_score = (1-weight) × 1/(rank_fts + 60) + weight × 1/(rank_vec + 60)
```

- `hybrid_weight` (기본 0.7): 벡터 70%, 키워드 30%
- 쿼리 길이 자동 조정: 2자 이하 ×0.3, 4자 이하 ×0.6

## 메타데이터 리랭킹

검색 결과에 메타데이터 신호를 추가 부스트:

| 신호 | 부스트 | 설명 |
|------|--------|------|
| backlink_count | +2%/개 (최대 20%) | 많이 참조되는 허브 문서 |
| view_count | log 스케일 (최대 15%) | 자주 열어본 인기 문서 |

- 프로젝트 내 검색: 인기도 부스트 기본 ON
- 전체 검색: 인기도 부스트 기본 OFF

## 메타데이터 리랭킹 상세

검색 결과에 다음 부스트를 적용하여 재정렬:

| 신호 | 부스트 | 조건 |
|------|--------|------|
| 문서 제목 매칭 | +10% | heading_path 최상위가 문서 제목과 일치 |
| 최상위 섹션 | +5% | heading_path에 ">" 없음 (depth 0) |
| backlink_count | +2%/개 (최대 20%) | use_popularity=true |
| view_count | log 스케일 (최대 15%) | use_popularity=true |

## FTS5 토큰 처리

- 언더스코어 분리: `wiki_links` → `"wiki_links" OR "wiki" OR "links"`
- 짧은 쿼리(≤3자): 프리픽스 매칭 (`앱` → `"앱" OR 앱*`)

## Alias Fallback

FTS5 결과가 limit 미만일 때, `document_aliases` 테이블을 LIKE 검색하여 추가 결과를 보충합니다.

- frontmatter의 `aliases` 필드에 등록된 별칭으로 검색 가능
- 예: "데이터독"으로 검색 → 본문은 "Datadog"이지만 alias "데이터독"으로 매칭
- FTS5 결과에 이미 포함된 문서는 중복 제거

## 태그 기반 검색

검색 결과를 태그로 필터링할 수 있습니다 (옵셔널).

```json
nexus_search({
  "query": "검색어",
  "tags": ["rust", "api"],
  "tag_match_all": false
})
```

- `tag_match_all=false` (기본, OR): 태그 중 **하나라도** 매칭되면 결과에 포함
- `tag_match_all=true` (AND): **모든** 태그가 매칭되는 결과만 포함
- 대소문자 무시
- `tags` 생략 시 전체 결과 반환
- `enrich=true` (기본값) 필요 — 태그 데이터가 있어야 필터링 가능

데스크톱 앱에서는 검색바 아래 태그 입력란에 `rust, api` 형태로 콤마 구분 입력.

## 검색 결과 구조

```json
{
  "chunk_id": "...",
  "file_path": "projects/obsidian-nexus.md",
  "heading_path": "프로젝트 > 기술 스택",
  "snippet": "...",
  "score": 0.017,
  "tags": ["rust", "project"],
  "backlink_count": 3,
  "view_count": 10,
  "last_modified": "2026-03-19"
}
```

## 관련 문서

- [[01-아키텍처]]
- [[03-MCP-도구-레퍼런스]]
- [[05-설정-가이드]]
