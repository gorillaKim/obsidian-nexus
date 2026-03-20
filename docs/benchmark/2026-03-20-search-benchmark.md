---
title: "Search Benchmark Report — 2026-03-20"
date: "2026-03-20"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-20"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-20
> - **Projects**: Obsidian Nexus Docs, test-vault, Obsidian Vault, xpert-da-web (4개)
> - **Scenarios**: 7/7

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 395 | 345 | -14%† | Yes |
| 2 | 개념 검색 | 1 | 5 | 388 | 1,838 | **79%** | Yes |
| 3 | 멀티홉 탐색 | 3 | 5+ | 350 | 588+ | **40%** | Partial‡ |
| 4 | 별칭 해소 | 1 | 2 | 46 | 150 | **69%** | Yes |
| 5 | 섹션 단위 조회 | 1 | 1 | 60 | 1,200 | **95%** | Yes |
| 6 | 크로스 프로젝트 검색 | 1 | 4–8 | 875 | 150–2,150§ | -483%–59% | Partial§ |
| 7 | 태그 필터링 검색 | 2 | 4 | 75 | 550 | **86%** | Partial¶ |

> †키워드 검색은 MCP가 4개 문서 스니펫을 반환해 단순 파일 Read보다 약간 더 소비.
> ‡FS는 backlink를 발견할 수 없음 — forward link(3)만 추출.
> §FS 파일목록만(4호출, 150토큰) vs MCP 랭킹+스니펫(1호출, 875토큰). 동등 품질(스니펫 포함) 시 FS는 8+ 호출, ~2,150토큰.
> ¶MCP `tags` 파라미터 버그로 빈 결과 반환 → `nexus_list_documents(tag=)` 폴백. FS는 3개 mcp-태그 파일을 모두 발견.

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 10 | 23 | **57% 적음** |
| 추정 토큰 (동등 품질 기준) | 2,189 | 6,621 | **67% 절약** |
| 정확도 일치율 | 5/7 | 6/7 | FS 1개 우위 (S7 버그) |

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회 (S5, 95% 절약)**: 파일 전체 읽기 없이 원하는 섹션만 추출
> - **역방향 링크(Backlink) 탐색 (S3)**: FS로는 corpus 전체 grep 없이 불가능
> - **별칭 즉시 해소 (S4, 69% 절약)**: "Search System" → `02-검색-시스템.md` 1회 호출
> - **크로스 볼트 단일 쿼리 (S6)**: 볼트 4개를 1회 호출로 랭킹+스니펫 반환
> - **의미 검색 (S2, 79% 절약)**: 정확한 키워드 없이 개념으로 관련 섹션 탐색

## 발견된 버그 / 개선점

> [!warning] S7 태그 필터 버그
> `nexus_search(tags=["mcp"])` 파라미터가 빈 배열로 처리됨 → 결과 0건.
> `nexus_list_documents(tag="mcp")`는 정상 동작 확인.
> 원인 추정: MCP 도구 JSON 배열 파라미터 파싱 불일치.

---

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: 1,580 chars (≈ 395 tokens), 1회 호출
> - **발견 문서**: `05-설정-가이드.md` (rank 4/4), `04-데이터베이스-스키마.md`, `01-아키텍처.md`, `03-MCP-도구-레퍼런스.md`
> - **스니펫 제공**: Yes (각 결과에 tags, backlink_count, score 포함)

> [!example]- Filesystem 결과
> - **도구**: Grep(`설정` --glob `*.md`) → Read(`05-설정-가이드.md` 50줄)
> - **호출 수**: 2회
> - **응답**: ~280 + ~1,100 = 1,380 chars (≈ 345 tokens)
> - **발견 문서**: `05-설정-가이드.md` (12개 매칭 파일 중 수동 식별)

> [!note] 비교 분석
> 이 시나리오에서는 FS가 토큰 소비가 약간 적음. 단, MCP는 4개 문서의 랭킹+스니펫+메타데이터를 동시 제공 — 탐색 목적이라면 MCP가 더 정보 밀도가 높음. 단순 파일 특정 시에는 FS도 충분.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5`
> - **응답**: 1,550 chars (≈ 388 tokens), 1회 호출
> - **발견 문서**: `02-검색-시스템.md` rank 4 — heading "벡터 검색" 섹션 스니펫 포함
> - **주목**: "의미", "유사도"가 헤딩에 없어도 벡터 임베딩으로 정확히 탐색

> [!example]- Filesystem 결과
> - **도구**: Grep(`벡터`) + Grep(`embedding`) + Grep(`유사도`) → Read 상위 2개 파일
> - **호출 수**: 5회 (Grep 3 + Read 2)
> - **응답**: ~550 (파일목록) + ~4,800 (02-검색-시스템.md) + ~2,000 (01-아키텍처.md) = 7,350 chars (≈ 1,838 tokens)
> - **추가 작업**: 3개 Grep 결과를 수동 중복 제거 필요

> [!note] 비교 분석
> **79% 토큰 절약**. MCP의 벡터 검색 핵심 우위: 정확한 키워드 없이 개념으로 관련 섹션을 찾음. FS는 패턴 3개를 따로 검색하고 전체 파일 2개를 읽어야 동등 정보 획득 가능.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처", limit=3` → `path="01-아키텍처.md"`
> - **응답**: ~900 + ~300 + ~200 = 1,400 chars (≈ 350 tokens), 3회 호출
> - **발견 문서 (forward)**: `00-프로젝트-개요.md`, `02-검색-시스템.md`, `04-데이터베이스-스키마.md`
> - **발견 문서 (backlink)**: `04-데이터베이스-스키마.md`, `06-개발-가이드.md`, `00-프로젝트-개요.md`, `02-검색-시스템.md`
> - **총 관련 문서**: 5개 (중복 제거)

> [!example]- Filesystem 결과
> - **도구**: Grep(`아키텍처`) → Read(`01-아키텍처.md`) → [[...]] 파싱 → Glob × 3
> - **호출 수**: 5회 (forward links만); backlinks 탐색 시 8+ 추가 Grep 필요
> - **응답**: ~200 + ~2,000 + ~150 = 2,350 chars (≈ 588 tokens)
> - **발견 문서 (forward only)**: `00-프로젝트-개요.md`, `02-검색-시스템.md`, `04-데이터베이스-스키마.md`
> - **backlinks**: `[[...]]` 역방향 추적 불가 — corpus 전체 grep 필요 (8+ 추가 호출)

> [!note] 비교 분석
> **backlink는 FS로 대체 불가**. Forward link만 비교 시 FS가 약간 더 소비(40% 차이). 역방향 그래프 탐색은 MCP의 절대적 우위 영역.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `alias="Search System", project="Obsidian Nexus Docs"`
> - **응답**: 185 chars (≈ 46 tokens), 1회 호출
> - **결과**: `02-검색-시스템.md` (title: "검색 시스템") 즉시 반환

> [!example]- Filesystem 결과
> - **도구**: Grep(`Search System` --glob `*.md`) → Read(`02-검색-시스템.md` 20줄)
> - **호출 수**: 2회
> - **응답**: ~200 (3개 매칭 파일) + ~400 (frontmatter 확인) = 600 chars (≈ 150 tokens)
> - **결과**: `02-검색-시스템.md` 발견 (aliases 필드에 "Search System" 확인)

> [!note] 비교 분석
> **69% 절약**. FS도 가능하지만 2단계 (grep → read frontmatter)가 필요. 별칭이 여러 파일에 분산되어 있거나 aliases가 중첩된 경우 FS는 더 복잡해짐.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `path="02-검색-시스템.md", heading="하이브리드 검색", project="Obsidian Nexus Docs"`
> - **응답**: 239 chars (≈ 60 tokens), 1회 호출
> - **반환 내용**: "### 3. 하이브리드 검색" 섹션만 — RRF 공식, hybrid_weight 설명 포함

> [!example]- Filesystem 결과
> - **도구**: Read(`02-검색-시스템.md` 전체)
> - **호출 수**: 1회
> - **응답**: 4,800 chars (≈ 1,200 tokens)
> - **목표 섹션 비율**: 전체의 약 5% (239 chars / 4,800 chars)

> [!note] 비교 분석
> **95% 절약** — 가장 극적인 차이. 동일 1회 호출이지만 MCP는 필요한 섹션만, FS는 파일 전체를 소비. 문서가 길수록 이 격차는 더 벌어짐.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: 3,500 chars (≈ 875 tokens), 1회 호출
> - **인덱싱 프로젝트**: 4개 볼트 동시 검색
> - **발견 문서**: `02-검색-시스템.md`, `01-아키텍처.md`, `00-프로젝트-개요.md`, `projects/obsidian-nexus.md`(test-vault), `05-설정-가이드.md`, `06-개발-가이드.md`, `03-MCP-도구-레퍼런스.md`, `04-데이터베이스-스키마.md` (8건, 2개 프로젝트)
> - **Obsidian Vault**: 결과 없음 (MCP 문서 없음)

> [!example]- Filesystem 결과
> - **도구**: Grep(`MCP`) × 4개 볼트 경로 (병렬)
> - **호출 수**: 4회 (파일목록만); 동등 품질(스니펫) 위해 +4 Read 호출 필요
> - **파일목록 응답**: ~600 chars (≈ 150 tokens) — 스니펫 없음
> - **동등 품질 시**: ~8 호출, ~8,600 chars (≈ 2,150 tokens)
> - **Obsidian Vault**: 결과 없음 (일치)

> [!note] 비교 분석
> 파일 목록만 필요하면 FS(4호출, 150토큰)가 유리. 랭킹+스니펫+메타데이터가 필요하면 MCP(1호출, 875토큰)가 59% 절약. **볼트가 늘수록 FS 호출 수는 선형 증가, MCP는 고정 1회**.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과 (버그 + 폴백)
> - **1차 시도**: `nexus_search(query="검색", tags=["mcp"], limit=5)` → **빈 결과** (버그)
> - **폴백**: `nexus_list_documents(project="Obsidian Nexus Docs", tag="mcp")` → `03-MCP-도구-레퍼런스.md` 반환
> - **응답**: ~300 chars (≈ 75 tokens), 2회 호출
> - **발견 문서**: `03-MCP-도구-레퍼런스.md` 1건 (실제 mcp 태그: 3개 파일)

> [!example]- Filesystem 결과
> - **도구**: Grep(`tags:` -A5) → 수동 필터 → Grep(`검색`) × mcp 태그 파일
> - **호출 수**: 4회 (1 frontmatter scan + 3 targeted grep)
> - **응답**: ~2,000 + ~200 = 2,200 chars (≈ 550 tokens)
> - **발견 문서 (mcp 태그)**: `03-MCP-도구-레퍼런스.md`, `07-MCP-시나리오-테스트.md`, `08-서브에이전트-MCP-설정-가이드.md`
> - **"검색" 포함**: `03-MCP-도구-레퍼런스.md` ✓, `07-MCP-시나리오-테스트.md` ✓

> [!note] 비교 분석
> **이 시나리오는 MCP 버그 발견**. `nexus_search`의 `tags` 배열 파라미터 파싱 불일치로 필터가 동작하지 않음. FS가 오히려 더 완전한 결과(3개 mcp-tagged 파일, 2개 "검색" 매칭) 반환. `nexus_list_documents(tag=)`는 정상 동작 — 버그는 `nexus_search`의 태그 필터 경로에만 해당.

---

## 관련 문서

- [[01-아키텍처]]
- [[02-검색-시스템]]
- [[03-MCP-도구-레퍼런스]]
