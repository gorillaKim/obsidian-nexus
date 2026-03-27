---
title: "Search Benchmark Report — 2026-03-27"
date: "2026-03-27T00:00:00"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-27"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-27
> - **Projects**: Obsidian Nexus Docs, brain, mad-de, xpert-da-web, xpert-na-web (5개 등록)
> - **Scenarios**: 8/8 (S1~S7 + S8 TOC)
> - **Vault 경로**: `/Users/madup/gorillaProject/obsidian-nexus/docs`

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2+Read | ~600 | ~500+Read | — | MCP ✓, FS ✗ (미발견) |
| 2 | 개념 검색 | 1 | 1+Read | ~600 | ~1,975 | **70%** | 둘 다 ✓ |
| 3 | 멀티홉 탐색 | 3 | 6+ | ~475 | ~2,100 | **77%** | 둘 다 ✓ |
| 4 | 별칭 해소 | 1 | 1+Read×3 | ~50 | ~300 | **83%** | MCP ✓, FS △ (오탐) |
| 5 | 섹션 단위 조회 | 1 | 1(전체) | ~70 | ~1,875 | **96%** | 둘 다 ✓ |
| 6 | 크로스 프로젝트 | 1 | 5 | ~1,000 | ~625 | — | MCP ✓, FS △ (단일볼트) |
| 7 | 태그 필터링 | 1 | 2 | ~600 | ~225 | — | MCP ✓, FS △ (불완전) |
| 8 | TOC + 섹션 | 2 | 1(전체) | ~275 | ~1,875 | **85%** | 둘 다 ✓ |

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 (S1~S7) | **9회** | **21회+** | 57% 적음 |
| 추정 토큰 (S1~S7) | **~3,395** | **~7,600** | **~55% 절약** |
| 정확도 우위 | 7/7 ✓ | 4/7 ✓ (3개 오탐/미발견) | MCP 압도적 우위 |

> [!tip] 핵심 발견
> - **섹션 조회(S5, S8)**: MCP가 96%/85% 토큰 절약 — 단 1개 섹션이 필요할 때 파일 전체 읽기는 낭비
> - **별칭 해소(S4)**: FS는 "Search System" 검색에서 벤치마크 파일들만 반환 (오탐). MCP는 즉시 정답
> - **키워드 검색(S1)**: `guides/configuration.md`가 FS top-10에 미등장 — MCP만 정확 발견
> - **크로스 프로젝트(S6)**: FS는 볼트별 별도 grep 필요, 통합 랭킹 불가

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> 1. **섹션 단위 조회**: 파일 전체 읽기 대비 최대 96% 토큰 절약 (`nexus_get_section`, `nexus_get_toc`)
> 2. **그래프 탐색**: backlinks/links로 관련 문서 자동 발견 — FS는 [[wikilink]] 파싱을 수동으로 해야 함
> 3. **별칭 해소**: 영문 alias → 한글 문서 즉시 O(1) 접근 (`nexus_resolve_alias`)
> 4. **크로스 볼트 단일 쿼리**: 5개 볼트를 1회 호출로 통합 랭킹 검색
> 5. **태그+쿼리 동시 필터링**: frontmatter multiline 블록 없이도 정확한 태그 필터 (`tags=["mcp"]`)
> 6. **의미 검색**: 벡터/유사도 키워드 없는 자연어 쿼리에서도 관련 문서 발견 (hybrid mode)

---

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: ~2,400 chars (≈ 600 tokens), 1회 호출
> - **발견 문서**:
>   - rank 1~3: `architecture/decisions/010-embedding-context-prefix.md` (3개 청크)
>   - **rank 4**: `guides/configuration.md` § "설정 가이드 > 설정 파일 위치" ← **정답**
>   - rank 5: `guides/getting-started.md`

> [!example]- Filesystem 결과
> - **도구**: Grep("설정") + Grep("가이드")
> - **응답**: ~600+600=1,200 chars (≈ 300 tokens), 2회 호출
> - **발견 문서**: 각각 10개 파일 (limit 10 truncated) — `guides/configuration.md` 미등장
> - **추가 필요**: Read로 각 파일 frontmatter 확인 → +~2,000 chars

> [!note] 비교 분석
> FS는 "설정" + "가이드" 각각 grep했지만 두 단어가 공존하는 `guides/configuration.md`가 결과 상위 10개 안에 포함되지 않았다. MCP는 랭킹 알고리즘으로 title 매칭(+25% 부스트)이 적용되어 rank 4에서 정확 발견. FS는 추가 Read 없이는 정답 불가.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (mode=hybrid)
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: ~2,400 chars (≈ 600 tokens), 1회 호출
> - **발견 문서**:
>   - rank 1~3: `architecture/decisions/010-embedding-context-prefix.md`
>   - **rank 4**: `architecture/search-system.md` § "1. 키워드 검색 (FTS5)" ← 관련 문서
>   - rank 5: `devlog/2026-03-26-embedding-prefix-search-quality.md`
> - **주목**: rank 5 devlog에 "\"의미 유사도 검색 원리\" 쿼리에서 search-system.md alias 보강으로 rank 1 달성" 기록 — alias 효과 부분 확인

> [!example]- Filesystem 결과
> - **도구**: Grep("벡터|embedding|유사도") → Read(`architecture/search-system.md`)
> - **응답**: ~400 chars (8개 파일 목록) + ~7,500 chars (208줄 전체) = ~7,900 chars (≈ 1,975 tokens), 2회 호출
> - **발견 문서**: `architecture/search-system.md` ← 파일 목록에 포함됨
> - **단점**: 전체 파일 읽어야 원하는 섹션 파악 가능

> [!note] 비교 분석
> 양쪽 모두 search-system.md를 발견했지만 FS는 전체 파일 Read가 필요해 **70% 더 많은 토큰** 소비. MCP는 hybrid 검색으로 "의미 유사도"라는 키워드 없이도 벡터 유사도로 관련 문서를 정확히 찾음. ADR-010이 rank 1~3을 차지한 건 임베딩 prefix 관련 내용이 쿼리와 높은 의미 유사도를 가지기 때문.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="architecture/search/README.md"`
> - **응답**: ~1,200 + ~400 + ~300 = ~1,900 chars (≈ 475 tokens), 3회 호출
> - **search top-1**: `architecture/search/README.md` (backlink_count: 3)
> - **backlinks (3개)**: `context/benchmark/2026-03-26-search-benchmark.md`, `architecture/search/performance/2026-03-26-search-benchmark.md`, `architecture/search/performance/2026-03-26-search-benchmark-v3.md`
> - **links (4개)**: `008-fts5-aliases-column-strategy`, `search-system-deep-dive`, `search-system`, `search-alias-improvement-plan`

> [!example]- Filesystem 결과
> - **도구**: Grep("아키텍처") → Read(top 파일) → Glob×4(wikilink 해소)
> - **응답**: ~500 + ~7,500 + ~400 = ~8,400 chars (≈ 2,100 tokens), 6회+ 호출
> - **단점**: [[wikilink]] 패턴을 수동 파싱해야 하며, 파일 전체 읽기 후 헤딩 구조 수동 분석 필요

> [!note] 비교 분석
> MCP는 3회 호출로 그래프 탐색을 완성. FS는 wikilink 파싱 → 각 링크별 Glob 추가 호출 구조로 **77% 더 많은 토큰**. 특히 `nexus_get_backlinks`는 FS로 구현 불가 — 모든 문서를 스캔해야 역방향 링크를 찾을 수 있기 때문.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: ~200 chars (≈ 50 tokens), 1회 호출
> - **결과**: `architecture/search-system.md` (title: "검색 시스템") ← 즉시 정답

> [!example]- Filesystem 결과
> - **도구**: Grep("Search System") → Read(frontmatter 확인)
> - **응답**: ~600 chars (10개 파일 목록 — 모두 벤치마크 파일) + Read×3 ~600 chars = ~1,200 chars (≈ 300 tokens), 4회 호출
> - **문제**: "Search System"이 벤치마크 파일 본문에 대거 등장해 오탐. `architecture/search-system.md`의 frontmatter `aliases: [Search System]`을 찾으려면 추가 Read 필요

> [!note] 비교 분석
> MCP는 `document_aliases` 인덱스를 대소문자 무관 exact match로 조회 — O(1), 1회, 50 tokens. FS는 "Search System" 텍스트가 본문에 등장하는 벤치마크 파일들을 먼저 반환해 오탐 발생. **83% 토큰 절약** + 정확도 우위.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="architecture/search-system.md", heading="하이브리드 검색"`
> - **응답**: ~280 chars (≈ 70 tokens), 1회 호출
> - **반환 내용**: "하이브리드 검색" 섹션만 정확히 추출 (RRF 공식, hybrid_weight, 쿼리 길이 자동조정)

> [!example]- Filesystem 결과
> - **도구**: Read(`architecture/search-system.md`)
> - **응답**: 7,500 chars (≈ 1,875 tokens), 1회 호출
> - **발견**: 파일 208줄 전체 — 하이브리드 검색 섹션은 그 중 약 8줄

> [!note] 비교 분석
> 동일 1회 호출이지만 MCP는 필요한 섹션만(~70 tokens) vs FS는 파일 전체(~1,875 tokens). **96% 토큰 절약**. 중복 헤딩이 있을 경우 `heading_path`로 정밀 지정 가능 — FS는 불가.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: ~4,000 chars (≈ 1,000 tokens), 1회 호출
> - **결과**: 5개 볼트 통합 랭킹, 상위 10개
>   - `integrations/subagent-mcp-setup.md` (Obsidian Nexus Docs, score: 0.0122)
>   - `integrations/mcp-tools.md` (Obsidian Nexus Docs, score: 0.0120)
>   - `architecture/decisions/011-agent-mcp-bundle-path.md` (score: 0.0116)
>   - `Engineering/Tools/Obsidian Nexus - Claude CLI 기본 도구 설정.md` (**brain** 볼트, score: 0.0084)
>   - + 6개 추가

> [!example]- Filesystem 결과
> - **도구**: Grep("MCP") × 5 볼트 경로
> - **응답**: ~500×5 = ~2,500 chars (≈ 625 tokens), 5회 호출
> - **한계**: 볼트별 파일 목록만 반환, 통합 랭킹 없음. `docs/` 폴더만 검색해도 10개 truncated.

> [!note] 비교 분석
> MCP는 1회 호출로 5개 볼트 통합 랭킹 — brain 볼트의 문서도 발견. FS는 볼트 수만큼 grep 반복 후 수동 병합 필요. 이 시나리오에서 MCP 토큰이 더 많지만 **질적 우위**가 압도적: 통합 랭킹, 스코어, 볼트간 비교.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (tags 파라미터)
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: ~2,400 chars (≈ 600 tokens), 1회 호출
> - **결과**: `context/benchmark/2026-03-19-search-benchmark.md`, `context/benchmark/2026-03-21-search-benchmark.md` 등 mcp+benchmark 태그 문서

> [!example]- Filesystem 결과
> - **도구**: Grep(`tags:.*mcp`) → Grep("검색") in 결과 파일
> - **응답**: ~400 + ~500 = ~900 chars (≈ 225 tokens), 2회 호출
> - **발견**: 5개 파일 (Grep 패턴이 단일행 `tags: [mcp]` 형식만 매칭 — 멀티라인 YAML 블록 미탐지)

> [!note] 비교 분석
> FS grep은 `tags:\n  - mcp` 형식의 멀티라인 frontmatter를 탐지하지 못해 **불완전**. MCP는 인덱싱 시 frontmatter를 파싱해 태그를 정확히 저장하므로 신뢰도 높음. 이 시나리오는 FS 토큰이 적지만 정확도에서 MCP 우위.

---

### 시나리오 8: TOC 조회 + 섹션 정밀 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_toc` → `nexus_get_section` (heading_path 활용)
> - **파라미터**: `path="architecture/search-system.md"` → heading_path 확인 후 섹션 조회
> - **응답**: ~800 + ~300 = ~1,100 chars (≈ 275 tokens), 2회 호출
> - **TOC**: 14개 헤딩 + heading_path 구조 반환 (level 2~4 포함)
>   - `검색 시스템 > 검색 모드 4가지 > 3. 하이브리드 검색 (기본)`
>   - `검색 시스템 > Alias 매칭`, `검색 시스템 > 태그 기반 검색` 등
> - **TOC → 섹션 파이프라인**: heading_path 그대로 get_section에 전달 → 정밀 추출 검증 ✓

> [!example]- Filesystem 결과
> - **도구**: Read(`architecture/search-system.md`) 전체
> - **응답**: ~7,500 chars (≈ 1,875 tokens), 1회 호출
> - **구조 파악**: 208줄 전체를 읽어야 TOC 파악 가능. 중복 헤딩 탐지 불가.

> [!note] 비교 분석
> **85% 토큰 절약**. TOC를 먼저 확인해 heading_path로 정밀 조회하는 2단계 워크플로우가 FS의 전체 파일 읽기보다 효율적. 특히 동일 heading 텍스트가 여러 섹션에 존재할 때 `heading_path`로 정확히 지정 가능 — FS는 불가.

---

## 관련 문서

- [[architecture/search/README|Search Architecture]]
- [[architecture/search-system|검색 시스템]]
- [[integrations/mcp-tools|MCP 도구 레퍼런스]]
- [[guides/configuration|설정 가이드]]
