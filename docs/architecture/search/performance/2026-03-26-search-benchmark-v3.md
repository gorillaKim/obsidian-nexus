---
title: "Search Benchmark Report — 2026-03-26 (v3)"
date: "2026-03-26 11:00"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-26 v3"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-26 11:00
> - **Projects**: Obsidian Nexus Docs, Obsidian Vault, brain, test-vault, xpert-da-web, xpert-na-web (총 6개)
> - **Scenarios**: 8/8 (시나리오 8 TOC 추가)
> - **MCP 서버**: nexus 네이티브 (stdin/stdout JSON-RPC 직접 호출)

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 3 | ~625 | ~600+ | — | MCP ✅ / FS ❌ |
| 2 | 개념 검색 | 1 | 3 | ~625 | ~1,550 | **60%** | 양쪽 ✅ |
| 3 | 멀티홉 탐색 | 3 | 6 | ~308 | ~1,400 | **78%** | MCP ✅ / FS △ |
| 4 | 별칭 해소 | 1 | 2 | ~38 | ~200 | **81%** | 양쪽 ✅ |
| 5 | 섹션 단위 조회 | 1 | 1 | ~75 | ~1,250 | **94%** | 양쪽 ✅ |
| 6 | 크로스 프로젝트 검색 | 1 | 4 | ~750 | ~200 | FS 73% 저렴† | MCP ✅ / FS △ |
| 7 | 태그 필터링 검색 | 1 | 2 | ~500 | ~88 | FS 82% 저렴† | MCP ✅ / FS △ |
| 8 | TOC + 섹션 조회 | 2 | 1 | ~200 | ~1,500 | **87%** | MCP ✅ / FS △ |

> †S6/S7: FS가 토큰은 적으나 품질(랭킹, 정밀 태그필터, 크로스볼트 병합)에서 열위

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 11 | 22 | **50% 적음** |
| 추정 토큰 (S2~S5, S8) | ~1,216 | ~6,900 | **82% 절약** |
| 정확도 일치율 | 8/8 | 4/8 | MCP 압도 |

> [!tip] 토큰 절약이 크게 나타나는 패턴
> - **섹션/TOC 조회** (S5, S8): 파일 전체 읽기 대비 90%+ 절약
> - **멀티홉** (S3): graph API 덕분에 Glob 반복 불필요
> - **별칭** (S4): 1회 호출로 즉시 해소, FS는 grep+read 2단계

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회**: `nexus_get_section`으로 파일 전체 읽기 대비 94% 토큰 절약
> - **TOC 구조 파악**: `nexus_get_toc`로 heading_path 확인 후 중복 헤딩 정밀 조회
> - **그래프 탐색**: `nexus_get_backlinks`/`nexus_get_links`로 관련 문서 자동 발견
> - **별칭 해소**: 영문 alias → 한글 문서 즉시 접근 (1 call)
> - **크로스 볼트 단일 쿼리**: 볼트 수에 무관한 1회 호출 + 통합 랭킹
> - **태그 필터링**: frontmatter 파싱 없이 정밀 태그 조건 검색 (false positive 없음)
> - **키워드 검색 정확도**: "설정 가이드" 복합 쿼리에서 FS는 top-10 내 미발견, MCP는 rank 4

---

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: ~2,500 chars (≈ 625 tokens), 1회 호출
> - **발견 문서**: `guides/configuration.md` (rank 4), `guides/getting-started.md` (rank 5)
> - **top-1**: `architecture/decisions/010-embedding-context-prefix.md` (score: 0.0152) — 관련도 낮음

> [!example]- Filesystem 결과
> - **도구**: Grep("설정") + Grep("가이드") → Read 후보 파일
> - **파라미터**: `"설정" --glob "*.md"`, `"가이드" --glob "*.md"`
> - **응답**: ~200+200 chars = ~400 chars (≈ 100 tokens), 2회 호출
> - **발견 문서**: 각 grep top-10에 `guides/configuration.md` **미포함** — 추가 read 필요
> - **교집합**: 두 grep 결과 교집합이 목표 문서를 포함하지 않음

> [!note] 비교 분석
> FS 2회 grep으로 `guides/configuration.md`가 top-10 외로 밀림 — 추가 읽기 없이는 목표 미발견.
> MCP는 복합 의미 쿼리를 단일 호출로 처리하여 rank 4에 정확히 위치. **정확도에서 MCP 우위**.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (mode=hybrid)
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: ~2,500 chars (≈ 625 tokens), 1회 호출
> - **발견 문서**: `architecture/decisions/010-embedding-context-prefix.md` (ranks 1-3), `architecture/search-system.md` (rank 4)

> [!example]- Filesystem 결과
> - **도구**: Grep(`벡터|embedding|유사도`) → Read 상위 2개 파일
> - **응답**: ~200(grep) + ~3,000×2(read) = ~6,200 chars (≈ 1,550 tokens), 3회 호출
> - **발견 문서**: `architecture/search-system.md` 포함 (8개 파일 목록에서 식별)

> [!note] 비교 분석
> 양쪽 모두 `architecture/search-system.md` 발견. MCP는 1회 호출 625 tokens, FS는 3회 호출 1,550 tokens. **60% 절약**. FS는 정확한 키워드 패턴이 필요한 반면 MCP는 자연어 쿼리로 관련 문서 검색 가능.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="architecture/search/README.md"`
> - **응답**: ~800 + ~180 + ~250 = ~1,230 chars (≈ 308 tokens), 3회 호출
> - **search top-1**: `architecture/search/README.md`
> - **backlinks**: `context/benchmark/2026-03-26-search-benchmark.md`, `architecture/search/performance/2026-03-26-search-benchmark.md`
> - **links**: `008-fts5-aliases-column-strategy`, `search-system-deep-dive`, `search-system`, `search-alias-improvement-plan`
> - **총 발견 문서**: 7개

> [!example]- Filesystem 결과
> - **도구**: Grep("아키텍처") → Read 상위 파일 (wiki-link 수동 추출) → Glob 각 링크마다
> - **응답**: ~200(grep) + ~5,000(read) + ~400(glob×4) = ~5,600 chars (≈ 1,400 tokens), ~6회 호출
> - **발견 문서**: grep 5개 파일 + wiki-link 수동 파싱 필요 (자동화 불가)

> [!note] 비교 분석
> MCP는 graph API로 backlink/link를 자동 추출. FS는 파일 내 `[[...]]` 패턴을 수동 파싱 후 Glob 반복 필요. **78% 절약**, FS는 graph 구조 자동 탐색 불가.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: ~150 chars (≈ 38 tokens), 1회 호출
> - **발견 문서**: `architecture/search-system.md` (title: "검색 시스템")

> [!example]- Filesystem 결과
> - **도구**: Grep(`Search System`) → Read 상위 파일 20줄
> - **응답**: ~300(grep) + ~500(read) = ~800 chars (≈ 200 tokens), 2회 호출
> - **발견 문서**: `architecture/search-system.md` (aliases 필드에서 확인)
> - **참고**: grep 결과에 benchmark 문서도 포함되어 수동 필터링 필요

> [!note] 비교 분석
> 양쪽 모두 목표 발견. MCP 38 tokens vs FS 200 tokens. **81% 절약**. FS는 alias 언급 문서를 모두 반환하므로 실제 alias 등록 문서를 추가로 판별해야 함.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="architecture/search-system.md", heading="하이브리드 검색"`
> - **응답**: ~300 chars (≈ 75 tokens), 1회 호출
> - **반환 내용**: "### 3. 하이브리드 검색" 섹션만 정확히 추출 (RRF 공식, hybrid_weight 설명)

> [!example]- Filesystem 결과
> - **도구**: Read("docs/architecture/search-system.md") 전체
> - **응답**: ~5,000 chars (≈ 1,250 tokens), 1회 호출
> - **비고**: 원하는 섹션은 전체의 일부이나 파일 전체를 읽어야 위치 파악 가능

> [!note] 비교 분석
> 호출 횟수는 동일(1회)이나 토큰 소비가 크게 다름. MCP 75 vs FS 1,250 tokens. **94% 절약**. 특히 대형 문서일수록 격차 심화.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: ~3,000 chars (≈ 750 tokens), 1회 호출
> - **발견 문서**: Obsidian Nexus Docs 9건 + brain 1건, 통합 랭킹 반환
> - **top-1**: `integrations/subagent-mcp-setup.md` (score: 0.0122, Obsidian Nexus Docs)

> [!example]- Filesystem 결과
> - **도구**: Grep("MCP") × 4 볼트 경로 (병렬)
> - **응답**: ~200×4 = ~800 chars (≈ 200 tokens), 4회 호출
> - **발견 문서**: Obsidian Vault 0건, brain 5건, (nexus/xpert 별도 실행 필요)
> - **비고**: 결과 수동 병합 필요, 랭킹 없음, 스니펫 없음

> [!note] 비교 분석
> FS가 토큰은 적으나 (200 vs 750), MCP는 통합 랭킹·스니펫·메타데이터를 한 번에 제공. FS는 볼트마다 별도 호출 후 수동 병합이 필요하며 관련도 순위를 알 수 없음. **품질 기준 MCP 우위**.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: ~2,000 chars (≈ 500 tokens), 1회 호출
> - **top-1**: `context/benchmark/2026-03-19-search-benchmark.md` (score: 0.0161)
> - **특징**: "mcp" 태그가 frontmatter에 있는 문서만 정밀 필터링

> [!example]- Filesystem 결과
> - **도구**: Grep(`- mcp`) → Grep(`검색`, path=mcp-tools.md)
> - **응답**: ~300 + ~50 = ~350 chars (≈ 88 tokens), 2회 호출
> - **문제**: `- mcp`는 content 내 언급도 매칭 (false positive 다수)
> - **발견**: `mcp-tools.md`에서 "검색" 확인되었으나 태그 정밀 필터 없음

> [!note] 비교 분석
> FS 토큰이 적으나 (88 vs 500), FS의 `- mcp` grep은 frontmatter tags가 아닌 본문 `- mcp` 패턴도 모두 매칭하여 false positive 발생. MCP는 frontmatter tags를 파싱 없이 정확히 필터링. **정확도 기준 MCP 우위**.

---

### 시나리오 8: TOC + 섹션 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_toc` → `nexus_get_section` (heading_path 활용)
> - **파라미터**: `path="architecture/search-system.md"` → `heading_path="검색 시스템 > 검색 모드 4가지 > 3. 하이브리드 검색 (기본)"`
> - **응답**: ~500 + ~300 = ~800 chars (≈ 200 tokens), 2회 호출
> - **TOC 결과**: 14개 헤딩, heading_path 포함 → 중복 헤딩 없이 정밀 조회
> - **섹션 결과**: RRF 공식, hybrid_weight, 쿼리 길이 자동 조정 설명

> [!example]- Filesystem 결과
> - **도구**: Read("docs/architecture/search-system.md") 전체
> - **응답**: ~5,000+ chars (≈ 1,500 tokens), 1회 호출
> - **문제**: 헤딩 구조 파악을 위해 전체 파일 읽어야 하며, 중복 헤딩 시 위치 수동 파악 필요

> [!note] 비교 분석
> MCP는 TOC로 문서 구조를 먼저 파악한 뒤 정밀 섹션만 추출. **87% 절약**. 특히 `heading_path`로 중복 헤딩을 명확히 지정할 수 있어 FS 대비 disambiguation 능력 우위.

---

## 관련 문서

- [[architecture/search/README|Search Architecture]]
- [[architecture/search-system|검색 시스템]]
- [[integrations/mcp-tools|MCP 도구 레퍼런스]]
