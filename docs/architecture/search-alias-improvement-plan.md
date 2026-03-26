---
title: 검색 alias 개선 구현 계획
tags: [search, alias, embedding, fts5, plan]
created: 2026-03-26
status: planned
---

# 검색 Alias 개선 구현 계획

## 배경

Gemini 제안 검토 결과, 현재 alias 처리에 두 가지 개선 포인트가 확인됨:

1. **임베딩 텍스트에 alias 미포함** — 벡터 검색에서 alias로 문서를 찾을 수 없음
2. **alias 다중 매칭 점수 미반영** — FTS alias 검색이 LIKE 기반이라 매칭 수와 무관하게 score=1.0 고정

---

## Task 1. 임베딩 텍스트 강화

### 현재 상태

`crates/core/src/index_engine.rs:166`

```rust
// 첫 번째 청크에만 제목 + 파일명 주입
let content = if i == 0 && !doc_title.is_empty() {
    format!("{} {}\n{}", doc_title, file_stem, chunk.content)
} else {
    chunk.content.clone()
};
```

임베딩은 이 `content`를 그대로 사용 (`embed_text(config, &chunk.content)`).
**aliases가 임베딩 입력에 포함되지 않아** 벡터 공간에서 alias로 유사도 매칭이 안 됨.

### 목표 상태

```
임베딩 입력 = "제목: {title}\n별칭: {alias1}, {alias2}\n{본문}"
```

### 구현 범위

**파일 1개, 변경 지점 1곳**

`crates/core/src/index_engine.rs` — `index_single_file()` 함수 내

```
변경 전:
  embeddings 생성 루프 → chunk.content 직접 사용

변경 후:
  첫 번째 청크 임베딩 시 aliases prefix 주입
  나머지 청크는 기존 그대로
```

구체적 변경:

```rust
// aliases 수집 (이미 parsed.aliases로 사용 가능)
let alias_prefix = if !parsed.aliases.is_empty() {
    format!("별칭: {}\n", parsed.aliases.join(", "))
} else {
    String::new()
};

// 임베딩용 텍스트 (FTS 저장용 content와 분리)
let embed_text = if i == 0 {
    format!("제목: {} {}\n{}{}",
        doc_title, file_stem, alias_prefix, chunk.content)
} else {
    chunk.content.clone()
};

// embed_text로 임베딩, content는 기존 그대로 DB 저장
match crate::embedding::embed_text(config, &embed_text) { ... }
```

> **핵심**: FTS에 저장되는 `content`와 임베딩 입력 텍스트를 분리.
> FTS는 오염 없이 유지, 벡터만 alias 의미를 흡수.

### 난이도 / 복잡도

| 항목 | 평가 |
|------|------|
| 난이도 | ⭐ 낮음 |
| 변경 파일 수 | 1개 |
| 마이그레이션 필요 | 없음 (재인덱싱만 필요) |
| 부작용 위험 | 낮음 — FTS 저장값 불변, 임베딩만 변경 |
| 재인덱싱 비용 | 전체 문서 × Ollama API 호출 (기존 인덱싱과 동일 경로) |

### 주의사항

- 임베딩 입력이 길어질수록 nomic-embed-text 품질이 약간 떨어질 수 있음 (실제 토큰 제한: **8192**, 768은 차원값)
- alias가 많은 문서(10개+)는 prefix가 길어지므로 최대 5개로 제한 고려 (품질/성능 가이드라인)
- 기존 vec_chunks는 재인덱싱 전까지 alias 없는 임베딩 유지 (점진적 전환 허용)

---

## Task 2. FTS5 aliases 컬럼 통합

### 현재 상태

**구조:**
```
document_aliases 테이블 (V4 마이그레이션)
  ├── document_id
  └── alias (TEXT)

chunks_fts (FTS5 virtual table)
  ├── content
  └── heading_path
```

**검색 흐름 (`search.rs`):**
1. `fts_search()` 또는 `hybrid_search()` 실행
2. `resolve_alias_results()` 별도 실행 — LIKE 패턴 매칭
   - `%쿼리%`, `%토큰1%`, `%토큰2%` OR 조건
3. alias 결과 **score=1.0 강제** 후 앞에 삽입

**문제:**
- alias가 3개 매칭되든 1개 매칭되든 동일 score=1.0
- LIKE는 FTS5 bm25 점수 체계 밖 → 본문 검색 결과와 자연스러운 순위 통합 불가
- 쿼리 2번 실행 (FTS + LIKE 별도)

### 목표 상태

```sql
CREATE VIRTUAL TABLE chunks_fts USING fts5(
    content,
    heading_path,
    aliases,          -- 신규 컬럼
    tokenize='unicode61'
);
```

```sql
-- aliases 컬럼에 5배 가중치
SELECT bm25(chunks_fts, 1.0, 0.5, 5.0) as score ...
WHERE chunks_fts MATCH ?
```

alias 다중 매칭 → bm25 자동 반영, 단일 쿼리로 통합.

### 구현 범위

**마이그레이션:**

`crates/core/migrations/V6__fts_aliases.sql`

```sql
-- 1. 기존 FTS5 테이블 재생성 (컬럼 추가 불가 → DROP & CREATE)
DROP TABLE IF EXISTS chunks_fts;

CREATE VIRTUAL TABLE chunks_fts USING fts5(
    content,
    heading_path,
    aliases,
    tokenize='unicode61'
);

-- 2. 기존 트리거 재생성 (aliases 컬럼 포함)
DROP TRIGGER IF EXISTS chunks_ai;
DROP TRIGGER IF EXISTS chunks_ad;
DROP TRIGGER IF EXISTS chunks_au;

-- (트리거는 aliases를 '' 기본값으로 삽입 — 재인덱싱 시 채워짐)
```

> FTS5는 `ALTER TABLE ADD COLUMN` 불가 → 기존 인덱스 데이터 손실.
> 재인덱싱 필수.

**Rust 변경 파일:**

| 파일 | 변경 내용 |
|------|-----------|
| `index_engine.rs` | chunks INSERT 시 aliases 컬럼 값 전달 |
| `db/sqlite.rs` | 트리거 재생성 로직 (마이그레이션 V6 처리) |
| `search.rs` | `resolve_alias_results()` 제거, bm25 가중치 쿼리로 교체, `merge_alias_results()` 제거 또는 단순화, FTS 쿼리를 `"{query}" OR aliases:"{query}"` 형태로 변경 |

**`document_aliases` 테이블 보존 여부:**

`nexus_get_backlinks`, `nexus_resolve_alias` MCP 도구가 `document_aliases` 직접 조회 — **테이블 유지 필요**.
FTS5 컬럼과 병렬 운영. (FTS5는 검색용, document_aliases는 메타데이터/역방향 링크용)

### 검색 쿼리 변경 예시

```sql
-- 변경 전: 별도 LIKE 쿼리
SELECT ... FROM document_aliases WHERE alias LIKE '%키워드%'

-- 변경 후: FTS5 통합 (aliases 컬럼 가중치 5배)
SELECT chunks_fts.rowid, bm25(chunks_fts, 1.0, 0.5, 5.0) as score
FROM chunks_fts
WHERE chunks_fts MATCH '키워드'
ORDER BY score
```

### 난이도 / 복잡도

| 항목 | 평가 |
|------|------|
| 난이도 | ⭐⭐⭐ 높음 |
| 변경 파일 수 | 4개 (마이그레이션 + 3개 Rust) |
| 마이그레이션 필요 | **전체 재인덱싱 필수** |
| 부작용 위험 | 중간 — FTS5 재생성 중 검색 불가 구간 발생 |
| 기존 alias 검색 동작 변경 | score 고정 → bm25 변동 (결과 순서 달라질 수 있음) |
| document_aliases 테이블 | 유지 (MCP 도구 의존성) |

### 주의사항

- **`chunks` 테이블 aliases 컬럼 추가 필요** — `chunks_fts`가 `content=chunks` 외부 콘텐츠 테이블이므로, FTS5 aliases 컬럼의 원본 데이터가 `chunks` 테이블에도 있어야 트리거 동기화가 가능함. 두 가지 옵션:
  - **옵션 A (권장)**: `chunks` 테이블에 `aliases TEXT` 컬럼 추가 — 첫 번째 청크에만 alias 저장, 나머지는 NULL
  - **옵션 B**: FTS5를 독립 테이블(contentless)로 전환 — 트리거 동기화 로직 전면 수정 필요, 복잡도 높음
- **트리거 3개 재생성 필수** — `chunks_ai`, `chunks_au`, `chunks_ad` 모두 aliases 컬럼 포함하도록 수정. 누락 시 FTS와 원본 테이블 간 동기화 깨짐
- 마이그레이션 실행 후 기존 사용자 DB는 재인덱싱 없이는 aliases 컬럼이 빈 상태
  → `nexus_index_project` 재실행 유도 또는 마이그레이션 시 자동 backfill 고려
- bm25 가중치 수치(5.0)는 실험으로 튜닝 필요
- RRF 기반 하이브리드 검색은 순위 기반이므로 bm25 가중치 스케일 변화에 견고함 — 별도 정규화 불필요

---

## 구현 순서 권고

```
Task 1 (임베딩 강화)  →  Task 2 (FTS5 통합)
     1~2시간                  반나절~1일
```

Task 1이 독립적이고 리스크가 낮으므로 먼저 적용.
Task 2는 Task 1 완료 후 별도 브랜치에서 진행 권장.

---

## 기대 효과 요약

| 검색 시나리오 | 개선 전 | Task 1 후 | Task 2 후 |
|--------------|---------|-----------|-----------|
| alias와 동의어로 벡터 검색 | ❌ 미검색 | ✅ 검색됨 | ✅ 검색됨 |
| alias 여러 개 매칭 시 순위 | ❌ 동일 score | ❌ 동일 score | ✅ 매칭 수 반영 |
| alias 키워드 FTS 검색 | ✅ LIKE로 검색 | ✅ LIKE로 검색 | ✅ bm25로 더 정밀 |
| 검색 쿼리 횟수 | 2번 | 2번 | 1번 |
