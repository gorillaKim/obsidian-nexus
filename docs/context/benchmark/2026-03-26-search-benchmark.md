---
title: "Search Benchmark Report — 2026-03-26"
date: "2026-03-26T00:00:00"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-26"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-26
> - **Projects**: Obsidian Nexus Docs, Obsidian Vault, brain, test-vault, xpert-da-web, xpert-na-web (6개)
> - **Scenarios**: 7/7

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | ~350 | ~550 | 36% | 불일치 |
| 2 | 개념 검색 | 1 | 4 | ~500 | ~2,525 | 80% | 불일치 |
| 3 | 멀티홉 탐색 | 3 | 5 | ~588 | ~1,475 | 60% | 일치 |
| 4 | 별칭 해소 | 1 | 2 | ~38 | ~225 | 83% | 일치 |
| 5 | 섹션 단위 조회 | 1 | 1 | ~88 | ~1,250 | **93%** | 일치 |
| 6 | 크로스 프로젝트 | 1 | 6 | ~875 | ~900 | 3% | 일치 |
| 7 | 태그 필터링 | 1 | 2 | ~3 | ~200 | — | ❌ MCP 버그 |

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 9 | 22 | 59% 적음 |
| 추정 토큰 | ~2,442 | ~7,125 | **~66% 절약** |
| 정확도 일치율 | 4/7 | 6/7 | FS가 우세 |

> [!tip] 핵심 관찰
> - 토큰 효율은 MCP가 우세 (평균 66% 절약)
> - **섹션 조회(S5)**: 93% 절약 — MCP의 가장 강력한 강점
> - **S1·S2**: MCP 검색 관련성이 낮아 FS가 정확도 우위
> - **S7**: `tags` 파라미터 배열 전달 시 필터링 미동작 — 버그 확인

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회**: 파일 전체 읽기 대비 93% 토큰 절약
> - **그래프 탐색**: backlinks/links로 관련 문서 자동 발견 (위키링크 수동 파싱 불필요)
> - **별칭 해소**: 영문 alias → 한글 문서 즉시 접근, 83% 절약
> - **크로스 볼트 단일 쿼리**: 6개 볼트를 1회 호출로 검색 (FS는 6회 필요)
> - **구조화 결과**: score, tags, backlink_count, snippet 일괄 반환

---

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: ~1,400 chars (≈ 350 tokens), 1회 호출
> - **발견 문서**:
>   1. `architecture/decisions/006-llm-query-rewriting.md` (score: 0.0131)
>   2. `architecture/decisions/001-view-cooldown-atomic-sql.md` (score: 0.0129)
>   3. `architecture/decisions/002-attention-docs-thresholds-constants.md` (score: 0.0129)
>   4. `architecture/decisions/008-fts5-aliases-column-strategy.md` (score: 0.0124)
>   5. `architecture/decisions/007-desktop-onboarding-button.md` (score: 0.0123)

> [!example]- Filesystem 결과
> - **도구**: `Grep "설정" --glob "*.md"` → `Read top file 50줄`
> - **응답**: ~700 chars (grep 33파일) + ~1,500 chars (read) = ~2,200 chars (≈ 550 tokens), 2회 호출
> - **발견 문서**: 33개 파일 목록 (architecture/search-system.md 포함)

> [!note] 비교 분석
> **불일치**: MCP top-5가 모두 `decisions/` 결정 문서. "설정 가이드"에 해당하는 `guides/configuration.md`, `guides/getting-started.md`를 놓침. FS도 33개 파일 목록만 반환되어 단독으론 불충분하지만 `search-system.md`가 상위에 포함됨. **Score가 전반적으로 낮음(0.012~0.013)** — 관련 문서가 없거나 쿼리 벡터 공간과 멀 가능성.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (hybrid, limit=5)
> - **파라미터**: `query="의미 유사도 검색 원리"`
> - **응답**: ~2,000 chars (≈ 500 tokens), 1회 호출
> - **발견 문서**:
>   1. `integrations/mcp-tools.md` (score: 0.0143)
>   2. `architecture/database-schema.md` (score: 0.0132)
>   3. `guides/getting-started.md` (score: 0.0121)
>   4. `devlog/2026-03-23-dashboard-ranking-empty-bug.md` (score: 0.0115)
>   5. `guides/installation.md` (score: 0.0113)

> [!example]- Filesystem 결과
> - **도구**: `Grep "벡터|embedding|유사도" --glob "**/*.md"` (3패턴 병렬)
> - **응답**: ~700 chars × 3 (각 25파일 목록) + Read 2파일 ~8,000 chars = ~10,100 chars (≈ 2,525 tokens), 4회 호출
> - **발견 문서**: `architecture/search-system.md`, `architecture/search-system-deep-dive.md`, `architecture/decisions/embedding-model-options.md` 등 25파일

> [!note] 비교 분석
> **불일치**: 핵심 문서 `architecture/search-system.md`(벡터 검색 전용 섹션 포함)를 MCP가 top-5에 포함하지 못함. FS는 패턴 매칭으로 바로 발견. 반면 MCP는 80% 토큰 절약. **쿼리 재작성(LLM query rewriting) 활성화 시 개선 기대**.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="context/benchmark/2026-03-21-search-benchmark.md"`
> - **응답**: ~2,000 + ~50 + ~300 = ~2,350 chars (≈ 588 tokens), 3회 호출
> - **search top-1**: `context/benchmark/2026-03-21-search-benchmark.md` (score: 0.0107)
> - **backlinks**: [] (없음)
> - **links**: `01-아키텍처`, `02-검색-시스템`, `03-MCP-도구-레퍼런스`

> [!example]- Filesystem 결과
> - **도구**: `Grep "아키텍처"` → Read top파일 → 위키링크 수동 파싱 → Glob × 3링크
> - **응답**: ~600 (grep 19파일) + ~5,000 (read 벤치마크 파일) + ~300 (3× Glob) = ~5,900 chars (≈ 1,475 tokens), 5+회 호출
> - **발견 문서**: 19개 파일에서 `아키텍처` 매칭, 링크 파싱 후 3개 관련 문서 추가 발견

> [!note] 비교 분석
> **일치**: 양쪽 모두 관련 문서 발견 가능. MCP의 `nexus_get_links`는 위키링크를 이미 파싱된 구조로 반환 — FS의 `[[...]]` 수동 파싱 + Glob 조합보다 60% 적은 토큰으로 동일 정보 획득. 단, search top-1이 아키텍처 문서 자체(architecture/architecture.md)가 아닌 벤치마크 문서인 점은 아쉬움.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: ~150 chars (≈ 38 tokens), 1회 호출
> - **발견 문서**: `architecture/search-system.md` (즉시 반환)

> [!example]- Filesystem 결과
> - **도구**: `Grep "Search System" --glob "**/*.md"` → `Read 상위파일 20줄`
> - **응답**: ~400 chars (6파일 목록) + ~500 chars (frontmatter 20줄) = ~900 chars (≈ 225 tokens), 2회 호출
> - **발견 문서**: `architecture/search-system.md` (frontmatter의 `aliases: - Search System` 확인)

> [!note] 비교 분석
> **일치**: 양쪽 모두 `architecture/search-system.md` 식별. MCP는 83% 토큰 절약 + 경로를 직접 반환. FS도 가능하지만 frontmatter 파싱 단계 추가 필요. Grep 결과 6개 파일에서 "Search System" 매칭 → 수동 판단 필요.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="architecture/search-system.md", heading="하이브리드 검색"`
> - **응답**: ~350 chars (≈ 88 tokens), 1회 호출
> - **반환 내용**: RRF 공식, hybrid_weight 설명 (섹션만 정확히 추출)

> [!example]- Filesystem 결과
> - **도구**: `Read "docs/architecture/search-system.md"` (전체)
> - **응답**: ~5,000 chars (162줄, ≈ 1,250 tokens), 1회 호출
> - **반환 내용**: 파일 전체 (필요 섹션은 전체의 약 7%)

> [!note] 비교 분석
> **일치 + 압도적 MCP 우위**: 동일 정보를 얻는 데 **93% 토큰 절약**. 섹션이 파일의 7%임에도 FS는 전체를 읽어야 함. 문서가 클수록 MCP `nexus_get_section`의 효율이 더 극적으로 증가.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: ~3,500 chars (≈ 875 tokens), 1회 호출
> - **발견 문서**: 6개 프로젝트 중 **2개 프로젝트**에서 10개 결과
>   - Obsidian Nexus Docs: `overview/glossary.md`, `guides/getting-started.md`, `guides/installation.md`, `guides/configuration.md`, `integrations/mcp-tools.md`, ...
>   - brain: `Engineering/Tools/Obsidian Nexus - Claude CLI 기본 도구 설정.md`

> [!example]- Filesystem 결과
> - **도구**: `Grep "MCP" --glob "**/*.md"` × 6개 볼트 경로 (각 1회)
> - **응답**: ~3,600 chars 추정 (6회 grep, 볼트당 ~600 chars), 6회 호출
> - **발견 문서**: 볼트별 독립 결과 목록 → 수동 병합 필요
> - **참고**: `/Users/madup/Documents/Obsidian Vault` grep 실행 결과 0 매칭

> [!note] 비교 분석
> **일치**: 토큰 절약은 상대적으로 낮음(~3%) — 크로스 볼트 검색에서도 결과 수가 많아 MCP 응답도 큼. 그러나 **호출 횟수 차이(1 vs 6)** 와 자동 점수 기반 랭킹은 MCP 고유 가치. FS는 볼트 경로를 사전에 알아야 하고 결과를 수동 병합해야 함.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: `[]` (0 results), 1회 호출
> - **상태**: ❌ **버그 — 태그 필터링 미동작**

> [!example]- Filesystem 결과
> - **도구**: `Grep "  - mcp" --glob "**/*.md"` → 수동 검색
> - **응답**: ~450 chars (9파일 목록), 1+회 호출
> - **발견 문서**: 9개 mcp 태그 문서 (integrations/mcp-tools.md, integrations/subagent-mcp-setup.md 등)

> [!note] 비교 분석
> **불일치 (MCP 버그)**: `tags` 파라미터를 배열(`["mcp"]`)로 전달했음에도 MCP가 빈 결과 반환. 동일 쿼리를 `tags` 없이 실행 시 정상 결과 반환. FS는 frontmatter에서 `  - mcp` 직접 grep으로 9개 파일 발견. **태그 필터링 버그 수정 필요** — MCP tool 스키마 vs 실제 파라미터 파싱 불일치 의심.

---

## 발견된 이슈

> [!warning] S7 태그 필터 버그
> `nexus_search(tags=["mcp"])` 호출 시 빈 배열 반환. `enrich=true`(기본값)임에도 태그 데이터 미활용. 재현: `tags` 파라미터 있는 경우와 없는 경우 비교 → 태그 파라미터 유무가 결과에 영향 없음(빈 배열 고정).

> [!warning] S1·S2 관련성 저하
> "설정 가이드", "의미 유사도 검색 원리" 쿼리에서 예상 문서(`guides/configuration.md`, `architecture/search-system.md`)가 top-5에 미포함. Score 범위 0.011~0.014로 전체적으로 낮음. LLM 쿼리 재작성(`rewrite_query=true`) 또는 벡터 재인덱싱이 도움될 수 있음.

## 관련 문서

- [[architecture/search/README|Search Architecture]]
- [[architecture/search-system|검색 시스템]]
- [[integrations/mcp-tools|MCP 도구 레퍼런스]]
