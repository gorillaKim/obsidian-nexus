---
title: "Search Benchmark Report — 2026-03-26 (All 7 Scenarios)"
date: "2026-03-26T00:00:00"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-26 v2"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-26
> - **Projects**: Obsidian Nexus Docs, Obsidian Vault, brain, test-vault, xpert-da-web, xpert-na-web (총 6개)
> - **Scenarios**: 7/7 (all)

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 700 | 625 | -12% | ✅ 동일 |
| 2 | 개념 검색 | 1 | 4 | 650 | 1,450 | 55% | ✅ 동일 |
| 3 | 멀티홉 탐색 | 3 | 5 | 500 | 1,000 | 50% | ✅ MCP 우위 |
| 4 | 별칭 해소 | 1 | 2 | 50 | 150 | 67% | ✅ 동일 |
| 5 | 섹션 단위 조회 | 1 | 1 | 88 | 1,300 | **93%** | ✅ 동일 |
| 6 | 크로스 프로젝트 검색 | 1 | 6 | 1,125 | 1,500 | 25% | ✅ MCP 우위 |
| 7 | 태그 필터링 검색 | 1 | 2 | 625 | 500 | -25% | ✅ 동일 |

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 9 | 22 | **59% 적음** |
| 추정 토큰 | 3,988 | 6,525 | **39% 절약** |
| 정확도 일치율 | 7/7 | 7/7 | 동일 (시나리오 3·6은 MCP 우위) |

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회** (시나리오 5): 93% 토큰 절약 — 파일 전체 읽기 대비 필요 섹션만 추출
> - **그래프 탐색** (시나리오 3): backlinks/links 단일 호출 — FS는 wiki-link 수동 파싱 + Glob×N
> - **별칭 해소** (시나리오 4): 대소문자 무관 alias → 실제 문서 경로 즉시 반환
> - **크로스 볼트 단일 쿼리** (시나리오 6): 6개 볼트를 1회 호출로 통합 랭킹 — FS는 볼트 수만큼 반복
> - **태그 필터링** (시나리오 7): frontmatter 형식(배열/인라인) 무관하게 정확 필터링

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: ~2,800 chars (≈ 700 tokens), 1회 호출
> - **발견 문서**: `guides/configuration.md` (rank 4, heading "설정 가이드")
> - **top-1**: `architecture/decisions/010-embedding-context-prefix.md` (score: 0.0152)

> [!example]- Filesystem 결과
> - **도구**: Grep(`설정`) → Read(`guides/configuration.md` 50줄)
> - **응답**: ~1,500 + ~1,000 = 2,500 chars (≈ 625 tokens), 2회 호출
> - **발견 문서**: `guides/configuration.md` (39개 매칭 파일 중 수동 선택)

> [!note] 비교 분석
> FS가 토큰 12% 적음. 단, FS는 39개 파일 목록에서 수동으로 정답을 골라야 함. MCP는 복합 쿼리("설정 가이드")로 의미 매칭을 시도하나 `guides/configuration.md`가 rank 4에 머문 것은 개선 여지. title="설정 가이드" 문서가 rank 1로 오르지 못한 원인은 낮은 backlink_count(2) + popularity 미부스트.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (mode=hybrid)
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: ~2,600 chars (≈ 650 tokens), 1회 호출
> - **발견 문서**: `architecture/search-system.md` (rank 4), `architecture/decisions/010-embedding-context-prefix.md` (rank 1~3), `devlog/2026-03-26-embedding-prefix-search-quality.md` (rank 5)

> [!example]- Filesystem 결과
> - **도구**: Grep(`벡터|embedding|유사도`) → 31개 파일 → Read 상위 2개
> - **응답**: ~1,800 + 2,000×2 = ~5,800 chars (≈ 1,450 tokens), 4회 호출
> - **발견 문서**: `architecture/search-system.md`, `architecture/search-system-deep-dive.md`

> [!note] 비교 분석
> MCP 55% 절약. FS는 "의미 유사도"라는 단어가 파일에 직접 없으면 탐색 실패 위험. MCP hybrid는 벡터 의미 유사도로 관련 문서를 연결. `architecture/search-system.md`는 alias("의미 유사도 검색", "벡터 검색 원리") 등록 후 rank 상승 추적 중.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="architecture/search/README.md"`
> - **응답**: ~1,400 + ~300 + ~300 = 2,000 chars (≈ 500 tokens), 3회 호출
> - **search top-1**: `architecture/search/README.md` (score: 0.0116)
> - **backlinks**: `context/benchmark/2026-03-26-search-benchmark.md`, `architecture/search/performance/2026-03-26-search-benchmark.md`
> - **links**: `008-fts5-aliases-column-strategy`, `search-system-deep-dive`, `search-system`, `search-alias-improvement-plan`

> [!example]- Filesystem 결과
> - **도구**: Grep(`아키텍처`) → Read(`architecture/search/README.md`) → wiki-link 파싱 → Glob×4
> - **응답**: ~1,500 + ~500 + ~2,000 = 4,000 chars (≈ 1,000 tokens), 5회 호출
> - **발견 문서**: `architecture/search/README.md` + 링크 파일들 (수동 파싱 필요)

> [!note] 비교 분석
> MCP 50% 절약, 호출 40% 적음. FS는 `[[wiki-link]]` 패턴 수동 파싱 + 각 링크마다 Glob 실행 필요. MCP `nexus_get_links`는 단일 호출로 모든 forward link를 `resolved: true/false` 여부와 함께 반환.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: ~200 chars (≈ 50 tokens), 1회 호출
> - **결과**: `architecture/search-system.md` (title: "검색 시스템", status: done)

> [!example]- Filesystem 결과
> - **도구**: Grep(`Search System`) → Read(`architecture/search-system.md` frontmatter)
> - **응답**: ~400 + ~200 = 600 chars (≈ 150 tokens), 2회 호출
> - **결과**: `architecture/search-system.md` (aliases 필드 수동 확인)

> [!note] 비교 분석
> MCP 67% 절약. FS도 빠르게 찾을 수 있으나, alias가 content에 없고 frontmatter에만 있는 경우 패턴이 달라질 수 있음. MCP `nexus_resolve_alias`는 `document_aliases` 테이블 대소문자 무관 조회로 항상 정확.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="architecture/search-system.md", heading="하이브리드 검색"`
> - **응답**: ~350 chars (≈ 88 tokens), 1회 호출
> - **내용**: RRF 공식, hybrid_weight 설명 (7줄 섹션만 추출)

> [!example]- Filesystem 결과
> - **도구**: Read(`architecture/search-system.md` 전체)
> - **응답**: ~5,200 chars (≈ 1,300 tokens), 1회 호출
> - **내용**: 208줄 전체 파일 (필요한 섹션은 약 7줄)

> [!note] 비교 분석
> **MCP 93% 절약** — 7개 시나리오 중 최대 효율 차이. 호출 횟수는 동일(1회)이나 반환 데이터가 14.7배 차이. 대형 문서에서 특정 섹션만 필요할 때 `nexus_get_section`이 결정적으로 유리.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: ~4,500 chars (≈ 1,125 tokens), 1회 호출
> - **발견 문서**: `integrations/subagent-mcp-setup.md` (rank 1, Nexus Docs), `integrations/mcp-tools.md` (rank 2), `Engineering/Tools/Obsidian Nexus - Claude CLI 기본 도구 설정.md` (rank 6, brain 볼트)
> - **프로젝트 수**: 6개 볼트 동시 검색, 글로벌 score 랭킹 제공

> [!example]- Filesystem 결과
> - **도구**: Grep(`MCP`) × 6개 볼트 경로 (순차/병렬)
> - **응답**: ~1,000 chars × 6 = ~6,000 chars (≈ 1,500 tokens), 6회 호출
> - **발견 문서**: 각 볼트별 매칭 파일 목록 (수동 병합, 통합 랭킹 불가)

> [!note] 비교 분석
> MCP 25% 절약, 호출 83% 적음 (1 vs 6). 볼트 수가 늘어날수록 FS 비용은 선형 증가. MCP는 볼트 경계를 넘는 통합 score 랭킹을 제공하지만 FS는 불가.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (tags 파라미터)
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: ~2,500 chars (≈ 625 tokens), 1회 호출
> - **발견 문서**: `context/benchmark/2026-03-19-search-benchmark.md` (rank 1), `context/benchmark/2026-03-21-search-benchmark.md`, `integrations/mcp-tools.md`

> [!example]- Filesystem 결과
> - **도구**: Grep(`- mcp`) → 12개 파일 → Grep(`검색`) path 재필터
> - **응답**: ~800 + ~1,200 = 2,000 chars (≈ 500 tokens), 2회 호출
> - **발견 문서**: `integrations/mcp-tools.md`, `context/benchmark/2026-03-19-search-benchmark.md` 등

> [!note] 비교 분석
> FS가 토큰 20% 적음. 단순 태그 필터에서는 FS가 효율적. 그러나 MCP는 `tags: [mcp]`(인라인), `- mcp`(블록) 등 모든 frontmatter 형식을 DB 파싱으로 정확 필터링. FS의 `- mcp` 패턴은 인라인 형식을 놓칠 수 있음.

---

## 관련 문서

- [[Search Architecture]]
- [[검색 시스템]]
- [[MCP 도구 레퍼런스]]
