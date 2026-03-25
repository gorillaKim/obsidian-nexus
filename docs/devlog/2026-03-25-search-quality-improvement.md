---
title: "검색 품질 개선 — LLM Query Rewriting & Alias 토큰화"
aliases:
  - search-quality-improvement
  - llm-query-rewriting
  - 검색 품질 개선
  - LLM 쿼리 재작성
  - alias 토큰화
created: "2026-03-25"
updated: "2026-03-25"
tags:
  - devlog
  - feature
  - search
  - llm
---

<!-- docsmith: auto-generated 2026-03-25 -->

# 검색 품질 개선 — LLM Query Rewriting & Alias 토큰화

## 배경

실무 사용 중 검색 품질 저하 피드백이 접수됐다. "overview 페이지 리뉴얼"로 검색했을 때 전혀 무관한 `component-conventions.md`가 상위에 노출되는 문제였다.

원인 분석:

- Vector 검색이 `component-conventions.md`를 score 0.705로 1위 반환 (무관 문서)
- Keyword BM25 score 음수 (-10.8): "리뉴얼" 단어가 문서 본문에 없음
- Hybrid = 두 문제 합산 → 최종 score 0.01대
- 근본 원인: 사용자 자연어(UX 용어) ↔ 문서 기술 용어 간 semantic gap

## 변경 내용

### 1. LLM Query Rewriting — `crates/core/src/llm.rs` (신규)

Ollama `/api/generate` 엔드포인트를 활용해 사용자 쿼리를 검색에 유리한 형태로 재작성한다. 기존 임베딩 인프라(Ollama 서버)를 그대로 재활용한다.

활성화 방법:

- `config.toml`의 `[llm]` 섹션에서 글로벌 활성화: `enabled = true`, `model = "mistral"`
- `nexus_search(rewrite_query=true)`로 요청별 활성화

LLM 호출 실패 시 원본 쿼리로 graceful fallback하여 서비스 무중단을 보장한다.

보안 처리 (defense-in-depth):

- 출력 sanitize: 첫 라인만 추출
- 원본 길이 3배 상한 (프롬프트 인젝션 방어)
- 멀티라인 차단
- 타임아웃 config화: `timeout_secs` 필드 (기본 5초, CPU Ollama 환경은 15초로 조정 가능)

### 2. Alias 토큰화 매칭 강화 — `crates/core/src/search.rs`

기존: `LIKE '%overview 페이지 리뉴얼%'` 단일 패턴 (전체 문자열 일치만 허용)

개선: 쿼리를 토큰으로 분리하여 각각 OR 조건으로 검색

```
"overview 페이지 리뉴얼"
→ LIKE '%overview%' OR LIKE '%페이지%' OR LIKE '%리뉴얼%'
```

N+1 SQL 문제도 동시에 해결했다. 기존에는 토큰 수만큼 개별 SQL을 실행했으나, `rusqlite::types::Value` 열거형과 `params_from_iter`를 활용해 단일 OR 쿼리로 통합했다.

### 3. Alias Score 버그 수정

원인: `enrich_results`가 마지막에 score로 재정렬 → alias 결과(score 0.0)가 하위로 밀림

수정: `merge_alias_results`에서 alias 결과 score를 1.0으로 초기화하여 정상 노출 보장

### 4. MCP 도구 확장 — `nexus_search`

`rewrite_query: boolean` 파라미터를 추가했다. keyword / vector / hybrid 모든 모드에서 dispatch 전 공통 처리된다.

### 영향 범위

- `crates/core/src/llm.rs`: 신규 파일 (LLM 클라이언트)
- `crates/core/src/search.rs`: alias 토큰화, score 버그 수정
- `crates/mcp-server/src/main.rs`: `nexus_search`에 `rewrite_query` 파라미터 추가
- `config.toml`: `[llm]` 섹션 추가

## 코드 리뷰 반영 (6건)

코드 리뷰(CCG)에서 발견된 이슈를 모두 수정했다.

| 심각도 | 내용 | 조치 |
|--------|------|------|
| CRITICAL | 프롬프트 인젝션 방어 | 멀티라인 차단 + 길이 3배 상한 |
| HIGH | blocking HTTP 타임아웃 하드코딩 | config화 (30s → 기본 5s) |
| HIGH | `unwrap_or_default()` 사용 | `?` 에러 전파로 교체 (컨벤션 준수) |
| MEDIUM | N+1 SQL | `params_from_iter` 단일 OR 쿼리 |
| MEDIUM | `rewrite_query` hybrid 전용 | 모든 검색 모드 공통 적용 |
| LOW | 루프 불변식 내부 위치 | N+1 수정과 함께 해결 |

## 테스트

신규 테스트 파일: `crates/core/tests/search_scenarios_test.rs`

12개 시나리오 테스트를 추가했다.

- 짧은 쿼리 (≤2자) 프리픽스 매칭
- 문장 쿼리 토큰 분리
- alias 토큰 OR 매칭
- 한영 혼합 쿼리
- 크로스 프로젝트 검색

전체 통과 확인.

## 실제 데이터 적용

`xpert-na-web` 프로젝트의 `features/performance-report.md`에 aliases를 추가했다.

```yaml
aliases:
  - overview
  - overview 페이지
  - 리뉴얼
  - 대시보드 리뉴얼
```

재인덱싱 후 "overview 페이지 리뉴얼" 검색 결과 개선을 확인했다.

## 발생한 문제와 해결

1. **`NexusError::Embedding` 미존재**: `NexusError::Search`로 대체
2. **`Box<dyn ToSql>` 수명 문제**: `rusqlite::types::Value` 열거형으로 교체 후 `params_from_iter` 적용
3. **alias score 0.0 → 재정렬 후 하위 밀림**: `merge_alias_results`에서 score 1.0으로 초기화

## 교훈

- semantic gap 문제는 단순 임베딩 개선만으로 해결이 어렵다. LLM을 활용한 쿼리 재작성이 실용적인 단기 해법이다.
- alias 매칭은 전체 문자열 일치보다 토큰 단위 OR 매칭이 실제 사용 패턴에 훨씬 잘 맞는다.
- graceful fallback 설계는 LLM처럼 외부 의존성이 있는 기능에서 필수다. 기능이 없어도 서비스는 동작해야 한다.
- score 기반 재정렬은 alias 결과처럼 별도 경로로 추가된 결과의 점수를 명시적으로 초기화하지 않으면 의도치 않게 하위로 밀린다.

## 관련 문서

- [[search-system]]
- [[Dashboard 인기 문서 랭킹 기능 구현]]
- [[module-map]]
