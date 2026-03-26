---
title: "Search Benchmark Report — 2026-03-26"
date: "2026-03-26T10:00:00"
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
> - **Projects**: Obsidian Nexus Docs (주), brain, Obsidian Vault, xpert-da-web, xpert-na-web, test-vault (크로스 볼트)
> - **Scenarios**: 7/7

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 170 | 450 | 62% | Partial |
| 2 | 개념 검색 | 1 | 2 | 300 | 875 | 66% | FS 우위 |
| 3 | 멀티홉 탐색 | 3 | 2 | 265 | 1,670 | 84% | MCP 우위 |
| 4 | 별칭 해소 | 1 | 2 | 38 | 175 | 78% | 동등 |
| 5 | 섹션 단위 조회 | 1 | 1 | 63 | 1,596 | **96%** | 동등 |
| 6 | 크로스 프로젝트 검색 | 1 | 6 | 875 | 600 | FS 31% 절약 | MCP 우위(완전성) |
| 7 | 태그 필터링 검색 | 1 | 3 | 400 | 100 | FS 75% 절약 | MCP 우위(정확도) |

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 9 | 18 | 50% 적음 |
| 추정 토큰 | 2,111 | 5,466 | **61% 절약** |
| 정확도 우위 | 4/7 | 2/7 | MCP 우세 |

> [!tip] 핵심 인사이트
> - **시나리오 5 (섹션 조회)**: MCP 63 tokens vs FS 1,596 tokens → **96% 절약**이 가장 극적
> - **시나리오 2 (개념 검색)**: 벡터 검색 비활성 환경에서 MCP가 FS보다 관련 문서를 못 찾음 — Ollama 의존성의 약점
> - **시나리오 7 (태그 필터링)**: FS Grep이 YAML 배열 형식(`- mcp`)을 `tags:.*mcp` 패턴으로 놓침 → MCP 정확도 우위
> - **시나리오 6 (크로스 볼트)**: FS는 볼트마다 Grep 1회씩 6회 필요 vs MCP 1회. 볼트가 늘수록 격차 확대.

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회**: 6,382자 파일에서 250자 섹션만 추출 — 96% 토큰 절약
> - **그래프 탐색**: `nexus_get_backlinks`/`nexus_get_links`로 위키링크 수동 파싱 불필요
> - **별칭 해소**: `nexus_resolve_alias`로 영문 alias → 한글 문서 1회 호출로 즉시 접근
> - **크로스 볼트 단일 쿼리**: 볼트 수와 무관하게 항상 1회 호출
> - **태그 구조 인식**: YAML frontmatter 배열 형식 태그를 Grep 패턴 실수 없이 정확 필터링

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: ~680 chars (≈ 170 tokens), 1회 호출
> - **발견 문서**: mcp-tools.md(1위), installation.md(2위), dashboard-bug(3위), **guides/configuration.md**(4위), getting-started.md(5위)
> - **기대 문서**: `guides/configuration.md` — rank 4에서 발견

> [!example]- Filesystem 결과
> - **도구**: `Grep "설정" --glob "*.md"` → `Read top 50줄`
> - **응답**: ~300 chars (grep) + ~1,500 chars (read) = ~1,800 chars (≈ 450 tokens), 2회 호출
> - **발견 문서**: architecture/search-system.md(1위), decisions/009(2위), devlog...(3위~)
> - **기대 문서**: `guides/configuration.md` — Grep top 10에 미등장

> [!note] 비교 분석
> MCP는 관련도 랭킹으로 configuration.md를 rank 4에서 발견. FS Grep은 단순 패턴 매칭이라 "설정"이 더 많이 등장하는 다른 파일들이 상위를 차지하여 기대 문서를 놓침. 토큰 62% 절약 + 정확도 Partial.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (mode=hybrid)
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: ~1,200 chars (≈ 300 tokens), 1회 호출
> - **발견 문서**: mcp-tools.md(1위), getting-started.md(2위), installation.md(3위), dashboard-bug(4위), mcp-path-mismatch(5위)
> - **⚠️ 기대 문서**: `architecture/search-system.md` — **미등장** (벡터 검색 비활성 추정)

> [!example]- Filesystem 결과
> - **도구**: `Grep "벡터|embedding|유사도" --glob "*.md"` → Read 상위 파일
> - **응답**: ~500 chars (grep, 28 files) + ~2,000 chars (read 일부) = ~2,500 chars (≈ 875 tokens), 2회 호출
> - **발견 문서**: **architecture/search-system.md(1위)** ✅, decisions/009(2위), devlog/search-quality-fix(3위)...

> [!note] 비교 분석
> 이 시나리오에서 Filesystem이 정확도 우위. MCP hybrid 검색이 Ollama 벡터 없이 키워드 FTS만 동작한 탓에 "의미 유사도"가 직접 등장하지 않는 파일을 찾지 못함. FS Grep은 "벡터|유사도" 키워드를 직접 포함한 search-system.md를 즉시 발견. **Ollama 비활성 시 hybrid 검색의 한계 확인**.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="context/benchmark/2026-03-21-search-benchmark.md"`
> - **응답**: ~800(search) + ~10(backlinks) + ~250(links) = ~1,060 chars (≈ 265 tokens), 3회 호출
> - **backlinks**: [] (없음)
> - **links**: 01-아키텍처, 02-검색-시스템, 03-MCP-도구-레퍼런스

> [!example]- Filesystem 결과
> - **도구**: `Grep "아키텍처" --glob "*.md"` → Read 상위 파일 전체 (위키링크 수동 파싱)
> - **응답**: ~300 chars (grep) + ~6,382 chars (read 전체) = ~6,682 chars (≈ 1,670 tokens), 2회 호출
> - **발견 문서**: search-system.md(1위), benchmark-26(2위), search/README(3위)...
> - **링크 추출**: `[[...]]` 패턴 수동 파싱 필요 — 구조화 불가

> [!note] 비교 분석
> 도구 호출 수는 MCP(3회) > FS(2회)이지만 토큰은 MCP 265 vs FS 1,670으로 **84% 절약**. FS는 위키링크 추출을 위해 파일 전체를 읽어야 하고 `[[링크]]` 패턴을 수동 파싱해야 함. MCP는 구조화된 링크 데이터를 즉시 반환.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: ~150 chars (≈ 38 tokens), 1회 호출
> - **발견 문서**: `architecture/search-system.md` (title: 검색 시스템) ✅

> [!example]- Filesystem 결과
> - **도구**: `Grep "Search System" --glob "*.md"` → Read 상위파일 20줄
> - **응답**: ~300 chars (grep) + ~400 chars (read frontmatter) = ~700 chars (≈ 175 tokens), 2회 호출
> - **발견 문서**: `architecture/search-system.md` (frontmatter의 `aliases: - Search System` 확인) ✅

> [!note] 비교 분석
> 두 방법 모두 올바른 문서를 찾음. MCP는 1회 호출에 구조화된 응답(id, path, title, status). FS는 Grep + Read 2단계 필요. **78% 토큰 절약**. alias 수가 많거나 여러 볼트에 분산된 경우 MCP 격차가 더 커짐.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="architecture/search-system.md", heading="하이브리드 검색"`
> - **응답**: ~250 chars (≈ 63 tokens), 1회 호출
> - **내용**: RRF 수식 + `hybrid_weight` 파라미터 + 쿼리 길이 자동 조정 설명 (섹션만 정확 추출)

> [!example]- Filesystem 결과
> - **도구**: `Read "docs/architecture/search-system.md"` (파일 전체)
> - **응답**: 6,382 chars (≈ 1,596 tokens), 1회 호출
> - **내용**: 185줄 전체 — 원하는 섹션 외 불필요한 내용 포함

> [!note] 비교 분석
> 동일 1회 호출이지만 토큰 차이가 압도적: MCP 63 vs FS 1,596 → **96% 절약**. 파일이 클수록 이 격차는 더 벌어짐. 특정 섹션만 필요한 경우 `nexus_get_section`이 사실상 필수.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: ~3,500 chars (≈ 875 tokens), 1회 호출
> - **발견 문서**: 6개 프로젝트 동시 검색 — Obsidian Nexus Docs + brain 볼트 혼합 10개 결과
> - **커버**: subagent-mcp-setup.md, mcp-tools.md, mcp-scenario-test.md, brain/Tools/... 등

> [!example]- Filesystem 결과
> - **도구**: `Grep "MCP"` × 6볼트 경로 (각각 1회)
> - **응답**: 볼트당 ~400 chars × 6 = ~2,400 chars (≈ 600 tokens), **6회 호출**
> - **한계**: 관련도 순위 없음, 수동 병합·중복 제거 필요

> [!note] 비교 분석
> 토큰은 FS가 31% 적지만 MCP는 관련도 순위 + 6볼트 통합 결과를 1회로 반환. 볼트가 늘수록 FS 호출 횟수는 선형 증가, MCP는 항상 1회. 관련도 랭킹이 필요한 에이전트 사용에서는 MCP가 절대적 우위.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (tags 파라미터)
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: ~1,600 chars (≈ 400 tokens), 1회 호출
> - **발견 문서**: benchmark 문서 5개 (mcp + evaluation 태그, 검색 관련 내용 포함)

> [!example]- Filesystem 결과
> - **도구**: `Grep "tags:.*mcp"` → 2 files → `Grep "검색"` in 2 files
> - **응답**: ~400 chars (≈ 100 tokens), 3회 호출
> - **⚠️ 발견 파일**: 2개만 검출 — YAML 배열 형식 `  - mcp`를 `tags:.*mcp` 패턴이 놓침

> [!note] 비교 분석
> FS가 토큰은 75% 적지만 YAML 배열 형식의 태그를 패턴 매칭으로 완전히 커버하지 못함. MCP는 frontmatter를 파싱하여 저장하므로 태그 구조에 무관하게 정확 필터링. **토큰 비용 vs 정확도 트레이드오프**.

---

## 관련 문서

- [[architecture/search/README|Search Architecture]]
- [[architecture/search-system|검색 시스템]]
- [[integrations/mcp-tools|MCP 도구 레퍼런스]]
