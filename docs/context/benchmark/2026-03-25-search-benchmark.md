---
title: "Search Benchmark Report — 2026-03-25"
date: "2026-03-25"
tags:
  - benchmark
  - mcp
  - evaluation
aliases:
  - "Search Benchmark 2026-03-25"
---

# Search Benchmark Report

> [!info] 벤치마크 메타데이터
> - **Date**: 2026-03-25
> - **Projects**: Obsidian Nexus Docs (primary), brain, Obsidian Vault, xpert-da-web, xpert-na-web, test-vault (6개 인덱싱)
> - **Scenarios**: 7/7
> - **Vault Root**: `/Users/madup/gorillaProject/obsidian-nexus/docs`

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 450 | 700 | 36% | 부분 일치 |
| 2 | 개념 검색 | 1 | 5 | 450 | 2,025 | 78% | 부분 일치 |
| 3 | 멀티홉 탐색 | 3 | 5+ | 365 | 775+ | 53% | 부분 일치 |
| 4 | 별칭 해소 | 1 | 2 | 40 | 225 | 82% | 일치 |
| 5 | 섹션 단위 조회 | 1 | 1 | 70 | 975 | 93% | 일치 |
| 6 | 크로스 프로젝트 검색 | 1 | 6 | 875 | 750* | -17%* | 부분 일치 |
| 7 | 태그 필터링 검색 | 1 | 2 | 3 | 38 | 92%** | 불일치** |

\* S6 FS는 랭킹·스니펫 없는 파일 목록만 반환 — 실질 정보량은 MCP가 우세
\*\* S7 MCP는 결과 0건 (tags 파라미터 파싱 이슈 의심) — FS도 YAML 배열 포맷 불일치로 1건만 발견

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 9회 | 23회 | **61% 적음** |
| 추정 토큰 | ~2,253 | ~5,288 | **~57% 절약** |
| 정확도 일치율 | 2/7 완전, 4/7 부분 | 2/7 완전, 3/7 부분 | MCP +1 부분 |

## MCP 고유 가치 (Filesystem 대체 불가)

> [!tip] MCP만의 강점
> - **섹션 단위 조회** (S5): 파일 전체(975토큰) 대비 93% 절약 — 동일 정보를 70토큰으로
> - **그래프 탐색** (S3): `nexus_get_links`로 위키링크를 파싱 없이 즉시 구조화 반환
> - **별칭 해소** (S4): `nexus_resolve_alias` 1회 호출로 "Search System" → `architecture/search-system.md` 직접 매핑
> - **크로스 볼트 단일 쿼리** (S6): 6개 볼트를 1회 호출로 랭킹 포함 탐색 (FS는 볼트 수만큼 grep 반복)
> - **랭킹 + 스니펫**: FS grep은 파일 목록만, MCP는 relevance score + 문맥 스니펫 제공

## 시나리오 상세

### 시나리오 1: 키워드 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="설정 가이드", project="Obsidian Nexus Docs", limit=5`
> - **응답**: ~1,800 chars (≈ 450 tokens), 1회 호출
> - **발견 문서**: integrations/mcp-tools.md (1위), architecture/decisions/001-view-cooldown-atomic-sql.md (2위), architecture/decisions/002-attention-docs-thresholds-constants.md (3위), architecture/database-schema.md (4위), **guides/installation.md** (5위)
> - **최고 score**: 0.0131 (매우 낮음)
> - **실제 "설정 가이드" 문서** guides/configuration.md는 top-5 밖

> [!example]- Filesystem 결과
> - **도구**: `Grep "설정" **/*.md` → `Read guides/installation.md (50줄)`
> - **응답**: ~800 (grep 27파일 목록) + ~2,000 (Read) = ~2,800 chars (≈ 700 tokens), 2회 호출
> - **발견 문서**: 27개 파일 (무순위) — 관련 없는 파일 다수 포함, 랭킹 없음

> [!note] 비교 분석
> 두 방법 모두 "설정 가이드"의 최적 문서(guides/configuration.md)를 top에 올리지 못함.
> MCP는 낮은 score(0.011~0.013)로 약한 신호 반영. FS는 27개 파일 노이즈.
> **공통 원인**: "설정"은 매우 빈번한 단어라 FTS5 IDF가 낮음 + vector 비활성 가능성.

---

### 시나리오 2: 개념 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (hybrid)
> - **파라미터**: `query="의미 유사도 검색 원리", mode="hybrid", limit=5`
> - **응답**: ~1,800 chars (≈ 450 tokens), 1회 호출
> - **발견 문서**: integrations/mcp-tools.md (1위), architecture/database-schema.md (2위), guides/getting-started.md (3위), devlog/2026-03-23-dashboard-ranking-empty-bug.md (4위), guides/installation.md (5위)
> - **핵심 문서 부재**: `architecture/search-system.md` (벡터 검색 상세 설명) top-5 미등장

> [!example]- Filesystem 결과
> - **도구**: `Grep "벡터"`, `Grep "embedding"`, `Grep "유사도"` → `Read` 상위 2파일
> - **응답**: ~2,100 (grep 3회) + ~6,000 (Read 2파일) = ~8,100 chars (≈ 2,025 tokens), 5회 호출
> - **발견 문서**: architecture/search-system.md ✓ (벡터 grep에서 발견, 수동 교차 분석 필요)

> [!note] 비교 분석
> FS가 핵심 문서(search-system.md)를 찾지만 수동 결과 병합 필요 + 2,025 토큰 소비.
> MCP는 450토큰이지만 관련 문서를 top-5에 올리지 못한 **정확도 문제** 발생.
> **원인 추정**: Ollama 벡터 비활성 상태에서 FTS5만 동작 → "의미 유사도"라는 한국어 구문이 영문 "embedding/vector" 본문과 미매칭.

---

### 시나리오 3: 멀티홉 탐색

> [!example]- MCP 결과
> - **도구**: `nexus_search` → `nexus_get_backlinks` + `nexus_get_links` (병렬)
> - **파라미터**: `query="아키텍처"` → `path="context/benchmark/2026-03-21-search-benchmark.md"`
> - **응답**: ~1,200 + ~10 + ~250 = ~1,460 chars (≈ 365 tokens), 3회 호출
> - **search top-1**: context/benchmark/2026-03-21-search-benchmark.md (아키텍처 관련 벤치마크 문서)
> - **backlinks**: [] (없음)
> - **links**: 01-아키텍처, 02-검색-시스템, 03-MCP-도구-레퍼런스

> [!example]- Filesystem 결과
> - **도구**: `Grep "아키텍처" **/*.md` → `Read architecture/architecture.md (80줄)` → 위키링크 파싱 → Glob × 3
> - **응답**: ~700 + ~2,500 + 추가 Glob = ~3,200+ chars (≈ 800+ tokens), 5+회 호출
> - **발견 위키링크**: [[00-프로젝트-개요]], [[02-검색-시스템]], [[04-데이터베이스-스키마]]

> [!note] 비교 분석
> MCP top-1이 benchmark 문서(예상과 다름)지만, links 탐색으로 핵심 아키텍처 문서 3개 발견.
> FS는 architecture.md를 직접 읽어 위키링크 추출 가능하나, 추가 Glob이 필요하고 토큰 2배 이상.
> **MCP 고유**: 링크 그래프가 구조화된 형태로 반환 — 정규식 파싱 불필요.

---

### 시나리오 4: 별칭 해소

> [!example]- MCP 결과
> - **도구**: `nexus_resolve_alias`
> - **파라미터**: `project="Obsidian Nexus Docs", alias="Search System"`
> - **응답**: ~160 chars (≈ 40 tokens), 1회 호출
> - **결과**: `architecture/search-system.md` (title: "검색 시스템") — 즉시 정확 매핑 ✓

> [!example]- Filesystem 결과
> - **도구**: `Grep "Search System" **/*.md` → `Read 해당 파일 20줄`
> - **응답**: ~300 (5파일 목록) + ~600 (Read 20줄) = ~900 chars (≈ 225 tokens), 2회 호출
> - **발견**: architecture/search-system.md ✓ (5개 결과 중 수동 확인 필요)

> [!note] 비교 분석
> 두 방법 모두 정답 문서 발견. MCP는 82% 토큰 절약 + 결과가 확정적(1개 경로).
> FS는 5개 결과 중 어느 것이 alias를 가진 원본 문서인지 추가 판단 필요.

---

### 시나리오 5: 섹션 단위 조회

> [!example]- MCP 결과
> - **도구**: `nexus_get_section`
> - **파라미터**: `project="Obsidian Nexus Docs", path="architecture/search-system.md", heading="하이브리드 검색"`
> - **응답**: ~280 chars (≈ 70 tokens), 1회 호출
> - **내용**: `### 3. 하이브리드 검색` 섹션만 정확히 반환 (RRF 수식 + hybrid_weight 설명)

> [!example]- Filesystem 결과
> - **도구**: `Read architecture/search-system.md` (전체 131줄)
> - **응답**: ~3,900 chars (≈ 975 tokens), 1회 호출
> - **내용**: 검색 모드 3가지 + 메타데이터 리랭킹 + FTS5 토큰 처리 + Alias Fallback + 태그 기반 검색 + 결과 구조 — 필요 정보는 그 중 약 7%

> [!note] 비교 분석
> **가장 명확한 MCP 우위 시나리오**. 동일 정보(하이브리드 검색 섹션)를 얻는 데 MCP는 70토큰, FS는 975토큰 — **93% 절약**.
> 파일이 클수록 효과가 커짐. 현재 search-system.md는 131줄이지만 대형 문서에서는 격차 더 증가.

---

### 시나리오 6: 크로스 프로젝트 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search` (project 파라미터 생략)
> - **파라미터**: `query="MCP", limit=10`
> - **응답**: ~3,500 chars (≈ 875 tokens), 1회 호출
> - **발견 프로젝트**: brain (3건), Obsidian Nexus Docs (7건) — 6개 볼트 중 2개에서 결과
> - **top-1**: brain/Ideas/SkillSmith - MCP 스킬 관리 시스템.md (score 0.0098)
> - **스니펫 + 랭킹 포함**

> [!example]- Filesystem 결과
> - **도구**: `Grep "MCP" **/*.md` × 6개 볼트 경로 (순차 or 병렬)
> - **응답**: ~3,000 chars (≈ 750 tokens, 파일 목록만), 6회 호출
> - **결과**: docs 21개, brain 5개, Obsidian Vault 0개 — 3개 볼트는 별도 실행 필요
> - **스니펫·랭킹 없음**: 파일 경로만 반환, 어느 문서가 가장 관련성 높은지 알 수 없음

> [!note] 비교 분석
> 토큰 수는 FS가 약간 적지만 **정보 밀도** 차이가 큼. MCP는 랭킹 + 스니펫 + 태그 포함.
> FS는 볼트 수에 비례해 grep 호출 증가 (볼트 10개 → 10회 grep). MCP는 항상 1회.
> **FS의 구조적 한계**: 파일 목록에서 관련성 판단 불가 → 추가 Read 필요 → 실질 토큰 증가.

---

### 시나리오 7: 태그 필터링 검색

> [!example]- MCP 결과
> - **도구**: `nexus_search`
> - **파라미터**: `query="검색", project="Obsidian Nexus Docs", tags=["mcp"], limit=5`
> - **응답**: `[]` — 0건 반환, ~10 chars (≈ 3 tokens), 1회 호출
> - **이슈**: `mcp` 태그가 붙은 문서(integrations/mcp-tools.md 등) 다수 존재하나 결과 없음
> - **추정 원인**: tags 파라미터가 문자열 `["mcp"]`로 전달되어 파싱 실패 가능성

> [!example]- Filesystem 결과
> - **도구**: `Grep "tags:.*mcp" **/*.md` → `Grep "검색" {filtered_files}`
> - **응답**: ~100 + ~50 = ~150 chars (≈ 38 tokens), 2회 호출
> - **발견**: context/benchmark/2026-03-20-search-benchmark.md (1건)
> - **FS 한계**: YAML 배열 포맷(`- mcp`) 미탐지 → 실제 mcp 태그 파일 다수 누락

> [!note] 비교 분석
> **두 방법 모두 실패**. MCP는 태그 파라미터 파싱 이슈, FS는 YAML 배열 포맷 불일치.
> 정상 작동 시 MCP가 우위: 태그 인덱스 DB 활용으로 frontmatter 전체 파싱 불필요.
> **액션 아이템**: MCP tags 파라미터 실제 타입 검증 필요 (JSON array vs string).

---

## 발견된 이슈 및 개선 제안

> [!warning] 검색 품질 문제
> 1. **낮은 score는 정상** (0.010~0.015): RRF 공식 특성상 최댓값이 `1/(1+60) ≈ 0.0164`. 실제 Ollama(`nomic-embed-text`)는 정상 실행 중이며 hybrid 검색 정상 동작 확인 (`nexus_status: overall=ready`).
> 2. **한국어 개념 쿼리 → 핵심 문서 미등장** (S2, 개선 필요): "의미 유사도 검색 원리" 쿼리에서 `architecture/search-system.md`가 top-5 밖. Ollama가 동작 중임에도 발생 → 문서의 한국어 설명이 부족하거나 alias가 없어 FTS5 매칭도 실패. **개선 가능**: alias 또는 본문 한국어 보강으로 해결 가능.
> 3. **S7 태그 필터 0건**: MCP tags 파라미터 파싱 이슈 확인 필요.

> [!tip] 개선 제안
> 1. **alias 보강** (즉시 가능, 효과 높음): `architecture/search-system.md`의 frontmatter에 "의미 유사도", "임베딩 검색", "벡터 검색 원리" 추가 → FTS5 alias fallback + 재인덱싱으로 S2 재현 시 개선 기대
> 2. **tags 파라미터 검증**: MCP 서버 핸들러에서 JSON 배열 역직렬화 타입 확인 (`Vec<String>` 파싱 여부)

## 관련 문서

- [[아키텍처]]
- [[검색 시스템]]
- [[integrations/mcp-tools]]
