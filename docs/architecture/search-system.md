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

## 검색 모드 4가지

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
- 첫 번째 청크 임베딩 시 alias를 포함한 텍스트로 강화 (최대 5개):
  ```
  제목: {title}
  별칭: {a1, a2, ...}
  {content}
  ```

#### 임베딩 품질 보호 (MIN_EMBED_WORD_COUNT)

`MIN_EMBED_WORD_COUNT = 4` — `build_embed_text()` 출력 기준 단어 수가 4 미만인 청크는 임베딩 생성을 스킵한다.

**이유:** "채택 (Accepted)" 같은 짧은 micro-chunk는 벡터 공간에서 허브를 형성하여 모든 쿼리에 높은 유사도를 반환하는 노이즈 원인이 된다.

**비대칭 처리:** chunk 0(첫 번째 청크)은 `build_embed_text()`가 `제목: {title}\n별칭: {...}\n{content}` prefix를 포함하므로, 본문이 짧더라도 유효 단어 수가 충분히 높아 별도 예외 없이 동일 기준을 적용한다.

### 3. 하이브리드 검색 (기본)

FTS5 + 벡터 결과를 RRF(Reciprocal Rank Fusion)로 합산.

```
RRF_score = (1-weight) × 1/(rank_fts + 60) + weight × 1/(rank_vec + 60)
```

- `hybrid_weight` (기본 0.7): 벡터 70%, 키워드 30%
- 쿼리 길이 자동 조정: 2자 이하 ×0.3, 4자 이하 ×0.6

### 4. LLM 쿼리 재작성 (선택적)

Ollama `/api/generate`를 활용해 자연어 쿼리를 검색에 유리한 형태로 재작성. 기존 임베딩 Ollama 인프라를 재활용하며, 실패 시 원본 쿼리로 graceful fallback.

활성화 방법:
- `config.toml` `[llm]` 섹션: `enabled = true`, `model = "mistral"`
- `nexus_search(rewrite_query=true)`로 요청별 활성화 (keyword / vector / hybrid 모든 모드 지원)

보안 처리:
- 출력 첫 라인만 추출, 원본 길이 3배 상한 (프롬프트 인젝션 방어)
- 타임아웃 config화: `timeout_secs` (기본 5초, CPU Ollama 환경은 15초 권장)

**효과:** 사용자 자연어(UX 용어) ↔ 문서 기술 용어 간 semantic gap 해소

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
| 문서 제목 매칭 | +25% | 쿼리가 문서 title과 일치 (`eq_ignore_ascii_case`) |
| 루트 청크 | +15% | `heading_path`가 None인 문서 루트 청크 |
| heading 제목 매칭 | +10% | heading_path 최상위가 문서 제목과 일치 |
| 최상위 섹션 | +5% | heading_path에 ">" 없음 (depth 0) |
| backlink_count | +2%/개 (최대 20%) | use_popularity=true |
| view_count | log 스케일 (최대 15%) | use_popularity=true |

- title 비교 및 alias resolve는 모두 대소문자 무관(case-insensitive) 처리

## Title 추출

문서 제목은 아래 우선순위로 추출한다:

1. **H1 헤딩** — 본문 첫 번째 `# 제목` 파싱
2. **frontmatter `title` 필드** — H1이 없을 때 fallback
3. **null** — 둘 다 없으면 null (파일명 기반 처리는 호출부 책임)

docsmith 자동 생성 문서는 H1 없이 frontmatter `title`만 포함하는 경우가 많으므로 fallback이 필수다.

## FTS5 토큰 처리

- 언더스코어 분리: `wiki_links` → `"wiki_links" OR "wiki" OR "links"`
- 짧은 쿼리(≤3자): 프리픽스 매칭 (`앱` → `"앱" OR 앱*`)

## Alias 매칭

FTS5 `chunks_fts`에 `aliases` 컬럼이 통합되어 (V6 마이그레이션) alias 키워드도 BM25로 랭킹됩니다.

```sql
bm25(chunks_fts, 1.0, 0.5, 5.0)  -- aliases 컬럼 5배 가중치
```

- alias 매칭 문서가 랭킹 상위에 자동 배치
- 다중 alias 매칭 시 score 우위 반영
- `document_aliases` 테이블은 `nexus_get_backlinks` / `nexus_resolve_alias` 도구 호환성을 위해 유지
- alias resolve SQL: `WHERE LOWER(alias) = LOWER(?1)` — 대소문자 무관 비교

**Fallback (limit 미만 시):** 쿼리를 토큰으로 분리하여 OR 조건으로 추가 검색
```
"overview 페이지 리뉴얼" → LIKE '%overview%' OR LIKE '%페이지%' OR LIKE '%리뉴얼%'
```

- 예: "데이터독"으로 검색 → 본문은 "Datadog"이지만 alias "데이터독"으로 매칭

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
- [[ADR-009: 검색 품질 개선 — Micro-chunk 임베딩 스킵 & Title Fallback]]
