---
name: search-benchmark
description: MCP 검색 vs Grep/Read 파일시스템 도구 구조화 벤치마크. 속도, 정확도, 토큰 효율 측정. 단문/장문 검색 품질 검증 포함.
triggers:
  - 'search-benchmark'
  - 'benchmark mcp'
  - 'mcp vs grep'
  - '검색 벤치마크'
  - '검색 품질 검증'
---

# Search Benchmark — MCP vs Filesystem 비교 평가 + 검색 품질 검증

obsidian-nexus MCP 네이티브 도구와 Grep/Read 파일시스템 도구를 동일 시나리오에서 비교 평가한다.
`quality` 모드에서는 단문/장문 쿼리가 의도한 문서를 실제로 반환하는지 PASS/FAIL로 검증한다.
`quality alias` 모드에서는 V6 alias 개선(FTS5 aliases 컬럼 + bm25 5x + 임베딩 prefix)이 실제로 검색 품질을 향상시켰는지 검증한다.

## Input

- `$ARGUMENTS`: 모드 선택
  - `all` (기본) — 효율 비교 전체 7개 시나리오
  - `1`~`8` — 효율 비교 개별 시나리오
  - `quick` — 효율 비교 시나리오 1, 4, 5만
  - **`quality`** — 단문/장문 검색 품질 검증
  - **`quality short`** — 단문 쿼리만
  - **`quality long`** — 장문 쿼리만
  - **`quality alias`** — V6 alias 개선 효과 검증 (FTS5 alias 컬럼, bm25 가중치, 임베딩 강화)

## MCP 호출 방법

nexus MCP 서버가 `.mcp.json`에 등록되어 있으므로 **네이티브 MCP 도구**(`mcp__nexus__nexus_*`)를 직접 호출한다.
JSON-RPC bash 우회는 사용하지 않는다.

**토큰 측정**: 각 MCP 도구 호출의 응답 텍스트 길이(chars)를 기록한다.

## 메트릭 추적 (효율 비교 모드)

각 방법(MCP / Filesystem)에서 아래 항목을 추적한다:

| 항목 | 설명 |
|------|------|
| `tool_calls` | 도구 호출 횟수 (병렬도 개별 카운트) |
| `documents_found` | 발견한 문서 경로 목록 |
| `response_chars` | 각 호출의 응답 문자 수 |
| `estimated_tokens` | `response_chars / 4` 근사 |

## 품질 메트릭 (quality 모드)

| 항목 | 설명 |
|------|------|
| `rank` | 기대 문서가 결과 목록에서 몇 위인지 (미등장 시 ∞) |
| `top1_match` | 1위 결과가 기대 문서인가 (PASS/FAIL) |
| `top3_match` | top-3 내에 기대 문서가 있는가 (PASS/FAIL) |
| `top1_score` | 1위 결과의 relevance score |
| `score_gap` | 1위와 2위의 score 차이 (높을수록 명확한 1위) |

---

## Workflow A: 효율 비교 모드 (all / 1~7 / quick)

### Phase 0: 환경 확인

1. `nexus_list_projects` MCP 도구를 호출하여 인덱싱된 프로젝트 목록 확인
2. 프로젝트가 없으면 **중단**: "인덱싱된 프로젝트 없음. `nexus_index_project` 먼저 실행하세요."
3. 프로젝트 목록에서 볼트 루트 경로를 파악하여 Filesystem 테스트에 사용

### Phase 1: 시나리오 선택

`$ARGUMENTS`를 파싱한다:
- 미입력 또는 `all` → 시나리오 1~7 전체
- 숫자 (`1`~`7`) → 해당 시나리오만
- `quick` → 시나리오 1, 4, 5

### Phase 2: 벤치마크 실행

각 시나리오에서 **Method A (MCP)** 먼저, **Method B (Filesystem)** 나중에 실행한다.
MCP를 먼저 실행하여 Filesystem 쪽에 "이미 답을 아는" 유리한 조건을 준다 (공정성).

---

#### 시나리오 1: 키워드 검색

**목표**: "설정 가이드" 문서를 찾아라.
**기대 결과**: 제목이나 내용에 "설정" + "가이드"가 포함된 문서가 top-3 내에 등장.

**MCP**:
```
nexus_search(query="설정 가이드", project="obsidian-nexus", limit=5)
```

**Filesystem**:
```
Grep "설정" --glob "*.md" → 매칭 파일 목록 확인
Read 상위 1개 파일의 첫 50줄
```

**폴백** (obsidian-nexus 프로젝트가 없을 때): `nexus_list_documents`로 문서 목록을 먼저 가져온 뒤, 첫 번째 문서 제목에서 키워드를 추출하여 쿼리로 사용.

---

#### 시나리오 2: 개념 검색

**목표**: "의미 유사도 검색이 어떻게 동작하는지" 설명하는 문서를 찾아라. 정확한 파일명을 모르는 상황.
**기대 결과**: 벡터 검색, 임베딩, 코사인 유사도를 다루는 문서가 top-3 내 등장.

**MCP**:
```
nexus_search(query="의미 유사도 검색 원리", project="obsidian-nexus", mode="hybrid", limit=5)
```

**Filesystem**:
```
Grep "벡터" --glob "*.md" → 파일 목록
Grep "embedding" --glob "*.md" → 파일 목록
Grep "유사도" --glob "*.md" → 파일 목록
# 3개 결과를 수동 병합/중복 제거
Read 상위 2개 파일
```

---

#### 시나리오 3: 멀티홉 탐색

**목표**: 아키텍처 문서에서 시작하여 2-hop 이내 연결된 모든 문서를 발견하라.

**MCP** (v0.5.9+: `nexus_get_cluster` 단일 호출):
```
nexus_search(query="아키텍처", project="obsidian-nexus", limit=3)
# 상위 결과의 path로:
nexus_get_cluster(project="obsidian-nexus", path="{top_result_path}", depth=2)
# → 앞/역방향 2-hop 이내 모든 문서를 distance, tags, snippet 포함하여 반환
```

**MCP** (구버전 방식 — 비교용):
```
nexus_get_backlinks(project="obsidian-nexus", path="{top_result_path}")  # 병렬
nexus_get_links(project="obsidian-nexus", path="{top_result_path}")      # 병렬
# 2-hop 탐색 시 각 결과마다 다시 호출 필요 (N회)
```

**Filesystem**:
```
Grep "아키텍처" --glob "*.md" → 상위 파일 선택
Read 해당 파일 전체 (wiki-link 추출 필요)
# 파일 내용에서 [[...]] 패턴을 수동 파싱
Glob "**/{link_target}.md" × 각 링크마다
```

**비교 포인트**: 2-hop 탐색에서 구버전은 N+1 도구 호출, `nexus_get_cluster`는 1회 쿼리로 대체.

---

#### 시나리오 4: 별칭 해소

**목표**: 영문 별칭 "Search System"으로 한글 문서를 찾아라.
**기대 결과**: aliases 필드에 "Search System"이 있는 문서 경로 반환.

**MCP**:
```
nexus_resolve_alias(project="obsidian-nexus", alias="Search System")
```

**Filesystem**:
```
Grep "Search System" --glob "*.md"  # frontmatter aliases 필드 탐색
# 매칭 파일에서 aliases: 블록을 확인하여 실제 문서 식별
Read 매칭 파일의 첫 20줄 (frontmatter 확인)
```

**폴백**: obsidian-nexus에 alias가 없으면, `nexus_list_documents`로 문서 목록을 가져와 alias가 있는 문서를 선택.

---

#### 시나리오 5: 섹션 단위 조회

**목표**: 검색 시스템 문서에서 특정 섹션만 가져와라.
**비교 포인트**: 동일 정보를 얻는데 소비한 토큰 차이가 핵심.

**MCP**:
```
nexus_get_section(project="obsidian-nexus", path="02-검색-시스템.md", heading="하이브리드 검색")
```

중복 헤딩이 있을 경우 `heading_path`로 정확히 지정 가능:
```
nexus_get_section(project="obsidian-nexus", path="02-검색-시스템.md", heading_path="검색 아키텍처/하이브리드 검색")
```

**Filesystem**:
```
Read "docs/02-검색-시스템.md" (파일 전체)
# 필요한 섹션은 전체 중 일부이지만, 파일 전체를 읽어야 함
```

---

#### 시나리오 6: 크로스 프로젝트 검색

**목표**: 모든 볼트에서 "MCP" 관련 문서를 동시에 찾아라.

**MCP**:
```
nexus_search(query="MCP", limit=10)  # project 파라미터 생략 = 전체 볼트
```

**Filesystem**:
```
# Phase 0에서 파악한 각 볼트 경로마다:
Grep "MCP" --glob "*.md" path="/vault1/path"
Grep "MCP" --glob "*.md" path="/vault2/path"
# ... 볼트 수만큼 반복
# 결과를 수동 병합
```

**참고**: 인덱싱된 프로젝트가 1개뿐이면 이 시나리오는 시나리오 1과 유사해진다. 리포트에 프로젝트 수를 명시한다.

---

#### 시나리오 7: 태그 필터링 검색

**목표**: 특정 태그가 붙은 문서 중에서 검색하라.

**MCP**:
```
nexus_search(query="검색", project="obsidian-nexus", tags=["mcp"], limit=5)
```

**Filesystem**:
```
Grep "tags:" --glob "*.md" -A 5  # frontmatter에서 태그 블록 찾기
# "mcp" 태그가 포함된 파일 수동 필터링
Grep "검색" path="{filtered_files}"  # 필터된 파일에서 재검색
```

**폴백**: "mcp" 태그가 없으면 `nexus_list_documents`에서 실제 사용 중인 태그를 파악하여 대체.

---

#### 시나리오 8: 목차(TOC) 조회

**목표**: 문서의 전체 목차 구조를 파악하고 원하는 섹션만 정밀 조회하라.
**비교 포인트**: 목차 구조 파악 → 특정 섹션 조회 2단계 워크플로우에서 MCP의 토큰 절약 효과.

**MCP**:
```
nexus_get_toc(project="obsidian-nexus", path="02-검색-시스템.md")
# → TOC에서 heading_path 확인 후 정밀 섹션 조회
nexus_get_section(project="obsidian-nexus", path="02-검색-시스템.md", heading_path="{heading_path}")
```

**Filesystem**:
```
Read "docs/02-검색-시스템.md" (파일 전체)
# 헤딩 구조를 파악하려면 전체 파일을 읽어야 함
# 중복 헤딩이 있을 경우 수동으로 위치 파악 필요
```

**검증 포인트**: `nexus_get_toc`가 `{heading, level, heading_path}` 구조로 반환하는지 확인. `heading_path`를 그대로 `nexus_get_section`에 전달하여 정확한 섹션이 반환되는지 검증.

---

#### 시나리오 9: 그래프 쿼리 도구 3종

**목표**: 관련 문서 추천, 경로 탐색, 클러스터 탐색의 정확도와 응답 품질을 측정하라.

**9-A: 관련 문서 추천**

**MCP**:
```
nexus_find_related(project="obsidian-nexus", path="architecture/search-system.md", k=5)
# → signals(["link", "tag"]) 포함 상위 5개 반환
```

**CLI** (동일 기능):
```bash
obs-nexus graph related "obsidian-nexus" "architecture/search-system.md" --k 5 --format json
```

**검증 포인트**: `signals` 필드에 `"link"` 또는 `"tag"`가 포함되는지. 점수 순 정렬 여부.

---

**9-B: 두 문서 간 경로 탐색**

**MCP**:
```
nexus_find_path(project="obsidian-nexus", from="devlog/2026-03-23-dashboard-popular-ranking.md", to="architecture/search-system.md")
# → path 배열 + hops 반환
```

**CLI**:
```bash
obs-nexus graph path "obsidian-nexus" "devlog/2026-03-23-dashboard-popular-ranking.md" "architecture/search-system.md" --format json
```

**주의**: resolve된 위키링크(`[[파일경로]]` 형식)만 탐색. `nexus_get_links`로 `"resolved": false` 비율 사전 확인 권장.

---

**9-C: N-hop 클러스터**

**MCP**:
```
nexus_get_cluster(project="obsidian-nexus", path="architecture/search-system.md", depth=2)
```

**CLI**:
```bash
obs-nexus graph cluster "obsidian-nexus" "architecture/search-system.md" --depth 2 --format json
```

**검증 포인트**: `distance` 필드가 1~depth 범위인지. `tags`, `snippet` 포함 여부.

---

### Phase 3: 메트릭 수집 (효율 비교)

각 시나리오 완료 후 아래 형태로 결과를 정리한다:

```
시나리오 N: {이름}
├─ MCP:  tool_calls={n}, documents={list}, response_chars={n}, est_tokens={n}
├─ FS:   tool_calls={n}, documents={list}, response_chars={n}, est_tokens={n}
├─ 정확도 일치: {Yes/No} (양쪽이 동일 핵심 문서를 찾았는지)
└─ MCP 고유 가치: {filesystem으로 불가능했던 것}
```

### Phase 4: 리포트 생성 (효율 비교)

`.claude/skills/search-benchmark/report-template.md` 템플릿을 기반으로 리포트를 작성한다.

1. 템플릿 파일을 읽는다
2. `{{PLACEHOLDER}}` 자리를 실제 벤치마크 결과로 채운다
3. 완성된 리포트를 `docs/architecture/search/performance/{YYYY-MM-DD}-search-benchmark.md`에 저장한다

---

## Workflow B: 검색 품질 검증 모드 (`quality`)

단문/장문 쿼리가 **의도한 문서를 실제로 반환하는지** PASS/FAIL로 검증한다.
효율이 아닌 **정확도**가 핵심이다.

### Phase 0: 환경 확인

1. `nexus_list_projects`로 사용 가능한 프로젝트 목록 확인
2. 각 프로젝트의 실제 문서 목록을 `nexus_list_documents`로 확인
3. 아래 시나리오의 `expected_contains`가 실제 존재하는 문서에 매칭되는지 사전 확인
   - 없으면 해당 시나리오는 **SKIP** 처리 (인덱스 없음으로 기록)

### Phase 1: 단문 쿼리 시나리오 (Short Query)

`quality long`이 아닌 경우 실행한다.

단문 = 1~4 토큰, 도메인 약어/단어형 쿼리.
**판정 기준**: `top3_match = PASS` (기대 문서가 top-3 내 등장)

---

#### QS-1: 영문 약어 검색

**쿼리**: `"RRF"`
**mode**: `keyword`
**expected_contains**: 파일명 또는 스니펫에 "RRF" 또는 "Reciprocal Rank" 포함
**판정**: top-3 내 등장 시 PASS

```
nexus_search(query="RRF", mode="keyword", limit=5)
```

**검증 포인트**: 짧은 대문자 약어가 FTS5에서 정확히 매칭되는가.

---

#### QS-2: 한글 단어 검색

**쿼리**: `"임베딩"`
**mode**: `keyword`
**expected_contains**: 파일명 또는 스니펫에 "임베딩" 또는 "embedding" 포함
**판정**: top-3 내 등장 시 PASS

```
nexus_search(query="임베딩", mode="keyword", limit=5)
```

**검증 포인트**: 한글 단어가 unicode61 tokenizer로 정확히 분리되는가.

---

#### QS-3: 영문 단어 검색

**쿼리**: `"alias"`
**mode**: `keyword`
**expected_contains**: 스니펫에 "alias" 포함
**판정**: top-3 내 등장 시 PASS

```
nexus_search(query="alias", mode="keyword", limit=5)
```

**검증 포인트**: 영문 단어가 대소문자 무관하게 매칭되는가.

---

#### QS-4: 혼합 단문 (한+영)

**쿼리**: `"vector 검색"`
**mode**: `hybrid`
**expected_contains**: 벡터 검색, 유사도, embedding 관련 문서
**판정**: top-3 내 등장 시 PASS

```
nexus_search(query="vector 검색", mode="hybrid", limit=5)
```

**검증 포인트**: 한영 혼합 2토큰 쿼리에서 hybrid 검색이 올바른 문서를 반환하는가.

---

#### QS-5: 단문 점수 분포 확인

**쿼리**: `"MCP"`
**mode**: `hybrid`
**판정**: 1위와 2위 score_gap > 0.01 이면 PASS (명확한 1위 존재)

```
nexus_search(query="MCP", mode="hybrid", limit=5)
```

**검증 포인트**: 단문 쿼리에서 score가 특정 문서에 집중되는가, 아니면 고르게 분산되는가.

---

### Phase 2: 장문 쿼리 시나리오 (Long Query)

`quality short`가 아닌 경우 실행한다.

장문 = 5토큰 이상, 자연어 문장형 쿼리.
**판정 기준**: `top5_match = PASS` (기대 문서가 top-5 내 등장)
장문은 단문보다 관련 문서가 분산될 수 있으므로 기준을 top-5로 완화한다.

---

#### QL-1: 자연어 기능 설명 쿼리

**쿼리**: `"검색 결과가 없을 때 alias로 보충하는 원리"`
**mode**: `hybrid`
**expected_contains**: 검색 시스템, alias fallback, document_aliases 관련 문서
**판정**: top-5 내 등장 시 PASS

```
nexus_search(query="검색 결과가 없을 때 alias로 보충하는 원리", mode="hybrid", limit=5)
```

**검증 포인트**: 구체적인 기능 설명 문장이 관련 설계 문서로 이어지는가.

---

#### QL-2: 사용자 업무 용어 → 기술 문서 매핑

**쿼리**: `"overview 페이지 리뉴얼"`
**mode**: `hybrid`
**expected_contains**: performance-report, 대시보드, 리뉴얼 관련 문서
**판정**: top-5 내 등장 시 PASS

```
nexus_search(query="overview 페이지 리뉴얼", mode="hybrid", limit=5)
```

**검증 포인트**: 사용자 자연어(UX 용어)가 기술 문서명(performance-report)과 연결되는가.
이 시나리오는 alias 토큰 매칭이 핵심 — alias에 "overview", "리뉴얼"이 등록된 경우만 PASS 가능.

---

#### QL-3: 문제 해결형 쿼리

**쿼리**: `"Ollama 연결 실패했을 때 검색은 어떻게 동작하나"`
**mode**: `hybrid`
**expected_contains**: embedding, graceful fallback, 연결 오류 관련 문서
**판정**: top-5 내 등장 시 PASS

```
nexus_search(query="Ollama 연결 실패했을 때 검색은 어떻게 동작하나", mode="hybrid", limit=5)
```

**검증 포인트**: 에러 시나리오 설명 쿼리가 관련 구현 문서로 이어지는가.

---

#### QL-4: 설정 방법 쿼리

**쿼리**: `"프로젝트 처음 설정하고 인덱싱하는 방법"`
**mode**: `hybrid`
**expected_contains**: 설치, 설정 가이드, onboarding, index_project 관련 문서
**판정**: top-5 내 등장 시 PASS

```
nexus_search(query="프로젝트 처음 설정하고 인덱싱하는 방법", mode="hybrid", limit=5)
```

**검증 포인트**: How-to 형식의 장문 쿼리가 가이드 문서로 이어지는가.

---

#### QL-5: 비교 분석형 쿼리

**쿼리**: `"FTS5 키워드 검색과 벡터 검색의 차이점과 각각 언제 쓰는지"`
**mode**: `hybrid`
**expected_contains**: 검색 시스템, hybrid search, 검색 모드 비교 관련 문서
**판정**: top-5 내 등장 시 PASS

```
nexus_search(query="FTS5 키워드 검색과 벡터 검색의 차이점과 각각 언제 쓰는지", mode="hybrid", limit=5)
```

**검증 포인트**: 비교/설명 요청 문장이 개념 정리 문서로 이어지는가.

---

### Phase 3: 품질 판정 집계

각 시나리오 완료 후 아래 형태로 결과를 정리한다:

```
[QS-1] 영문 약어 "RRF"
  top-1: {문서명} (score: {n})
  top-3: [{문서1}, {문서2}, {문서3}]
  expected match: top-{rank}위에서 발견 / 미발견
  판정: ✅ PASS / ❌ FAIL / ⏭ SKIP
  실패 원인 (FAIL 시): {인덱스 없음 / alias 미등록 / FTS 토큰화 실패 / 스코어 희석 등}
```

### Phase 4: 품질 리포트 생성

완료 후 아래 형식으로 리포트를 `docs/architecture/search/performance/{YYYY-MM-DD}-search-quality.md`에 저장한다.

**요약 테이블**:

```markdown
## 단문 쿼리 결과

| # | 쿼리 | mode | top-1 문서 | 기대 매칭 | rank | 판정 |
|---|------|------|-----------|---------|------|------|
| QS-1 | "RRF" | keyword | {문서명} | {기대} | {n}위 | ✅/❌ |
| QS-2 | "임베딩" | keyword | ... | ... | ... | ... |
| QS-3 | "alias" | keyword | ... | ... | ... | ... |
| QS-4 | "vector 검색" | hybrid | ... | ... | ... | ... |
| QS-5 | "MCP" (score gap) | hybrid | ... | gap:{n} | — | ✅/❌ |

## 장문 쿼리 결과

| # | 쿼리 (요약) | mode | top-1 문서 | 기대 매칭 | rank | 판정 |
|---|------------|------|-----------|---------|------|------|
| QL-1 | alias 보충 원리 | hybrid | ... | ... | ... | ... |
| QL-2 | overview 리뉴얼 | hybrid | ... | ... | ... | ... |
| QL-3 | Ollama 연결 실패 | hybrid | ... | ... | ... | ... |
| QL-4 | 처음 설정 방법 | hybrid | ... | ... | ... | ... |
| QL-5 | FTS5 vs 벡터 차이 | hybrid | ... | ... | ... | ... |

## 종합

- 단문 PASS율: {n}/5
- 장문 PASS율: {n}/5
- 전체 PASS율: {n}/10
- 주요 실패 패턴: {예: alias 미등록, 짧은 쿼리 score 희석}
- 개선 제안: {예: alias 추가, hybrid_weight 조정}
```

**옵시디언 호환 필수 사항**:
- YAML frontmatter: `title`, `date`, `tags`, `aliases` 포함
- Callout 문법 사용: `> [!info]`, `> [!tip]`, `> [!example]-`
- 접이식 상세는 `> [!example]-` (마이너스로 기본 접힘)

---

---

## Workflow C: Alias 개선 효과 검증 모드 (`quality alias`)

V6 마이그레이션(FTS5 aliases 컬럼 + bm25 5x 가중치 + 임베딩 prefix 강화)이 실제로 효과가 있는지 검증한다.
**핵심 질문**: alias 키워드로 검색했을 때 해당 문서가 content 매칭 문서보다 높은 순위에 오는가?

### Phase 0: alias 등록 문서 동적 탐색

고정 쿼리 대신, 실제 인덱싱된 문서에서 alias가 등록된 문서를 먼저 파악한다.

```
1. nexus_list_documents(project="obsidian-nexus") → 전체 문서 목록 확인
2. 상위 5~10개 문서에 대해 nexus_get_metadata(project, path) 병렬 호출
3. aliases 필드가 있는 문서 목록 수집
4. alias가 없으면 → SKIP 처리: "alias 등록 문서 없음. 인덱싱 후 재시도 권장"
5. alias가 있으면 → 아래 시나리오에서 실제 alias 값을 쿼리로 사용
```

---

### Phase 1: Alias FTS 검색 시나리오

#### QA-1: alias 값 직접 검색 → top-1 반환

**목표**: 문서 A의 frontmatter `aliases`에 등록된 값을 그대로 쿼리로 사용했을 때 문서 A가 top-1에 오는가.

**동적 구성**:
- Phase 0에서 alias가 있는 문서 선택 (예: `aliases: ["검색 시스템", "Search System"]`)
- 쿼리: alias 값 중 하나 (예: `"검색 시스템"`)

```
nexus_search(query="{alias_value}", mode="keyword", project="obsidian-nexus", limit=5)
```

**판정**: top-1이 해당 문서이면 PASS
**검증 포인트**: FTS5 aliases 컬럼에 데이터가 실제로 저장되었는가, bm25 5x 가중치로 다른 문서를 누르고 1위가 되는가.

---

#### QA-2: alias 매칭 score > content 매칭 score

**목표**: 동일 키워드가 한 문서의 aliases에 있고 다른 문서의 content에 있을 때, aliases 매칭 문서가 더 높은 score를 받는가.

**동적 구성**:
- Phase 0에서 alias 값(예: "검색 시스템")을 선택
- 해당 단어가 content에도 등장하는 다른 문서가 있는지 확인

```
nexus_search(query="{alias_value}", mode="keyword", project="obsidian-nexus", limit=10)
```

**판정**: alias 등록 문서의 rank < content에만 해당 단어가 있는 문서의 rank → PASS
**검증 포인트**: bm25(chunks_fts, 1.0, 0.5, 5.0) 가중치가 실제로 alias 컬럼 매칭을 우선시하는가.

---

#### QA-3: 다중 alias 매칭 → 단일 alias 매칭보다 높은 score

**목표**: alias가 2개 이상 쿼리에 매칭되는 문서가 1개만 매칭되는 문서보다 높은 score를 받는가.

**구성**:
- alias가 여러 개인 문서를 Phase 0에서 파악
- 두 alias 값을 모두 포함하는 쿼리: `"{alias1} {alias2}"`

```
nexus_search(query="{alias1} {alias2}", mode="keyword", project="obsidian-nexus", limit=10)
```

**판정**: 두 alias가 모두 있는 문서가 하나만 있는 문서보다 score가 높으면 PASS
**검증 포인트**: bm25은 매칭 토큰이 많을수록 score가 높아지는가 (다중 alias 효과).
이 시나리오가 SKIP되면 "alias 2개 이상인 문서 없음"으로 기록.

---

#### QA-4: document_aliases 테이블 regression

**목표**: V6 마이그레이션 후 `nexus_resolve_alias`(document_aliases 의존)가 여전히 동작하는가.

**구성**:
- Phase 0에서 찾은 alias 값으로 resolve_alias 호출

```
nexus_resolve_alias(project="obsidian-nexus", alias="{alias_value}")
```

**판정**: 해당 문서 경로가 반환되면 PASS (document_aliases 테이블이 여전히 존재하고 데이터가 있음)
**검증 포인트**: V6에서 chunks_fts를 DROP & CREATE했지만 document_aliases 테이블은 건드리지 않았는가 (regression).

---

#### QA-5: 벡터 검색에서 alias 임베딩 효과

**목표**: alias 관련 쿼리로 vector 검색 시, alias prefix 임베딩 강화 덕분에 해당 문서가 상위에 오는가.

**구성**:
- Phase 0에서 alias가 있는 문서 선택
- 해당 alias 값을 vector 검색 쿼리로 사용 (content에는 해당 단어가 없거나 적은 경우)

```
nexus_search(query="{alias_value}", mode="vector", project="obsidian-nexus", limit=5)
```

**판정**: top-3 내에 해당 문서 등장 시 PASS
**검증 포인트**: `build_embed_text()`가 alias prefix를 삽입했기 때문에 벡터 공간에서 alias 쿼리와 가까워졌는가.
Ollama 비활성 시 → SKIP (벡터 검색 불가).

---

### Phase 2: 결과 집계

```
[QA-1] alias 직접 검색 top-1
  쿼리: "{alias_value}" (문서: {doc_path})
  top-1: {문서명} (score: {n})
  판정: ✅ PASS / ❌ FAIL / ⏭ SKIP
  실패 원인 (FAIL 시): {aliases 컬럼 비어있음 / bm25 가중치 미적용 / 인덱스 없음}

[QA-2] alias score > content score
  alias 문서: {doc} (rank: {n}, score: {s})
  content 문서: {doc} (rank: {n}, score: {s})
  판정: ✅ PASS / ❌ FAIL / ⏭ SKIP

[QA-3] 다중 alias > 단일 alias score
  ...

[QA-4] document_aliases regression
  nexus_resolve_alias("{alias}") → {반환값}
  판정: ✅ PASS / ❌ FAIL

[QA-5] 벡터 검색 alias 임베딩 효과
  ...
```

### Phase 3: alias 검증 리포트 생성

`docs/architecture/search/performance/{YYYY-MM-DD}-alias-search-validation.md`에 저장한다.

```markdown
## Alias 검색 개선 검증 (V6)

| # | 검증 항목 | 쿼리 | 결과 | 판정 |
|---|---------|------|------|------|
| QA-1 | alias top-1 반환 | "{alias}" | rank {n} | ✅/❌/⏭ |
| QA-2 | alias score > content score | "{alias}" | alias:{s1} vs content:{s2} | ✅/❌/⏭ |
| QA-3 | 다중 alias 부스트 | "{a1} {a2}" | {score 비교} | ✅/❌/⏭ |
| QA-4 | document_aliases regression | resolve_alias | {경로} | ✅/❌ |
| QA-5 | 벡터 alias 임베딩 | "{alias}" (vector) | rank {n} | ✅/❌/⏭ |

### 종합 판정
- PASS율: {n}/5
- V6 alias 개선 효과: {검증됨 / 부분적 / 미확인}
- 주요 발견: {예: bm25 5x 가중치 효과 확인, 벡터 검색은 재인덱싱 필요}
- 권장 조치: {예: 전체 재인덱싱, alias 추가 등록}
```

---

## Rules

1. **동일 쿼리**: MCP와 Filesystem에서 같은 검색어를 사용한다. 쿼리를 방법에 맞춰 바꾸지 않는다.
2. **공정한 순서**: 효율 비교 모드에서 MCP 먼저 실행하여 Filesystem에게 "답을 아는" 유리한 조건을 준다.
3. **개별 카운트**: 병렬 호출도 각각 1회로 카운트한다.
4. **토큰 근사**: `response_chars / 4`로 추정한다. 한/영 혼합 특성상 보수적 추정.
5. **MCP 미연결 시**: 네이티브 MCP 도구가 없으면 사용자에게 알리고 중단한다.
6. **폴백 전략**: 대상 프로젝트가 없으면 `nexus_list_documents`로 동적 쿼리를 생성한다.
7. **SKIP 처리**: expected_contains 문서가 인덱스에 없으면 FAIL이 아닌 SKIP으로 기록한다.
8. **실패 원인 분류**: FAIL 시 아래 중 하나로 원인을 명시한다.
   - `alias 미등록`: 관련 alias가 document_aliases에 없음
   - `FTS 토큰화 실패`: 쿼리 토큰이 FTS5에서 인식 안 됨
   - `스코어 희석`: 관련 문서가 있지만 top-N 바깥으로 밀림
   - `인덱스 없음`: 해당 문서가 인덱싱되지 않음
   - `벡터 비활성`: Ollama 연결 없어 vector score 0
9. **리포트 자족성**: 쿼리 문자열, 수치, 문서 목록을 모두 포함하여 재현 가능하게 한다.
