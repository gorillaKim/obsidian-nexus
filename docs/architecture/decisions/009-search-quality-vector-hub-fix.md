---
title: "ADR-009: 검색 품질 개선 — Micro-chunk 임베딩 스킵 & Title Fallback"
aliases:
  - ADR-009
  - search-quality-vector-hub-fix
  - 검색 품질 개선
  - micro-chunk 임베딩 스킵
tags:
  - architecture
  - decision
  - search
  - embedding
  - quality
created: 2026-03-26
updated: 2026-03-26
status: Accepted
---

<!-- docsmith: auto-generated 2026-03-26 -->

# ADR-009: 검색 품질 개선 — Micro-chunk 임베딩 스킵 & Title Fallback

## 상태

채택 (Accepted)

## 배경

V6 마이그레이션 이후 `decisions/` 폴더에 "채택 (Accepted)" 같은 2~3단어짜리 micro-chunk가 생성되었다. 이 짧은 청크들이 벡터 공간에서 **허브(hub)**를 형성하여 아무 쿼리에나 높은 코사인 유사도를 반환하는 문제가 발생했다.

증상:
- "설정 가이드" 쿼리의 벡터 검색 결과 상위 5개를 `decisions/` 문서가 모두 점유
- "overview 리뉴얼" 쿼리에서 `performance-report.md`가 rank 5에 위치 (기대값: 2 이내)

추가로, `extract_title()`이 H1 헤딩만 파싱하여 docsmith 자동 생성 문서(H1 없음)의 title이 `null`로 저장되는 버그가 함께 확인되었다.

## 결정

### 결정 1: MIN_EMBED_WORD_COUNT = 4

`build_embed_text()` 출력의 단어 수가 4 미만인 청크는 임베딩 생성을 스킵한다.

```rust
const MIN_EMBED_WORD_COUNT: usize = 4;

// build_embed_text() 반환 후
let word_count = embed_text.split_whitespace().count();
if word_count < MIN_EMBED_WORD_COUNT {
    // 임베딩 생성 스킵
    continue;
}
```

**비대칭 처리 의도:** chunk 0(첫 번째 청크)은 `build_embed_text()`가 `제목: {title}\n별칭: {a1, ...}\n{content}` 형태의 prefix를 포함하므로 실제 content가 짧아도 유효 단어 수가 충분히 높다. 따라서 별도 예외 처리 없이 동일한 기준을 적용한다.

### 결정 2: frontmatter title fallback

`extract_title()`이 H1 헤딩 파싱에 실패하면 frontmatter `title` 필드를 fallback으로 사용한다.

```
H1 헤딩 파싱 시도
  → 성공: H1 텍스트 사용
  → 실패: frontmatter title 사용
  → 없음: null (파일명 기반 생성은 호출부 책임)
```

docsmith 자동 생성 문서는 frontmatter에 `title`을 항상 포함하므로 이 fallback으로 null title 버그가 해소된다.

### 결정 3: 대소문자 무관 비교

메타데이터 리랭킹 및 alias resolve에서 대소문자를 무시한다.

- title match boost: `eq_ignore_ascii_case()` 사용
- alias resolve SQL: `WHERE LOWER(alias) = LOWER(?1)` 패턴

## 검토한 대안

### SQL 길이 필터링

```sql
WHERE length(content) < 15
```

**기각 이유:** SQL의 `content` 컬럼은 원본 텍스트이고, 임베딩에 실제로 사용되는 `build_embed_text()` 출력(prefix 포함)을 SQL 레이어에서 알 수 없다. Rust 코드의 단어 수 기준과 불일치가 발생하므로 기각.

### 벡터 검색 min_score 상향

`min_vector_score` 기본값을 0.65에서 0.80으로 올리는 방안. 허브 문서를 걸러내는 효과가 있으나, 합법적인 저점수 관련 문서까지 제외되어 recall이 떨어질 수 있어 근본 해결책으로 부적절하다.

## 결과

전체 재인덱싱(`obs-nexus index --full`) 후:

- "채택" micro-chunk가 임베딩 대상에서 제거됨
- "설정 가이드" 벡터 검색 결과에서 `decisions/` 문서 과점유 해소
- "overview 리뉴얼" 쿼리: `performance-report.md` rank 5 → 2 개선
- docsmith 자동 생성 문서의 title null 버그 해소

## 관련 문서

- [[검색 시스템]]
- [[ADR-008: FTS5 Aliases 컬럼 전략]]
- [[ADR-006: LLM 쿼리 재작성]]
