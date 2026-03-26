---
title: "Alias 검색 개선 — 임베딩 강화 & FTS5 통합"
aliases:
  - alias-search-improvement
  - fts5-alias-integration
  - embedding-enhancement
  - alias 검색 개선
  - 임베딩 강화
  - FTS5 통합
created: "2026-03-26"
updated: "2026-03-26"
tags:
  - devlog
  - feature
  - search
  - alias
  - fts5
  - embedding
---

<!-- docsmith: auto-generated 2026-03-26 -->

# Alias 검색 개선 — 임베딩 강화 & FTS5 통합

## 배경

Gemini의 alias 개선 제안 검토 결과 현재 검색 시스템에서 두 가지 구조적 한계를 확인했다.

1. **벡터 검색 누락**: alias가 임베딩 텍스트에 포함되지 않아, alias 키워드로 벡터 검색 시 관련 문서를 찾지 못함
2. **alias 매칭 점수 미반영**: FTS alias fallback이 `LIKE` 기반 검색으로 동작하며 score=1.0 고정 — 다중 alias 매칭 우위가 랭킹에 반영되지 않음

두 문제를 각각 독립 태스크로 분리하여 구현했다.

## Task 1: 임베딩 텍스트 강화

**파일**: `crates/core/src/index_engine.rs`

`build_embed_text(title, file_stem, aliases, content, chunk_index)` 함수를 `pub(crate)`로 추출했다. 첫 번째 청크 임베딩 시 아래 형태로 Ollama 입력 텍스트를 구성한다.

```
제목: {title}
별칭: {a1, a2, ...}
{content}
```

핵심 결정 사항:

- `MAX_ALIAS_COUNT = 5` 모듈 레벨 상수로 정의 — 6개 이상 alias는 임베딩 텍스트에서 잘라냄
- FTS 저장 content는 변경 없이 유지 — 임베딩 입력과 FTS 인덱스 텍스트를 분리
- non-first chunk는 alias 없이 content만 사용 (chunk 중복 방지)

단위 테스트 4개 추가: aliases 있는 경우, 없는 경우, non-first chunk, 6개 초과 제한.

## Task 2: FTS5 aliases 컬럼 통합

### 마이그레이션 (V6)

**파일**: `crates/core/migrations/V6__fts_aliases.sql` (신규)

FTS5는 `ALTER TABLE`을 지원하지 않으므로 `chunks_fts`를 DROP 후 aliases 컬럼을 포함하여 재생성했다.

- `chunks` 테이블에 `aliases TEXT` 컬럼 추가
- `chunks_fts` DROP & CREATE (aliases 컬럼 추가)
- 트리거 3개 재생성: `chunks_ai`, `chunks_au`, `chunks_ad`
- `INSERT INTO chunks_fts(chunks_fts) VALUES('rebuild')` — 기존 데이터 FTS 재인덱싱 (코드 리뷰에서 CRITICAL로 지적된 항목)

### 검색 가중치

**파일**: `crates/core/src/search.rs`

```sql
bm25(chunks_fts, 1.0, 0.5, 5.0)
```

aliases 컬럼에 5배 가중치를 부여하여 alias 매칭 문서가 랭킹 상위에 오도록 했다. 기존 `resolve_alias_results()` / `merge_alias_results()` (~130줄)는 삭제했다.

`document_aliases` 테이블은 MCP `nexus_get_backlinks` / `nexus_resolve_alias` 도구 의존성 때문에 유지했다.

## 검토 및 리뷰 과정

### Gemini CCG 리뷰 피드백

- 768은 차원값이며 토큰 제한은 8192로 정정 (계획 문서 오류 수정)
- chunks 테이블에도 aliases 컬럼 필요 — FTS5 외부 콘텐츠 테이블 제약
- 트리거 3개 재생성 필수 (INSERT / UPDATE / DELETE 모두)

### 코드 리뷰 에이전트 피드백

- V6 마이그레이션에 `rebuild` 추가 (CRITICAL)
- `MAX_ALIAS_COUNT` 상수 중복 제거
- `pub` → `pub(crate)` 범위 축소
- `LATEST_VERSION = 6`으로 버전 로그 수정

## 테스트 결과

| 테스트 | 설명 |
|--------|------|
| `test_build_embed_text_*` (×4) | `build_embed_text` 단위 테스트 |
| `test_alias_fts_search_via_bm25` | alias 키워드로 FTS 검색 동작 확인 |
| `test_alias_fts_multi_match_scores_higher` | 다중 alias 매칭 시 score 우위 검증 |

최종 **107/107** 통과.

## 변경 파일 목록

| 파일 | 변경 유형 |
|------|-----------|
| `crates/core/src/index_engine.rs` | 수정 — `build_embed_text` 추출, aliases 저장 |
| `crates/core/src/search.rs` | 수정 — BM25 가중치, alias fallback 제거 |
| `crates/core/src/db/sqlite.rs` | 수정 — V6 마이그레이션 등록 |
| `crates/core/src/test_helpers.rs` | 수정 |
| `crates/core/migrations/V6__fts_aliases.sql` | 신규 |
| `crates/core/tests/integration_test.rs` | 수정 — 통합 테스트 2개 추가 |
| `crates/core/tests/search_scenarios_test.rs` | 수정 |
| `docs/architecture/search-alias-improvement-plan.md` | 신규 — 구현 계획 문서 |

## 관련 문서

- [[검색 품질 개선 — LLM Query Rewriting & Alias 토큰화]]
- [[search-alias-improvement-plan]]
