---
title: Search Benchmark Report
tags:
  - benchmark
  - mcp
  - evaluation
---

# Search Benchmark Report

> Date: 2026-03-19 12:30
> Projects: Obsidian Nexus Docs, Obsidian Vault, test-vault (3개)
> Scenarios: 7/7

## 요약

| # | 시나리오 | MCP 호출 | FS 호출 | MCP 토큰 | FS 토큰 | 절약률 | 정확도 |
|---|---------|---------|--------|---------|--------|-------|-------|
| 1 | 키워드 검색 | 1 | 2 | 347 | 654 | 47% | O |
| 2 | 개념 검색 | 1 | 2 | 367 | 1,395 | 74% | O |
| 3 | 멀티홉 탐색 | 3 | 3 | 339 | 1,195 | 72% | O |
| 4 | 별칭 해소 | 1 | 2 | 39 | 107 | 64% | O |
| 5 | 섹션 단위 조회 | 1 | 1 | 57 | 850 | **93%** | O |
| 6 | 크로스 프로젝트 검색 | 1 | 3 | 703 | 455 | -55% | O |
| 7 | 태그 필터링 검색 | 1 | 2 | 239 | 582 | 59% | O |

## 종합

| 항목 | MCP 합계 | Filesystem 합계 | 차이 |
|------|---------|----------------|------|
| 도구 호출 | 9 | 15 | 40% 적음 |
| 추정 토큰 | 2,093 | 5,239 | **60% 절약** |
| 정확도 일치율 | 7/7 | 7/7 | 동일 |

## 시나리오 6 분석 (MCP가 불리한 케이스)

시나리오 6(크로스 프로젝트 검색)에서 MCP가 -55% 더 많은 토큰을 소비했다.

**원인**: MCP는 10개 결과를 관련도순으로 enriched JSON(태그, 백링크 수, 스코어 포함)으로 반환하는 반면, Grep은 매칭 라인만 반환한다. MCP 응답에는 구조화된 메타데이터가 포함되어 있어 문자 수가 많지만, 후속 작업(문서 선택, 우선순위 판단)에서 추가 호출이 필요 없다는 점에서 **총 워크플로우 토큰**은 MCP가 유리할 수 있다.

## MCP 고유 가치 (Filesystem 대체 불가)

### 1. 섹션 단위 조회 (93% 절약)
파일 전체(121줄, 3,400자) 대신 필요한 "하이브리드 검색" 섹션만(228자) 반환. 대형 문서에서 효과 극대화.

### 2. 그래프 탐색
`nexus_get_backlinks`/`nexus_get_links`로 문서 간 관계를 구조화된 JSON으로 즉시 반환. Filesystem에서는 파일 전체를 읽고 `[[...]]` 패턴을 수동 파싱해야 함.

### 3. 별칭 해소 (64% 절약)
영문 "Search System" → `02-검색-시스템.md` 즉시 매핑. Filesystem에서는 모든 파일의 frontmatter를 Grep하고 aliases 블록을 파싱해야 함.

### 4. 크로스 볼트 단일 쿼리
`project` 파라미터 생략으로 3개 볼트를 한 번에 검색. Filesystem에서는 각 볼트 경로마다 별도 Grep 필요.

### 5. 태그 필터링 검색 (59% 절약)
`tags: ["search"]`로 1회 호출. Filesystem에서는 frontmatter Grep → 수동 필터링 → 재검색의 2단계 필요.

### 6. 의미 검색 (74% 절약)
"의미 유사도 검색 원리"로 벡터 검색 섹션을 정확히 찾음. Grep은 정확한 키워드가 필요하고 여러 패턴을 시도해야 함.

## 시나리오 상세

<details>
<summary>시나리오 1: 키워드 검색 — "설정 가이드"</summary>

**MCP** (1회 호출, 1,391자):
```
nexus_search(query="설정 가이드", project="Obsidian Nexus Docs", limit=5)
```
- 결과: 05-설정-가이드.md(5위), 04-데이터베이스-스키마.md(1위), 03-MCP-도구-레퍼런스.md, 01-아키텍처.md
- 정확한 문서를 찾았으나 스코어 순위가 최적이 아님 (설정-가이드가 5위)

**Filesystem** (2회 호출, 2,618자):
```
Grep "설정" --glob "*.md" → 7개 파일 매칭 (18줄)
Read docs/05-설정-가이드.md (첫 50줄)
```
- 결과: 동일 문서 발견, 단 파일명에서 "설정-가이드" 직접 확인 가능

**비교**: MCP는 메타데이터 포함 구조화 결과, FS는 원시 텍스트. 정확도 동일.
</details>

<details>
<summary>시나리오 2: 개념 검색 — "의미 유사도 검색 원리"</summary>

**MCP** (1회 호출, 1,468자):
```
nexus_search(query="의미 유사도 검색 원리", project="Obsidian Nexus Docs", mode="hybrid", limit=5)
```
- 결과: 02-검색-시스템.md(벡터 검색 섹션, 5위), 01-아키텍처.md(1위)
- 의미 검색으로 "벡터", "유사도" 키워드 없이도 관련 문서 발견

**Filesystem** (2회 호출, 5,580자):
```
Grep "벡터|embedding|유사도" --glob "*.md" → 8개 파일, 30+줄
Read 상위 2개 파일 전체
```
- 3개 키워드 OR 패턴 필요, 결과에 중복/노이즈 다수

**비교**: MCP는 자연어 쿼리 1회, FS는 키워드 추측+멀티패턴 필요. **74% 토큰 절약**.
</details>

<details>
<summary>시나리오 3: 멀티홉 탐색 — 아키텍처에서 관련 문서 발견</summary>

**MCP** (3회 호출, 1,358자):
```
nexus_search(query="아키텍처", limit=3) → 04-데이터베이스-스키마.md
nexus_get_backlinks(path="04-데이터베이스-스키마.md") → 06-개발-가이드, 00-프로젝트-개요, 01-아키텍처
nexus_get_links(path="04-데이터베이스-스키마.md") → 01-아키텍처, 02-검색-시스템
```
- 총 5개 관련 문서를 구조화된 그래프로 발견

**Filesystem** (3회 호출, 4,780자):
```
Grep "아키텍처" → 6개 파일 매칭
Read 04-데이터베이스-스키마.md 전체 (130줄)
[[...]] 패턴 수동 파싱 → Glob per link
```
- 파일 전체를 읽고 위키링크를 수동 추출해야 함

**비교**: MCP 그래프 API로 관계 즉시 파악. **72% 토큰 절약**.
</details>

<details>
<summary>시나리오 4: 별칭 해소 — "Search System"</summary>

**MCP** (1회 호출, 158자):
```
nexus_resolve_alias(project="Obsidian Nexus Docs", alias="Search System")
```
- 결과: `02-검색-시스템.md` 즉시 반환

**Filesystem** (2회 호출, 430자):
```
Grep "Search System" --glob "*.md" → 2개 매칭 (02-검색-시스템.md, 07-MCP-시나리오-테스트.md)
Read 02-검색-시스템.md 첫 10줄 (frontmatter에서 aliases 확인)
```
- 동일 결과지만 2단계 필요

**비교**: MCP 158자 vs FS 430자. **64% 절약**. MCP는 alias→문서 직접 매핑.
</details>

<details>
<summary>시나리오 5: 섹션 단위 조회 — "하이브리드 검색"</summary>

**MCP** (1회 호출, 228자):
```
nexus_get_section(project="Obsidian Nexus Docs", path="02-검색-시스템.md", heading="하이브리드 검색")
```
- 결과: 해당 섹션만 반환 (RRF 공식, 가중치 설명)

**Filesystem** (1회 호출, 3,400자):
```
Read docs/02-검색-시스템.md (전체 121줄)
```
- 필요한 섹션은 8줄이지만 전체 파일을 읽어야 함

**비교**: 228자 vs 3,400자. **93% 절약**. 가장 큰 효율 차이.
</details>

<details>
<summary>시나리오 6: 크로스 프로젝트 검색 — "MCP"</summary>

**MCP** (1회 호출, 2,813자):
```
nexus_search(query="MCP", limit=10) → 10개 결과 (3개 볼트)
```
- 관련도순 정렬, 태그/백링크 메타데이터 포함

**Filesystem** (3회 호출, 1,820자):
```
Grep "MCP" --glob "*.md" path="docs/" → 26줄 매칭
Grep "MCP" --glob "*.md" path="~/Documents/Obsidian Vault/"
Grep "MCP" --glob "*.md" path="~/Documents/test-vault/"
```
- 볼트 수만큼 별도 호출 필요, 결과 합산은 수동

**비교**: MCP가 토큰은 55% 더 사용했으나, enriched 메타데이터(스코어, 태그, 백링크)를 포함. 후속 판단에 추가 호출 불필요.
</details>

<details>
<summary>시나리오 7: 태그 필터링 검색 — tags:["search"] + "검색"</summary>

**MCP** (1회 호출, 956자):
```
nexus_search(query="검색", project="Obsidian Nexus Docs", tags=["search"], limit=5)
```
- 결과: 02-검색-시스템.md의 2개 섹션 (키워드 검색, 메타데이터 리랭킹)

**Filesystem** (2회 호출, 2,330자):
```
Grep "tags:" --glob "*.md" -A 5 → 전체 frontmatter 태그 스캔
# "search" 태그 포함 파일: 02-검색-시스템.md 수동 식별
Grep "검색" path="docs/02-검색-시스템.md"
```
- 2단계: 태그 스캔 → 필터링 → 재검색

**비교**: MCP 1회로 태그+쿼리 동시 필터링. **59% 절약**.
</details>

## 방법론

- **토큰 추정**: `response_chars / 4` (한/영 혼합 보수적 추정)
- **공정성**: MCP 먼저 실행하여 Filesystem에 "답을 아는" 유리한 조건 부여
- **호출 카운트**: 병렬 호출도 각각 1회로 개별 카운트
- **MCP 호출 방식**: JSON-RPC stdin/stdout으로 `nexus-mcp-server` 바이너리 직접 호출
- **Filesystem 도구**: Claude Code의 Grep, Read, Glob 도구 사용

## 관련 문서

- [[03-MCP-도구-레퍼런스]]
- [[02-검색-시스템]]
- [[07-MCP-시나리오-테스트]]
