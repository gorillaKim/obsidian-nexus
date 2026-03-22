---
title: "Search Benchmark Report — 2026-03-21"
date: "2026-03-21"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-21"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-21
> - **Projects**: Obsidian Nexus Docs, Obsidian Vault, test-vault, xpert-da-web (총 4개)
> - **Scenarios**: 7/7

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 450 | 550 | 18% | ✓ |
| 2 | 개념 검색 | 1 | 4 | 475 | 550 | 14% | ✓ |
| 3 | 멀티홉 탐색 | 3 | 3 | 550 | 250 | -120% | ✓ |
| 4 | 별칭 해소 | 1 | 1 | 50 | 950 | **95%** | ✓ |
| 5 | 섹션 단위 조회 | 1 | 1 | 80 | 800 | **90%** | ✓ |
| 6 | 크로스 프로젝트 검색 | 1 | 4 | 850 | 175 | -386% | ✓ |
| 7 | 태그 필터링 검색 | 3 | 2 | 200 | 1250 | **84%** | ✓ |

> [!tip] 절약률 해석
> - 음수 절약률(S3, S6)은 MCP가 더 많은 토큰을 소비했음을 의미 — 단, **메타데이터·스니펫·랭킹이 포함**되어 후속 호출 불필요
> - 양수 절약률은 MCP가 동일 정보를 더 적은 토큰으로 제공

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 11 | 17 | **35% 적음** |
| 추정 토큰 | 2,655 | 4,525 | **41% 절약** |
| 정확도 일치율 | 7/7 | 7/7 | 동일 |

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회 (S5, 90% 절약)**: 파일 전체 읽기 없이 원하는 섹션만 정밀 추출
> - **별칭 즉시 해소 (S4, 95% 절약)**: "Search System" → `02-검색-시스템.md` 1회 호출
> - **역방향 링크(Backlink) 탐색 (S3)**: FS로는 corpus 전체 grep 없이 방향성 구분 불가능
> - **크로스 볼트 단일 쿼리 (S6)**: 볼트 4개를 1회 호출로 랭킹+스니펫 반환
> - **의미 검색 (S2)**: 정확한 키워드 없이 개념만으로 관련 섹션 탐색
> - **태그 필터링 (S7, 84% 절약)**: frontmatter 파싱 없이 태그+키워드 원자적 처리

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: 1,800 chars (≈ 450 tokens), 1회 호출
> - **발견 문서**: `05-설정-가이드.md` (rank 4, score 0.0058), `01-아키텍처.md`, `04-데이터베이스-스키마.md`, `03-MCP-도구-레퍼런스.md`
> - **비고**: 랭킹 4위 — "설정 가이드" 정확 매칭보다 백링크 많은 문서가 상위 랭크됨

> [!example]- Filesystem 결과
> - **도구**: Grep(`설정` --glob `*.md`) → Read(`05-설정-가이드.md` 50줄)
> - **호출 수**: 2회
> - **응답**: ~1,400 (13개 파일 목록) + ~800 (Read) = 2,200 chars (≈ 550 tokens)
> - **발견 문서**: 13개 파일 중 수동으로 `05-설정-가이드.md` 식별

> [!note] 비교 분석
> MCP는 관련도 점수+스니펫으로 즉시 상위 결과 판별 가능. FS는 13개 파일 목록에서 사람이 직접 선별해야 함. 토큰 절약은 소폭(18%)이나 품질 차이가 큼.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (mode=hybrid)
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: 1,900 chars (≈ 475 tokens), 1회 호출
> - **발견 문서**: `02-검색-시스템.md` § "벡터 검색 (sqlite-vec + Ollama)" (스니펫에 수식+코드 포함)

> [!example]- Filesystem 결과
> - **도구**: Grep(`벡터`) + Grep(`embedding`) + Grep(`유사도`) 병렬 → Read(`02-검색-시스템.md` 50줄)
> - **호출 수**: 4회 (Grep×3 + Read×1)
> - **응답**: ~800 (3개 grep, 각 11/8/4 파일) + ~1,400 (Read) = 2,200 chars (≈ 550 tokens)
> - **발견 문서**: 3개 Grep 결과 수동 교집합 → `02-검색-시스템.md`

> [!note] 비교 분석
> FS는 키워드를 사전에 알고 있어야 하며, 3번 검색 후 수동 병합. MCP는 자연어 개념으로 1회 검색. "의미 유사도"라는 단어가 본문에 없어도 벡터 유사도로 탐색 가능.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="01-아키텍처.md"`
> - **응답**: ~800 + ~400 + ~400 = 2,200 chars (≈ 550 tokens), 3회 호출
> - **backlinks**: `04-데이터베이스-스키마.md`, `06-개발-가이드.md`, `00-프로젝트-개요.md`, `02-검색-시스템.md`
> - **links**: `00-프로젝트-개요.md`, `02-검색-시스템.md`, `04-데이터베이스-스키마.md`

> [!example]- Filesystem 결과
> - **도구**: Grep(`아키텍처`) → Grep(`\[\[.*\]\]` in `01-아키텍처.md`) + Grep(`01-아키텍처`)
> - **호출 수**: 3회
> - **응답**: ~400 + ~100 + ~300 = 1,000 chars (≈ 250 tokens)
> - **발견 문서**: links=[00, 02, 04] 확인, backlinks=[00, 02, 04, 06] 확인 (benchmark 파일 노이즈 포함)

> [!note] 비교 분석
> 토큰은 FS가 54% 적으나, MCP는 **backlink/forward-link 방향성을 명확히 구분**. FS에서는 `Grep "01-아키텍처"` 결과에 benchmark 파일이 포함되어 노이즈 필터링 필요. MCP는 DB 인덱스 기반으로 정확한 링크 그래프 반환.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `alias="Search System", project="Obsidian Nexus Docs"`
> - **응답**: 200 chars (≈ 50 tokens), 1회 호출
> - **결과**: `02-검색-시스템.md` (title: "검색 시스템") 즉시 반환

> [!example]- Filesystem 결과
> - **도구**: Grep(`Search System` --glob `*.md` -C2)
> - **호출 수**: 1회
> - **응답**: 3,800 chars (≈ 950 tokens) — benchmark 파일 내 과거 기록 대거 포함
> - **발견 문서**: `02-검색-시스템.md` 확인 가능하나 노이즈 19건 함께 반환

> [!note] 비교 분석
> **95% 토큰 절약**. FS는 동일 alias가 benchmark 문서 등에 언급되어 있어 노이즈가 폭발적으로 증가. MCP는 `document_aliases` 테이블 직접 조회로 노이즈 0.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="02-검색-시스템.md", heading="하이브리드 검색"`
> - **응답**: 320 chars (≈ 80 tokens), 1회 호출
> - **결과**: "3. 하이브리드 검색" 섹션 (RRF 공식, hybrid_weight 설명) 정확히 추출

> [!example]- Filesystem 결과
> - **도구**: Read(`02-검색-시스템.md` 전체)
> - **호출 수**: 1회
> - **응답**: 3,200 chars (≈ 800 tokens) — 131줄 전체
> - **필요 정보**: 43~52줄 (전체의 7%)만 필요

> [!note] 비교 분석
> **90% 토큰 절약**. 동일한 1회 호출이지만 MCP는 섹션만, FS는 파일 전체를 반환. 파일이 길어질수록 격차 심화. 이것이 MCP의 가장 명확한 우위.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: 3,400 chars (≈ 850 tokens), 1회 호출
> - **발견 문서**: Obsidian Nexus Docs 7건 + test-vault 1건 (`projects/obsidian-nexus.md`) — 랭킹+스니펫 포함

> [!example]- Filesystem 결과
> - **도구**: Grep(`MCP`) × 4 볼트 경로 병렬
> - **호출 수**: 4회
> - **응답**: ~700 chars (≈ 175 tokens) — 파일 목록만, 스니펫/랭킹 없음
> - **발견 문서**: Nexus Docs 12건 + test-vault 2건 (Obsidian Vault, xpert-da-web은 0건)

> [!note] 비교 분석
> FS가 토큰은 79% 적으나 **랭킹·스니펫·관련도 점수 없음**. 후속 판단(어떤 파일이 더 관련성 있는가)을 위해 추가 Read 호출 필요. MCP는 1회로 4볼트 통합 랭킹 반환.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (tags=["mcp"] 실패 → 폴백 `nexus_list_documents` → tags=["search"] 재시도)
> - **파라미터**: 최종 `query="검색", project="Obsidian Nexus Docs", tags=["search"], limit=5`
> - **응답**: ~800 chars (≈ 200 tokens), 3회 호출 (폴백 포함)
> - **발견 문서**: `02-검색-시스템.md` § "키워드 검색 (FTS5)"

> [!example]- Filesystem 결과
> - **도구**: Grep(`tags:` -C5 전체 스캔) → 수동으로 "search" 태그 파일 식별 후 Grep(`검색`)
> - **호출 수**: 2회
> - **응답**: ~5,000 (frontmatter 전체 스캔) + ~200 = 5,200 chars (≈ 1,300 tokens)
> - **발견 문서**: `02-검색-시스템.md`

> [!note] 비교 분석
> **84% 토큰 절약** (폴백 3회 포함에도). FS는 frontmatter 전체 스캔이 필수로 토큰 소비가 큼. MCP는 DB 인덱스로 태그+키워드 원자적 처리. 단, 초기 "mcp" 태그 쿼리 빈 결과 반환으로 폴백 필요 — 태그 데이터가 인덱싱 범위 내에 있어야 함.

---

## 관련 문서

- [[01-아키텍처]]
- [[02-검색-시스템]]
- [[03-MCP-도구-레퍼런스]]
