---
title: "Search Benchmark Report — 2026-03-27 v2"
date: "2026-03-27 15:00"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-27 v2"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-27 15:00
> - **Projects**: Obsidian Nexus Docs, brain, mad-de, xpert-da-web, xpert-na-web (5개 등록)
> - **Scenarios**: 8/8 (S1~S7 + S8 TOC 보너스)
> - **Vault 경로**: `/Users/madup/gorillaProject/obsidian-nexus/docs`
> - **Ollama**: 미확인 (hybrid 검색 실행됨 — fallback 여부 미측정)
> - **MCP 호출 방법**: 네이티브 MCP 도구 (`mcp__nexus__nexus_*`)

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 613 | 200+miss | — | MCP ✓ / FS ✗ |
| 2 | 개념 검색 | 1 | 4 | 600 | 1,050 | 43% | 양쪽 ✓ |
| 3 | 멀티홉 탐색 | 2 | ~9 | 1,250 | ~1,475 | 15% | MCP 완전 / FS 부분 |
| 4 | 별칭 해소 | 1 | 2 | 50 | 325 | 85% | MCP ✓ / FS 부분 |
| 5 | 섹션 단위 조회 | 2 | 1 | 238 | 1,250 | 81% | 양쪽 ✓ |
| 6 | 크로스 프로젝트 | 1 | 5 | 1,200 | 500 | −140% | MCP ✓ / FS ✓ |
| 7 | 태그 필터링 | 1 | 3 | 750 | 875 | 14% | MCP ✓ / FS 부분 |
| 8 | TOC + 섹션 | 2 | 1 | 238 | 1,250 | 81% | 양쪽 ✓ |

> **S6 토큰 역전 해설**: FS Grep은 파일명 목록만 반환(500 tokens)하지만, MCP는 스니펫·메타데이터 포함 전체 결과를 반환(1,200 tokens). 그러나 MCP는 1회 호출로 5개 볼트를 동시 탐색 — FS는 볼트당 1회 × 5 = 5회 호출 필요.

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 11 | 27 | **59% 적음** |
| 추정 토큰 | 4,939 | 5,925+ (S1 miss 포함) | **17% 절약** (정확도 동등 기준) |
| 정확도 우위 | 5/8 완전 일치 | 3/8 완전 일치 | MCP +2 시나리오 우세 |

> [!tip] 실질 절약은 더 크다
> - S1에서 FS는 목표 문서를 완전히 놓쳤음 (top-10 미등장). 실사용에서는 추가 쿼리 반복이 필요해 토큰 비용이 더 늘어남.
> - S3 멀티홉에서 FS는 위키링크 수동 파싱이 필요 — 실질 호출 수는 더 많음.

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회**: 파일 전체 읽기 대비 81% 토큰 절약 (S5, S8)
> - **그래프 탐색**: `nexus_get_cluster` 1회 호출로 2-hop 23개 문서 자동 발견 (S3)
> - **별칭 해소**: "Search System" → `architecture/search-system.md` 즉시 접근, FS는 수동 파싱 필요 (S4)
> - **크로스 볼트 단일 쿼리**: 5개 볼트를 project 파라미터 생략 1회 호출로 탐색 (S6)
> - **태그+쿼리 동시 필터링**: frontmatter 파싱 없이 태그 조건 검색 (S7)
> - **키워드 검색 리랭킹**: 단순 텍스트 매칭보다 backlink/view_count 기반 리랭킹이 관련 문서 우선 (S1)

---

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: 2,450 chars (≈ 613 tokens), 1회 호출
> - **발견 문서**:
>   - rank1~3: `architecture/decisions/010-embedding-context-prefix.md` (3개 청크)
>   - rank4: `guides/configuration.md` ← **목표 문서** (snippet: "설정 가이드 > 설정 파일 위치")
>   - rank5: `guides/getting-started.md`

> [!example]- Filesystem 결과
> - **도구**: Grep("설정") + Grep("가이드") 병렬
> - **응답**: ~800 chars (≈ 200 tokens), 2회 호출
> - **발견 문서**: `guides/configuration.md` top-10 내 **미등장** — benchmark/search-system 관련 문서들이 상위 점령
> - FS 한계: 단순 패턴 매칭은 파일명·스니펫 리랭킹 없이 매칭 빈도만 반영

> [!note] 비교 분석
> MCP는 rank4에서 목표 문서 발견. FS는 top-10에서 완전히 놓쳤음 (추가 Read 없이는 식별 불가).
> **MCP 정확도 우위**. 단, rank4는 다소 아쉬움 — 리랭킹 튜닝 여지 있음.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (mode=hybrid)
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: 2,400 chars (≈ 600 tokens), 1회 호출
> - **발견 문서**:
>   - rank1~3: `architecture/decisions/010-embedding-context-prefix.md` (임베딩 관련 ADR)
>   - rank4: `architecture/search-system.md` ← 핵심 문서
>   - rank5: `devlog/2026-03-26-embedding-prefix-search-quality.md` (스니펫에 "의미 유사도 검색 원리" 정확 매칭)

> [!example]- Filesystem 결과
> - **도구**: Grep("벡터|embedding|유사도") 병렬 3종 → Read 상위 파일
> - **응답**: ~1,200 chars Grep + ~3,000 chars Read = 4,200 chars (≈ 1,050 tokens), 4회 호출
> - **발견 문서**: `architecture/search-system.md` ✓, `010-embedding-context-prefix.md` ✓
> - FS 한계: 3개 패턴 Grep 결과 수동 교차 필요, 정확한 섹션 특정 불가

> [!note] 비교 분석
> 양쪽 모두 핵심 문서 발견. MCP는 1회, FS는 4회 호출.
> **43% 토큰 절약**. MCP는 semantic 매칭으로 관련 devlog도 발견 (FS 놓침).

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과 (nexus_get_cluster 사용)
> - **도구**: `nexus_search` → `nexus_get_cluster(depth=2)` 2회
> - **파라미터**: `query="아키텍처"` → `path="architecture/search/README.md", depth=2`
> - **응답**: ~200 + 4,800 = 5,000 chars (≈ 1,250 tokens), 2회 호출
> - **발견 문서**: **23개** (distance=1: 8개, distance=2: 15개)
>   - dist=1: decisions/008-fts5-aliases, search-alias-improvement-plan, search-system-deep-dive, search-system, benchmark 3종, context/benchmark
>   - dist=2: chunk-scalability, decisions/001~010, devlog 7종, guides/configuration, integrations/mcp-tools

> [!example]- Filesystem 결과 (구버전 방식)
> - **도구**: Grep("아키텍처") → Read(상위 파일) → 위키링크 수동 파싱 → Glob × ~6
> - **응답**: ~400 + 3,000 + 2,500 = 5,900 chars (≈ 1,475 tokens), ~9회 호출
> - **발견 문서**: 1-hop ~8개 (수동 파싱), 2-hop 탐색은 현실적으로 불완전
> - FS 한계: `[[위키링크]]` 파싱 후 각각 Glob → 재귀 시 호출 폭발

> [!note] 비교 분석
> `nexus_get_cluster` 1회로 23개 문서를 distance·tags·snippet 포함하여 완전 탐색.
> FS는 ~9회 호출로도 2-hop 완전 탐색 불가. **구버전 MCP(N+1회)** 대비에서도 `get_cluster`가 압도적.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: 200 chars (≈ 50 tokens), 1회 호출
> - **발견 문서**: `architecture/search-system.md` ✓ (즉시 반환)

> [!example]- Filesystem 결과
> - **도구**: Grep("Search System") --glob "*.md" → Read 매칭 파일 frontmatter
> - **응답**: ~800 + 500 = 1,300 chars (≈ 325 tokens), 2회 호출
> - **발견 문서**: aliases: 필드가 있는 파일들 반환 — 수동으로 "Search System" 값 확인 필요
> - FS 한계: YAML frontmatter 파싱 없이는 alias 값과 파일의 정확한 매핑 불확실

> [!note] 비교 분석
> **85% 토큰 절약**. MCP는 alias 테이블을 직접 조회하여 1회에 정확한 경로 반환.
> FS는 2회 호출 + 수동 확인 필요.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_toc` → `nexus_get_section(heading_path=...)`
> - **파라미터**: `path="architecture/search-system.md"` → `heading_path="검색 시스템 > 검색 모드 4가지 > 3. 하이브리드 검색 (기본)"`
> - **응답**: 700 + 250 = 950 chars (≈ 238 tokens), 2회 호출
> - **발견 내용**: RRF 수식 + hybrid_weight 설명 (정확한 섹션만)
> - TOC 구조: 14개 헤딩 + heading_path 반환 ✓

> [!example]- Filesystem 결과
> - **도구**: Read("docs/architecture/search-system.md") 전체
> - **응답**: ~5,000 chars (≈ 1,250 tokens), 1회 호출
> - **발견 내용**: 파일 전체 (원하는 섹션은 그 중 일부)
> - FS 한계: 중복 헤딩 있을 경우 섹션 특정 불가, 전체 파일 강제 읽기

> [!note] 비교 분석
> **81% 토큰 절약**. `nexus_get_toc`로 heading_path 확인 후 `nexus_get_section`으로 정밀 추출.
> FS는 필요한 섹션이 파일의 10%여도 100% 읽어야 함.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: 4,800 chars (≈ 1,200 tokens), 1회 호출
> - **발견 문서**: 10개 (Obsidian Nexus Docs 9개 + brain 1개)
>   - `integrations/subagent-mcp-setup.md`, `integrations/mcp-tools.md`, `architecture/decisions/011-agent-mcp-bundle-path.md` 등
>   - brain: `Engineering/Tools/Obsidian Nexus - Claude CLI 기본 도구 설정.md`

> [!example]- Filesystem 결과
> - **도구**: Grep("MCP") × 5 볼트 경로 병렬
> - **응답**: ~400 × 5 = 2,000 chars (≈ 500 tokens), 5회 호출
> - **발견 문서**: 파일 경로 목록만 (스니펫·메타데이터 없음), 수동 병합 필요

> [!note] 비교 분석
> MCP는 1회로 모든 볼트 탐색 + 스니펫 포함. FS는 5회 호출 + 수동 병합.
> **토큰은 MCP가 더 많지만** 정보 밀도가 높음(스니펫·태그·backlink 포함).
> 볼트 수가 늘수록 FS 호출 수는 선형 증가, MCP는 고정 1회.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (tags 필터 포함)
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: 3,000 chars (≈ 750 tokens), 1회 호출
> - **발견 문서**: 5개 (`architecture/search/performance/2026-03-27-search-benchmark.md`, `context/benchmark/2026-03-19-search-benchmark.md` 등 mcp 태그 + 검색 관련)

> [!example]- Filesystem 결과
> - **도구**: Grep("tags:") -A5 → 수동 mcp 태그 파일 식별 → Grep("검색") 재실행
> - **응답**: ~1,500 + 700 + 1,300 = 3,500 chars (≈ 875 tokens), 3회 호출
> - **발견 문서**: mcp 태그 식별 후 검색 키워드 재필터링 — 2단계 워크플로우

> [!note] 비교 분석
> **14% 토큰 절약**. MCP는 태그+쿼리 동시 필터링. FS는 2단계(태그 스캔 → 재검색) 필요.
> 태그 수가 많거나 frontmatter 구조가 복잡할수록 FS 비용이 급증.

---

### 시나리오 8: TOC 조회 + 섹션 정밀 조회 (보너스)

> [!example]- MCP 결과
> - **도구**: `nexus_get_toc` → `nexus_get_section`
> - **파라미터**: `path="architecture/search-system.md"` → `heading="3. 하이브리드 검색 (기본)", heading_path="검색 시스템 > 검색 모드 4가지 > 3. 하이브리드 검색 (기본)"`
> - **응답**: 700 + 250 = 950 chars (≈ 238 tokens), 2회 호출
> - **TOC 구조**: 14개 헤딩 (`{heading, level, heading_path}` 형식) ✓
> - **heading_path → get_section 파이프라인**: 정확한 섹션 추출 검증 ✓

> [!example]- Filesystem 결과
> - **도구**: Read 전체 파일
> - **응답**: ~5,000 chars (≈ 1,250 tokens), 1회 호출
> - **한계**: 헤딩 구조 파악에 전체 파일 읽기 강제. 중복 헤딩 시 위치 수동 파악 필요.

> [!note] 비교 분석
> **81% 토큰 절약** (S5와 동일 패턴). TOC → 섹션 2단계 파이프라인이 FS 1회 전체 읽기보다 효율적.
> `heading_path` 구조가 중복 헤딩 disambiguate에 핵심 역할.

---

## 관련 문서

- [[architecture/search/README|Search Architecture]]
- [[architecture/search-system|검색 시스템]]
- [[integrations/mcp-tools|MCP 도구 레퍼런스]]
- [[architecture/search/performance/2026-03-27-search-benchmark|이전 벤치마크 (2026-03-27 v1)]]
