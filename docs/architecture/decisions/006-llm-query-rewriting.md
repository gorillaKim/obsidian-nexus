---
title: "LLM 기반 쿼리 재작성 도입 — Ollama /api/generate 활용"
aliases:
  - llm-query-rewriting
  - query-rewriting-adr
  - LLM query rewriting 설계
  - 쿼리 재작성 ADR
tags:
  - decision
  - search
  - llm
  - architecture
created: 2026-03-25
updated: 2026-03-25
---

<!-- docsmith: auto-generated 2026-03-25 -->

# LLM 기반 쿼리 재작성 도입 — Ollama /api/generate 활용

검색 시 사용자의 자연어 쿼리와 문서의 기술 용어 간 semantic gap을 해소하기 위해, Ollama `/api/generate` 엔드포인트를 활용한 LLM 기반 쿼리 재작성(Query Rewriting) 모듈을 도입하기로 한 결정을 기록합니다.

## 상태

채택 (Accepted)

## 배경

사용자가 UX 관점의 자연어("overview 페이지 리뉴얼")로 검색할 때, 실제 문서는 기술 용어("performance-report")로 작성되어 있어 키워드 매칭이 실패합니다. vector 검색도 이 경우를 완전히 해결하지 못했는데, 임베딩 모델이 한국어 도메인 의도를 정확히 파악하지 못해 유사도가 낮게 산출되었습니다.

alias를 수동으로 추가하는 방식이 부분적으로 유효하지만, 문서마다 작업자가 직접 관리해야 하는 운영 부담이 존재합니다.

## 검토한 대안

### 방법 A: 사용자가 쿼리를 직접 조정

검색어를 기술 용어로 바꿔 재시도하도록 유도합니다.

**문제**: 근본 원인을 해결하지 못하며, 사용자에게 도메인 내부 용어를 알도록 요구합니다. UX가 나쁩니다.

### 방법 B: alias 수동 추가

문서 frontmatter의 `aliases` 필드에 자연어 표현을 미리 등록합니다.

**장점**: 검색 결과가 즉각적으로 개선됩니다.

**문제**: 문서마다 수동 작업이 필요하며, 새 문서 추가 시 지속적인 유지보수가 필요합니다.

### 방법 C: LLM Query Rewriting — 채택

사용자 쿼리를 LLM에 전달하여 도메인 기술 용어로 재작성한 뒤 검색에 활용합니다.

**장점**:
- alias 없이도 작동 — 문서 수동 편집 불필요
- 기존 Ollama 서버 재활용 (추가 인프라 없음)
- 실패 시 원본 쿼리로 fallback하여 서비스 무중단
- opt-in 설계로 기존 사용자에게 영향 없음

## 결정

방법 C를 채택합니다. 기존 임베딩 서버(`localhost:11434`)에서 `/api/generate` 엔드포인트를 추가 호출하는 방식으로, 인프라 변경 없이 쿼리 품질을 개선합니다. 기능은 기본 비활성화(opt-in)로 제공하여 Ollama가 없는 환경에서도 정상 동작을 보장합니다.

## 구현

### 신규 모듈

`crates/core/src/llm.rs` — Ollama `/api/generate` 호출 및 응답 sanitize 담당

### 설정

`config.toml`에 `[llm]` 섹션을 추가하여 활성화합니다.

```toml
[llm]
enabled = false          # 기본값: 비활성화 (opt-in)
model = "llama3.2"
timeout_secs = 5         # CPU Ollama 환경에서는 15 권장
```

### MCP 인터페이스

`nexus_search` 도구에 `rewrite_query` 파라미터를 노출합니다. 호출자가 명시적으로 활성화하지 않으면 config 기본값을 따릅니다.

### 출력 sanitize 규칙

- 첫 번째 라인만 추출
- 원본 쿼리 길이의 3배를 상한으로 잘라냄
- LLM 응답이 비어있거나 타임아웃 발생 시 원본 쿼리 그대로 사용

## 결과

- 자연어 쿼리 → 기술 용어 자동 변환으로 검색 召환率 개선
- alias 없는 문서에서도 의도 기반 검색 가능
- Ollama 미설치 환경에서도 graceful degradation 보장

## 한계 및 후속 과제

- Ollama 설치가 선택적으로 필요 (필수 의존성 아님)
- CPU Ollama 환경에서 latency가 높을 수 있어 타임아웃 튜닝 필요
- 재작성 품질은 선택한 LLM 모델에 의존 — 모델별 프롬프트 최적화 여지 있음

## 관련 문서

- [[search-system]]
- [[module-map]]
- [[005-attention-needed-docs]]
