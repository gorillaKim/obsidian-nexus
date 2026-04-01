---
title: 날짜/태그 복합 검색 + 페이지네이션 + 다중 문서 읽기 + Onboard 개선
date: 2026-03-31
status: completed
completed_at: 2026-04-01
tags: [plan, search, pagination, onboard]
---

> **구현 완료** — v0.5.11에 전체 반영. 상세 구현 기록: [[2026-04-01-search-filter-pagination-multi-doc]]

# Plan: 날짜/태그 복합 검색 + 페이지네이션 + 다중 문서 읽기

## Context
obsidian-nexus 검색에서 특정 기간 + 특정 태그를 복합 조건으로 필터링하고, 결과를 페이지 단위로 탐색하는 기능이 필요. 추가로 여러 문서 내용을 한 번에 읽어오는 기능도 필요.

현재 상태:
- 태그 필터(`TagFilter`)는 이미 core/search.rs에 구현되어 MCP에도 노출됨
- 날짜 컬럼: `documents.last_modified` (파일 수정일), `documents.created_at` (생성일)
- 페이지네이션 없음 — `limit`만 있음
- 단일 문서 읽기 도구만 존재 (`nexus_get_document`)

핵심 시나리오:
```
nexus_search(tags=["devlog"], date_from="2026-01-01", date_to="2026-03-31", sort_by="date_desc")
→ 결과의 file_path를 그대로
nexus_get_documents(paths=[...최대 5개...])
→ 실제 내용 읽기
```

---

## 수정 파일 목록

| 파일 | 변경 내용 |
|------|----------|
| `crates/core/src/search.rs` | DateFilter struct 추가, 각 검색 함수에 date/offset 파라미터 추가 |
| `crates/mcp-server/src/main.rs` | nexus_search 파라미터 확장, nexus_get_documents 도구 신규 추가 |
| `crates/core/src/onboard.rs` | 바이너리 설치 확인, 심링크 검증, .claude 폴더 탐색 로직 추가 |

---

## Phase 1: DateFilter struct 추가 (`crates/core/src/search.rs`)

### 1-1. DateFilter struct 정의
```rust
pub struct DateFilter {
    pub date_from: Option<String>,  // ISO 8601: "2025-01-01" or "2025-01-01T00:00:00"
    pub date_to: Option<String>,
    pub field: DateField,           // last_modified (기본) or created_at
}

pub enum DateField {
    LastModified,
    CreatedAt,
}
```

### 1-2. `get_document_ids_by_date()` 함수 추가
- `documents.last_modified` 또는 `documents.created_at` 컬럼에 WHERE 절 적용
- `date_from`, `date_to` 중 None인 쪽은 조건 제외 (단방향 범위도 지원)
- 프로젝트 ID 필터 병행 적용

### 1-3. 복합 필터: 날짜 + 태그 교집합
- 두 조건 모두 있을 때: 두 document_id 집합의 교집합(intersection) 사용
- 각각만 있을 때: 해당 집합만 사용
- 둘 다 없으면: None (필터 없음)

### 1-4. 각 검색 함수에 파라미터 추가
`fts_search`, `vector_search`, `hybrid_search` 시그니처 변경:
```rust
pub fn fts_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
    offset: usize,          // 신규
    tag_filter: Option<&TagFilter>,
    date_filter: Option<&DateFilter>,  // 신규
) -> Result<Vec<SearchResult>>
```

### 1-5. offset 적용 방식 (rerank 안전성)
- 검색 시 DB에서 `limit + offset`개 조회
- enrich/rerank 완료 후 Rust에서 `.skip(offset).take(limit)` 적용
- DB OFFSET을 직접 사용하면 리랭킹 결과와 불일치 발생 가능성 있음

### 1-6. `date_to` 경계 처리
- `date_to` 조건: `documents.last_modified < date_to_next_day` (당일 포함)
- 입력이 날짜만(`2025-12-31`)인 경우 자동으로 `2026-01-01 00:00:00`으로 변환
- 시간까지 포함된 입력은 그대로 사용

---

## Phase 2: MCP 서버 확장 (`crates/mcp-server/src/main.rs`)

### 2-1. nexus_search 파라미터 확장

**`query` optional화**: 필수(required) → 선택(optional). 없으면 filter-only 모드로 동작 (태그+날짜 조건만으로 검색).

추가 파라미터:
```json
"query":      { "type": "string", "description": "검색어 (생략 시 필터 조건만으로 검색)" },
"date_from":  { "type": "string", "description": "날짜 범위 시작 (ISO 8601, 예: '2025-01-01' or '2025-01-01T15:00:00Z')" },
"date_to":    { "type": "string", "description": "날짜 범위 끝 (ISO 8601, 예: '2025-12-31'). 당일 포함." },
"date_field": { "type": "string", "description": "last_modified (기본) | created_at" },
"offset":     { "type": "integer", "description": "페이지네이션 오프셋 (기본: 0). has_more가 true면 offset += limit 으로 다음 페이지 조회.", "default": 0 },
"sort_by":    { "type": "string", "description": "relevance (기본) | date_desc | date_asc. 날짜 필터 사용 시 date_desc 권장.", "default": "relevance" }
```

**유효성 검사**: `date_from > date_to`인 경우 에러 반환 (`"date_from must be before date_to"`).

**날짜 필터 사용 시 `last_modified` 자동 포함**: enrich 파라미터 설정과 무관하게 날짜 필터가 있으면 응답에 `last_modified` 항상 포함.

**filter-only 모드 동작**: query 없을 때 FTS/벡터 검색 스킵, 날짜+태그 필터로 문서 목록 직접 조회 후 sort_by 적용.

### 2-2. nexus_get_documents 도구 신규 추가

한 번에 최대 5개 문서 내용을 반환하는 bulk 읽기 도구.

```json
{
  "name": "nexus_get_documents",
  "description": "여러 문서(최대 5개)의 내용을 한 번에 읽어옴",
  "inputSchema": {
    "type": "object",
    "properties": {
      "paths": {
        "type": "array",
        "items": { "type": "string" },
        "maxItems": 5,
        "description": "문서 경로 목록 (project/path 형식 또는 절대경로)"
      },
      "project": { "type": "string", "description": "프로젝트 이름 (경로가 상대경로인 경우 필요)" }
    },
    "required": ["paths"]
  }
}
```

**경로 형식 통일**: `nexus_search` 결과의 `file_path`를 그대로 `paths`에 사용 가능. 절대경로와 상대경로(project/relative) 모두 허용.

**Partial success 응답 구조**:
```json
{
  "success": { "vault/docs/devlog.md": "# 내용..." },
  "errors":  { "vault/docs/missing.md": "File not found" }
}
```

---

## 응답 구조 (페이지네이션)

```json
{
  "results": [...],
  "meta": {
    "total_returned": 20,
    "offset": 0,
    "has_more": true
  }
}
```

**has_more 정확도 보장**: DB에서 `limit + offset + 1`개 조회 후 실제 응답은 `limit`개만 반환. `limit + 1`번째 결과 존재 여부로 판단.

---

## Phase 3: Onboard 기능 개선 (`crates/core/src/onboard.rs`)

### 현재 문제점
- MCP 서버 경로를 `current_exe().parent() + "nexus-mcp-server"`로 찾음 → 설치 여부 검증 없음
- 심링크 유효성 체크 없음 (broken symlink → 조용히 실패)
- `.claude` 폴더 위치를 `project_path/.claude`로 하드코딩 → 상위 폴더 탐색 없음

### 3-1. 바이너리 설치 확인 + 자동 설치

**탐색 순서 (nexus-mcp-server, obs-nexus 공통)**:
1. 심링크 체크: `~/.local/bin/nexus-mcp-server` (또는 `/usr/local/bin/`)
2. 실행 파일 직접 체크: `current_exe().parent()`
3. `PATH` 환경변수 탐색 (`which nexus-mcp-server`)

**심링크 검증**:
- symlink 존재 시 `fs::read_link()` + 대상 파일 존재 여부 확인
- broken symlink 감지 → 재생성 시도

**자동 설치 로직**:
- 바이너리 미발견 시 `~/.local/bin/`에 현재 실행 바이너리 기준 심링크 생성
- 심링크 생성 실패 시 절대 경로를 `.mcp.json`에 직접 기록 (폴백)

### 3-2. 최상위 `.claude` 폴더 탐색

```
project_path → parent → parent → ... → $HOME
각 디렉토리에 .claude/ 폴더가 있으면 기록
→ 가장 상위(HOME에 가까운) .claude 폴더 선택
```

- `.mcp.json`, `CLAUDE.md`: 탐색된 최상위 `.claude` 부모 디렉토리에 생성
- 예: `~/.claude` 발견 → `~/.mcp.json`, `~/CLAUDE.md`
- 탐색 실패 시 기존대로 `project_path/`

### 3-3. OnboardStep 구조 확장

```rust
pub enum StepStatus {
    Created,
    Skipped,
    Error,
    Installed,   // 신규: 바이너리 설치/심링크 생성
    Repaired,    // 신규: broken symlink 수정
}

pub struct OnboardStep {
    pub name: String,
    pub status: StepStatus,
    pub message: String,
    pub path: Option<String>,  // 신규: 실제 생성/수정된 파일 경로
}
```

### 3-4. 전체 온보딩 단계 (개선 후)

| 순서 | 단계 | 동작 |
|------|------|------|
| 1 | 바이너리 탐색 | nexus-mcp-server, obs-nexus 위치 확인 |
| 2 | 심링크 검증/수정 | broken symlink 감지 → 재생성 |
| 3 | 미설치 시 심링크 생성 | `~/.local/bin/`에 심링크 |
| 4 | `.claude` 폴더 탐색 | 최상위 .claude 위치 결정 |
| 5 | `.mcp.json` 생성 | 최상위 .claude 부모 디렉토리 |
| 6 | `settings.json` 생성 | 탐색된 .claude 폴더 내부 |
| 7 | `CLAUDE.md` 생성 | 최상위 .claude 부모 디렉토리 |

MCP, CLI, Desktop은 Core 함수 시그니처 동일하게 유지 → Core 수정만으로 전 계층 반영.

---

## Verification

1. `cargo build -p nexus-mcp-server nexus-cli nexus-core` — 컴파일 오류 없음
2. `cargo test -p nexus-core` — 기존 테스트 통과
3. 검색 시나리오:
   - `nexus_search(tags=["devlog"], date_from="2026-01-01", date_to="2026-03-31", sort_by="date_desc", limit=5)` — query 없이 filter-only 동작 확인
   - `nexus_search(..., offset=5)` — 다음 페이지 + has_more 정확성 확인
   - `nexus_get_documents(paths=[search결과.file_path])` — 경로 형식 호환 확인
4. Onboard 시나리오:
   - 바이너리 없는 환경 → Installed 단계 확인
   - broken symlink 상황 → Repaired 단계 확인
   - `~/.claude` 존재 시 → `.mcp.json`이 `~/`에 생성 확인
5. 사이드카 바이너리 갱신 후 데스크톱 앱 재빌드 (build.md 절차 따름)
