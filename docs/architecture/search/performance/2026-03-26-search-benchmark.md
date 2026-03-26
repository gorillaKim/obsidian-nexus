---
title: "Search Benchmark Report — 2026-03-26 (Post-Fix)"
date: "2026-03-26T13:00:00"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-26 Post-Fix"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-26
> - **Build**: Post search quality fix (path boost, popularity tiebreaker, Ollama fallback marker)
> - **Projects**: Obsidian Nexus Docs (주), brain, Obsidian Vault, xpert-da-web, xpert-na-web, test-vault (6개 볼트)
> - **Scenarios**: 7/7
> - **Ollama**: 비활성 (vector search degraded → keyword_only fallback)

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 350 | 550 | 36% | Partial (rank 4) |
| 2 | 개념 검색 | 1 | 2 | 350 | 525 | 33% | FS 우위 (Ollama 비활성) |
| 3 | 멀티홉 탐색 | 3 | 2 | 363 | 1,725 | 79% | MCP 우위 ✅ |
| 4 | 별칭 해소 | 1 | 2 | 38 | 250 | 85% | 동등 |
| 5 | 섹션 단위 조회 | 1 | 1 | 63 | 1,596 | **96%** | 동등 |
| 6 | 크로스 프로젝트 검색 | 1 | 6 | 875 | 450 | FS 49% 절약 | MCP 우위 (완전성) |
| 7 | 태그 필터링 검색 | 1 | 2 | 400 | 163 | FS 59% 절약 | 동등 |

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 9 | 17 | 47% 적음 |
| 추정 토큰 | 2,439 | 5,259 | **54% 절약** |
| 정확도 우위 | 3/7 | 1/7 | MCP 우세 (3 MCP, 1 FS, 3 동등) |

> [!tip] 핵심 인사이트
> - **시나리오 5 (섹션 조회)**: MCP 63 tokens vs FS 1,596 tokens → **96% 절약** 최대
> - **시나리오 3 개선**: path boost 적용 후 `architecture/search/README.md`가 rank 2로 상승 (이전: rank 3 이하)
> - **시나리오 2 (개념 검색)**: Ollama 비활성 시 hybrid=keyword_only — `search_mode: "keyword_only"` 신호로 degraded 상태 표시됨
> - **시나리오 6 (크로스 볼트)**: FS는 6볼트 × 1회 = 6회 필요, MCP는 항상 1회. 볼트 수 증가 시 격차 확대.
> - **인기도 부스트 감소**: max 0.35 → 0.035 (tiebreaker only). mcp-tools.md(backlink=6) 점수 10.6% 감소.

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회**: 6,382자 파일에서 250자 섹션만 추출 — 96% 토큰 절약
> - **그래프 탐색**: `nexus_get_backlinks`/`nexus_get_links`로 위키링크 수동 파싱 불필요
> - **별칭 해소**: `nexus_resolve_alias`로 영문 alias → 한글 문서 1회 호출로 즉시 접근
> - **크로스 볼트 단일 쿼리**: 6볼트 동시 검색 1회 vs FS 6회 (볼트 수에 비례)
> - **태그 구조 인식**: YAML frontmatter 배열 형식 태그를 파싱하여 정확 필터링
> - **Degraded 상태 신호**: `search_mode: "keyword_only"` — Ollama 비활성 시 에이전트가 인식 가능

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: ~1,400 chars (≈ 350 tokens), 1회 호출
> - **발견 문서**: integrations/mcp-tools.md(1위, score:0.0148), guides/installation.md(2위), devlog/dashboard-bug(3위), **guides/configuration.md(4위, score:0.0065)**, guides/getting-started.md(5위)
> - **기대 문서**: `guides/configuration.md` — rank **4**에서 발견 ✅

> [!example]- Filesystem 결과
> - **도구**: `Grep "설정" --glob "*.md"` → `Read 상위 1개 파일 50줄`
> - **응답**: ~200 chars (grep 10 paths) + ~1,800 chars (read) = ~2,200 chars (≈ 550 tokens), 2회 호출
> - **발견 문서**: benchmark/2026-03-26(1위), architecture/search-system(2위), decisions/009(3위)...
> - **기대 문서**: `guides/configuration.md` — **Grep top 10에 미등장** ❌

> [!note] 비교 분석
> MCP는 관련도 랭킹으로 configuration.md를 rank 4에서 발견. FS Grep은 단순 빈도 매칭으로 "설정"이 더 많이 등장하는 벤치마크/아키텍처 문서가 상위를 차지. **MCP 36% 토큰 절약 + 정확도 우위**.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (mode=hybrid → keyword_only fallback)
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: ~1,400 chars (≈ 350 tokens), 1회 호출
> - **발견 문서**: integrations/mcp-tools.md(1위), guides/getting-started.md(2위), guides/installation.md(3위), devlog/dashboard-bug(4위), devlog/mcp-path-mismatch(5위)
> - **⚠️ 기대 문서**: `architecture/search-system.md` — **미등장** (Ollama 비활성, keyword_only 동작)
> - **search_mode**: `"keyword_only"` 신호로 degraded 상태 명시

> [!example]- Filesystem 결과
> - **도구**: `Grep "벡터|embedding|유사도" --glob "*.md"` → Read 상위 파일 50줄
> - **응답**: ~300 chars (5 files) + ~1,800 chars (read) = ~2,100 chars (≈ 525 tokens), 2회 호출
> - **발견 문서**: **architecture/search-system.md(1위)** ✅, decisions/009(2위), devlog/search-quality-fix(3위)

> [!note] 비교 분석
> Ollama 비활성 환경에서 FS가 정확도 우위. MCP hybrid 검색이 keyword_only로 동작하여 "의미 유사도"가 직접 등장하지 않는 search-system.md를 찾지 못함. FS는 `벡터|유사도` 키워드 직접 포함 파일을 즉시 발견. **Ollama 활성화 시 MCP 역전 예상**. search_mode 신호로 에이전트가 degraded 상태를 인지 가능.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="context/benchmark/2026-03-21-search-benchmark.md"`
> - **응답**: ~1,200 + ~50 + ~200 = ~1,450 chars (≈ 363 tokens), 3회 호출
> - **search top-3**: context/benchmark/2026-03-21(rank1, score:0.01162), **architecture/search/README.md(rank2, score:0.01162)**, context/benchmark/2026-03-25(rank3)
> - **⬆️ 개선**: path boost 적용 후 architecture 문서 rank 2 상승 (이전 rank 3 이하)
> - **backlinks**: [] (없음) | **links**: 01-아키텍처, 02-검색-시스템, 03-MCP-도구-레퍼런스

> [!example]- Filesystem 결과
> - **도구**: `Grep "아키텍처" --glob "*.md"` → Read 상위 파일 전체 (위키링크 수동 파싱)
> - **응답**: ~300 chars (5 files) + ~6,382 chars (full read) = ~6,682 chars (≈ 1,725 tokens), 2회 호출
> - **발견 문서**: benchmark/2026-03-26(1위), architecture/search-system(2위), architecture/search/README(3위)
> - **링크 추출**: `[[...]]` 패턴 수동 파싱 필요 — 구조화 불가

> [!note] 비교 분석
> 도구 호출 수는 MCP(3회) > FS(2회)지만 토큰은 MCP 363 vs FS 1,725 → **79% 절약**. path boost 효과로 architecture/search/README.md가 rank 2로 상승. benchmark 문서가 rank 1인 것은 "아키텍처" 키워드를 여러 번 포함하고 있기 때문 — Phase 2 프로젝트 description/alias 라우팅으로 개선 예정.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: ~150 chars (≈ 38 tokens), 1회 호출
> - **발견 문서**: `architecture/search-system.md` (title: 검색 시스템) ✅

> [!example]- Filesystem 결과
> - **도구**: `Grep "Search System" --glob "*.md"` → Read 상위파일 20줄
> - **응답**: ~400 chars (5 files) + ~600 chars (read frontmatter) = ~1,000 chars (≈ 250 tokens), 2회 호출
> - **발견 문서**: `architecture/search-system.md` (frontmatter aliases 확인) ✅

> [!note] 비교 분석
> 두 방법 모두 올바른 문서를 찾음. MCP는 1회 호출에 구조화된 응답. FS는 Grep + Read 2단계 필요. **85% 토큰 절약**.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="architecture/search-system.md", heading="하이브리드 검색"`
> - **응답**: ~252 chars (≈ 63 tokens), 1회 호출
> - **내용**: RRF 수식 + `hybrid_weight` 파라미터 + 쿼리 길이 자동 조정 설명 (섹션만 정확 추출)

> [!example]- Filesystem 결과
> - **도구**: `Read "docs/architecture/search-system.md"` (파일 전체)
> - **응답**: 6,382 chars (≈ 1,596 tokens), 1회 호출
> - **내용**: 185줄 전체 — 원하는 섹션 외 불필요한 내용 포함

> [!note] 비교 분석
> 동일 1회 호출이지만 토큰 차이가 압도적: MCP 63 vs FS 1,596 → **96% 절약**. 파일이 클수록 격차가 더 벌어짐. 특정 섹션만 필요한 경우 `nexus_get_section`이 사실상 필수.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: ~3,500 chars (≈ 875 tokens), 1회 호출
> - **발견 문서**: 6개 프로젝트 동시 검색 — Obsidian Nexus Docs + brain 혼합 10개 결과
> - **커버**: subagent-mcp-setup.md(1위), mcp-tools.md(2위), context/mcp-scenario-test(3위), brain/Engineering/Tools/...(5위) 등
> - **관련도 순위** 포함, 크로스 볼트 통합

> [!example]- Filesystem 결과
> - **도구**: `Grep "MCP" --glob "*.md"` × 6볼트 경로 (각각 1회)
> - **응답**: 볼트별 파일 수 (29+0+6+2+0+0 = 37 files), ~1,800 chars (≈ 450 tokens), **6회 호출**
> - **한계**: 관련도 순위 없음, 수동 병합·중복 제거 필요

> [!note] 비교 분석
> 토큰은 FS가 49% 적지만 MCP는 관련도 순위 + 6볼트 통합 결과를 1회로 반환. 볼트 수 증가 시 FS 호출 횟수는 선형 증가, MCP는 항상 1회. 관련도 랭킹이 필요한 에이전트 사용에서는 MCP가 절대적 우위.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (tags 파라미터)
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: ~1,600 chars (≈ 400 tokens), 1회 호출
> - **발견 문서**: benchmark 문서 5개 (mcp + evaluation 태그, 검색 관련 내용 포함)

> [!example]- Filesystem 결과
> - **도구**: `Grep "- mcp" --glob "*.md"` → 11 files 식별 → `Grep "검색"` in matched files
> - **응답**: ~500 chars + ~150 chars = ~650 chars (≈ 163 tokens), 2회 호출
> - **발견 파일**: 11개 mcp 태그 파일 — YAML 배열 형식 `  - mcp`를 `- mcp` 패턴으로 커버
> - **FS 개선점**: `- mcp` 패턴은 YAML 배열 형식을 캐치하지만 `tags:.*mcp` 인라인 패턴은 놓침

> [!note] 비교 분석
> FS가 토큰 59% 적음. 하지만 MCP는 frontmatter를 파싱하여 저장하므로 `- mcp`, `tags: [mcp]`, `mcp` 등 모든 형식에 무관하게 정확 필터링. FS는 패턴 선택에 따라 일부 형식을 놓칠 수 있음. 이번 테스트에서는 `- mcp` 패턴이 유효하여 동등.

---

## 변경사항 적용 전후 비교

> [!info] 이번 빌드에 적용된 수정사항
> 1. **인기도 부스트 감소**: max 0.35 → 0.035 (tiebreaker only) — mcp-tools.md 점수 10.6% 감소
> 2. **path 기반 부스트**: architecture/guides/overview +0.05, context/devlog -0.03
> 3. **Ollama fallback 개선**: prefix 쿼리 보충 + `search_mode: "keyword_only"` 신호
> 4. **FTS5 특수문자 sanitize**: `build_prefix_fallback_query()` 보안 강화

| 시나리오 | 이전 | 이후 | 변화 |
|---------|------|------|------|
| S1 rank of configuration.md | rank 4 | rank 4 | 변동 없음 (FTS base score 차이 유지) |
| S3 architecture 문서 순위 | rank 3+ | rank 2 | ✅ path boost 효과 |
| S1 mcp-tools.md score | 0.016468 | 0.014772 | ↓10.6% (인기도 감소) |
| S2 degraded 신호 | 없음 | search_mode:"keyword_only" | ✅ 에이전트 인식 가능 |

## 관련 문서

- [[architecture/search-system|검색 시스템]]
- [[architecture/search/README|Search Architecture]]
- [[integrations/mcp-tools|MCP 도구 레퍼런스]]
- [[context/benchmark/2026-03-26-search-benchmark|이전 벤치마크 (Pre-Fix)]]
