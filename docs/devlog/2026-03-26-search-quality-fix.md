---
title: "검색 품질 수정 — Micro-chunk 허브 오염, Score Clamp, Title Boost"
aliases:
  - search-quality-fix
  - 검색 품질 수정
  - micro-chunk-hub-fix
  - title-boost-fix
  - 임베딩 허브 오염
created: "2026-03-26"
updated: "2026-03-26"
tags:
  - devlog
  - feature
  - search
  - bugfix
  - embedding
---

<!-- docsmith: auto-generated 2026-03-26 -->

# 검색 품질 수정 — Micro-chunk 허브 오염, Score Clamp, Title Boost

## 배경

V6 마이그레이션 이후 벤치마크 분석에서 검색 정확도 하락이 관측됐다. "설정 가이드"처럼 명확한 의도의 쿼리에서도 `decisions/` 폴더 문서가 상위에 노출되는 이상 현상이 반복됐다.

원인 분석:

- `decisions/` 폴더에 V6 마이그레이션과 함께 추가된 "채택 (Accepted)" 같은 2~3단어짜리 micro-chunk가 임베딩 모델에서 **벡터 허브**가 됨
- 벡터 허브: 코사인 유사도 공간에서 다수 쿼리와 근접해 아무 검색에나 상위 노출되는 문서
- 추가로 부동소수점 정밀도 문제로 벡터 score가 1.0을 초과하는 버그, `extract_title()`이 frontmatter `title` 필드를 무시하는 버그도 발견됨

## 변경 내용

### 1. MIN_EMBED_WORD_COUNT — `crates/core/src/index_engine.rs`

`MIN_EMBED_WORD_COUNT = 4` 상수를 추가했다. `build_embed_text()` 출력 기준 단어 수가 4 미만이면 임베딩을 스킵하고 `None`을 반환한다.

효과: "채택 (Accepted)", "결정됨" 같은 단어 수 부족 chunk가 벡터 DB에 저장되지 않아 허브 오염이 차단된다.

### 2. 벡터 Score Clamp — `crates/core/src/search.rs`

기존 공식:

```
score = 1.0 - (distance * distance) / 2.0
```

수정 후:

```
score = (1.0 - (distance * distance) / 2.0).clamp(0.0, 1.0)
```

L2 정규화 벡터 간 거리의 부동소수점 정밀도 오차로 score가 1.0을 미세하게 초과하던 버그를 수정했다.

### 3. Title Match Boost 강화 — `crates/core/src/search.rs`

| 항목 | 이전 | 이후 |
|------|------|------|
| title 매칭 boost | 0.10 | 0.25 |
| document root chunk boost | 없음 | +0.15 (heading_path가 None인 경우) |

document root chunk는 문서 전체 맥락을 대표하는 chunk로, 섹션 chunk보다 검색 적합성이 높은 경향이 있어 가중치를 추가했다.

### 4. Frontmatter `title` Fallback — `crates/core/src/indexer.rs`

`extract_title()` 함수가 H1 헤딩만 파싱하여 frontmatter `title` 필드를 무시하는 버그가 있었다.

수정: H1 헤딩이 없을 때 frontmatter의 `title` 필드를 fallback으로 사용하도록 파싱 순서를 변경했다.

### 5. 대소문자 무관 매칭

- title match boost 비교: `==` → `eq_ignore_ascii_case` 적용
- `resolve_by_alias` SQL: `LOWER()` 함수 적용으로 alias 대소문자 무관 매칭

## 영향 범위

| 파일 | 변경 내용 |
|------|-----------|
| `crates/core/src/index_engine.rs` | `MIN_EMBED_WORD_COUNT = 4` 상수, 임베딩 스킵 로직 |
| `crates/core/src/search.rs` | score clamp, title boost 강화, 대소문자 매칭 |
| `crates/core/src/indexer.rs` | `extract_title()` frontmatter title fallback |

## 검증 결과

전체 재인덱싱 실행 후 검색 품질 검증:

- "설정 가이드" 쿼리: `guides/configuration.md` rank 4 등장, `decisions/` 문서 사라짐
- "overview 리뉴얼" 쿼리: `performance-report.md` rank 5 → rank 2로 개선 (title null 버그 수정 효과)

## 릴리즈

v0.5.3 → v0.5.4

## 교훈

- 임베딩 벡터 허브 문제는 짧은 chunk를 무분별하게 인덱싱할 때 발생한다. 최소 단어 수 기준으로 필터링하는 것이 효과적인 방어책이다.
- score 정규화 후에도 수치 오버플로우가 발생할 수 있다. 출력 경계에 `clamp`를 적용하는 습관이 필요하다.
- title boost는 단순하지만 검색 관련도에 직접적인 효과가 크다. document root chunk를 따로 구분해 가중치를 주는 전략은 재사용할 만하다.
- frontmatter 파싱은 H1과 title 필드 중 하나만 존재하는 문서가 현실에 많다. 두 경로 모두 처리해야 안정적이다.

## 관련 문서

- [[검색 품질 개선 — LLM Query Rewriting & Alias 토큰화]]
- [[search-system]]
- [[module-map]]
