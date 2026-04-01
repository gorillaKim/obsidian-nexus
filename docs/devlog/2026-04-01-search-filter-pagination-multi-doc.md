---
title: "검색 필터·페이지네이션·다중 문서 읽기 구현 (TDD)"
aliases:
  - search-filter-pagination-multi-doc
  - 검색 필터 페이지네이션
  - date-filter-search
  - 날짜 필터 검색
  - nexus-get-documents
  - 2026-04-01 검색 개선
tags:
  - devlog
  - feature
  - search
  - pagination
  - tdd
  - bugfix
created: "2026-04-01"
updated: "2026-04-01T05:30:00"
---

<!-- docsmith: auto-generated 2026-04-01 -->

# 검색 필터·페이지네이션·다중 문서 읽기 구현 (TDD)

## 배경

`docs/plan/2026-03-31-search-filter-pagination-onboard.md` 계획서를 바탕으로, TDD 방식으로 날짜 기반 필터 검색, 오프셋 페이지네이션, 다중 문서 일괄 읽기 기능을 구현했다. 기존 검색은 쿼리 텍스트 없이 날짜·태그 조건만으로 문서를 필터링하는 방법이 없었고, 대량 결과에 대한 페이지 이동도 불가능했다.

## 변경 내용

### 1. DateFilter + 날짜 기반 검색 (`crates/core/src/search.rs`)

날짜 범위로 문서를 필터링하는 `DateFilter` struct와 관련 enum을 추가했다.

```rust
DateFilter { date_from, date_to, field: DateField }
DateField: LastModified | CreatedAt
SortBy: Relevance | DateDesc | DateAsc
```

- `get_document_ids_by_date()` — 날짜 범위에 해당하는 document ID set 반환
- `intersect_id_sets()` — date_ids와 tag_ids의 교집합 연산으로 복합 필터 구현

### 2. 오프셋 페이지네이션

`fts_search`, `vector_search`, `hybrid_search` 세 함수 모두에 `offset` 파라미터를 추가했다. rerank 결과의 순서를 보장하기 위해 DB에서 `limit + offset` 개를 fetch한 뒤 rerank를 완료하고, 그 후 `.skip(offset).take(limit)`을 적용하는 rerank-safe 전략을 채택했다.

### 3. filter_search() — 쿼리 없는 필터 전용 모드

텍스트 쿼리 없이 날짜·태그 조건만으로 문서 목록을 반환하는 `filter_search()` 함수를 신설했다. `SortBy::DateDesc`, `SortBy::DateAsc`, `SortBy::Relevance` 세 가지 정렬 방식을 지원한다.

### 4. nexus_get_documents (MCP) / handle_get_docs (CLI)

최대 5개 경로를 한 번에 읽는 다중 문서 일괄 조회 기능을 추가했다. 일부 경로가 존재하지 않더라도 나머지 결과를 반환하는 partial success 방식으로 응답한다.

```json
{"success": {"path1": "...", "path2": "..."}, "errors": {"path3": "not found"}}
```

MCP와 CLI 양쪽에 동일 기능을 제공하여 feature parity를 유지했다.

### 5. Onboard 개선 (`crates/core/src/onboard.rs`)

- 바이너리 탐색 순서를 `~/.local/bin` → `current_exe` dir → PATH 순으로 명시
- `resolve_symlink_or_file()` + `repair_or_create_symlink()` — symlink 상태 검증 및 자동 복구
- `.claude` 폴더 탐색: project 디렉토리에서 HOME 방향으로 topmost 탐색
- `StepStatus::Installed`, `StepStatus::Repaired` enum 변형 추가
- `NEXUS_TOOLS`에 `mcp__nexus__nexus_get_documents` 등록

### 6. MCP 스키마 업데이트 (`crates/mcp-server/src/main.rs`)

`nexus_search` 도구의 `query` 필드를 optional로 변경하고, 아래 파라미터를 추가했다.

| 파라미터 | 설명 |
|----------|------|
| `offset` | 오프셋 페이지네이션 |
| `date_from` / `date_to` | 날짜 범위 필터 (ISO 8601) |
| `date_field` | `last_modified` 또는 `created_at` |
| `sort_by` | `relevance`, `date_desc`, `date_asc` |

응답 형식도 메타데이터를 포함하도록 변경했다.

```json
{
  "results": [...],
  "meta": {"total_returned": 10, "offset": 0, "has_more": true}
}
```

### 7. TDD 테스트 (`crates/core/tests/integration_test.rs`)

7개 테스트를 먼저 작성하고 구현을 완료했다.

| 테스트 | 검증 내용 |
|--------|-----------|
| `test_date_filter_from_only` | date_from만 지정한 필터 |
| `test_date_filter_range` | 날짜 범위 필터 |
| `test_date_tag_intersection` | 날짜 + 태그 교집합 |
| `test_fts_search_with_offset_pagination` | FTS offset 페이지네이션 |
| `test_fts_search_with_date_filter` | FTS + 날짜 필터 조합 |
| `test_filter_search_sort_date_desc` | filter_search 날짜 내림차순 |
| `test_filter_search_offset` | filter_search offset |

## 결과

- 전체 테스트 **127개 통과**
- `cargo clippy` 경고 **0개**

## 교훈

- rerank 이후에 offset을 적용해야 결과 순서가 보장된다. DB 레벨에서 `OFFSET`을 쓰면 rerank 전 순서 기준으로 잘려 정확도가 떨어진다.
- `query` optional화는 MCP 스키마 변경 중 가장 파급 범위가 컸다. 기존 호출자가 query를 항상 넘기던 가정이 곳곳에 있어서 Option 처리를 꼼꼼히 전파해야 했다.

## 관련 문서

- [[2026-03-31-search-filter-pagination-onboard]]
- [[module-map]]
- [[data-flow]]

## 2026-04-01 (2차)

### 1. CLI 태그 필터 패리티 수정

- `filter_by_tags()` 함수 삭제 — deprecated + 미사용 상태 (외부 호출 없음)
  - 이 함수는 `&mut Vec<T>` + `retain()` 패턴으로 `clippy::ptr_arg` 경고를 suppress하고 있었음
  - 올바른 해결은 함수 삭제 (미사용이므로)
- CLI `search` 명령에 `--tags rust,devlog` (콤마 구분, OR 필터) + `--tag-match-all` (AND 모드) 추가
- `handle_search`에 태그 파싱 로직 추가: `split(',')` + `trim()` + `filter(!is_empty())`
- 모든 검색 함수 호출에 `tag_filter.as_ref()` 실제 전달 (기존에 `None`으로 하드코딩되어 있었음)
- MCP/CLI 태그 필터 기능 패리티 완성

### 2. `HashMap` → `BTreeMap` (출력 순서 안정화)

- `handle_get_docs`의 `success`/`errors` 맵이 `HashMap`이라 출력 순서 비결정적
- `BTreeMap`으로 교체 → 알파벳 안정 정렬, 새 의존성 불필요

### 3. `last_modified` DB 저장 버그 수정

- 인덱싱 시 `documents` 테이블에 `last_modified`가 저장되지 않던 버그 발견
  - `INSERT INTO documents`에 `last_modified` 누락
  - UPDATE 쿼리에도 `last_modified = ?5` 미포함
- `index_engine.rs`에서 `abs_path.metadata().modified()`로 파일 수정 시간 추출
- `index_single_file` 시그니처에 `last_modified: Option<&str>` 추가
- UPDATE 쿼리에 `last_modified = ?5` 추가
- 수정 후 날짜 필터(`--date-from`) 정상 동작 확인

### 4. `sanitize_frontmatter_dates()` — 잘못된 날짜 자동 교정

- 인덱싱 시 frontmatter의 `created`/`updated`/`date` 필드 검사
- 유효하지 않은 날짜 (한글, 범위 초과, 불가능한 날짜) 감지 시:
  - 파일 frontmatter를 오늘 날짜로 수정 (`std::fs::write`)
  - DB `last_modified`도 오늘 날짜로 저장
  - 파일 쓰기 실패 시 원본 유지 (DB/파일 불일치 방지)
- 날짜 검증: `chrono::NaiveDate::parse_from_str` 사용 (2026-02-31 같은 불가능한 날짜도 거부)
- 멀티바이트 안전: `chars().take(10).collect()` 사용
- 수정 시 WARN 로그 출력

### 5. 문서/에이전트 프롬프트 업데이트

- `crates/core/src/templates/claude_md_section.md`: `nexus_get_documents`, 날짜/태그 필터, offset 페이지네이션 사용 예시 추가
- `~/.obsidian-nexus/agents/librarian/search-strategy.md`: 날짜 filter-only 모드, 태그 AND/OR, 페이지네이션 섹션 추가

### 코드 리뷰 결과 반영

- `is_valid_date`: 수동 월/일 범위 체크 → `chrono::NaiveDate` 파싱으로 교체
- `fs::write` 실패 시 원본 폴백 (DB/파일 일관성 보장)
- 빈 `//` 주석 제거 (`filter_by_tags` 삭제 잔재)

---

최종 상태: cargo clippy 0개, 127/127 테스트 통과

## 2026-04-01 (3차)

### 1. 데스크톱 빌드 시그니처 불일치 버그 수정

- 이전 세션에서 `fts_search`, `vector_search`, `hybrid_search` 시그니처에 `offset`, `date_filter` 파라미터를 추가했으나 `apps/desktop/src-tauri/src/main.rs`가 미갱신 상태
- E0061 (인자 수 불일치) 컴파일 에러 3개 발생
- `apps/desktop/src-tauri/src/main.rs` 검색 호출에 `0, None` (offset, date_filter) 추가

### 2. nexus_get_sections 신기능 구현 (TDD)

**설계 결정:**
- `SectionRequest<'a> { heading, heading_path: Option<&'a str> }` 구조체 도입 — MCP/CLI 동일 인터페이스
- `heading`만으로 동작, 중복 제목 있을 때만 `heading_path` 사용
- `(success: BTreeMap, errors: BTreeMap)` 반환 — 부분 실패 허용, `nexus_get_documents` 패턴과 일관성
- max 20개 제한

**TDD 흐름:**
1. 테스트 4개 먼저 작성 → 컴파일 에러(Red) 확인
2. `get_sections()` 구현 → 4/4 통과(Green)
3. MCP/CLI 구현

**변경 파일:**
| 파일 | 변경 내용 |
|------|-----------|
| `crates/core/src/search.rs` | `SectionRequest` + `get_sections()` 추가 |
| `crates/mcp-server/src/main.rs` | `nexus_get_sections` 도구 정의 + 핸들러 + HELP_TEXT |
| `crates/cli/src/commands/doc.rs` | `doc sections --heading ... --heading-path ...` 추가 |
| `crates/core/tests/integration_test.rs` | 테스트 4개 추가 |
| `crates/core/src/templates/claude_md_section.md` | 도구 테이블 업데이트 |
| `~/.obsidian-nexus/agents/librarian/search-strategy.md` | nexus_get_sections 가이드 추가 |

**CLI 사용 예시:**
```bash
# heading만 (기본)
obs-nexus doc sections my-proj docs/guide.md --heading 'Intro' --heading 'Usage'

# 중복 제목 있을 때 heading-path 추가
obs-nexus doc sections my-proj docs/guide.md \
  --heading 'Introduction' --heading-path 'API > Introduction'
```

**MCP 응답 형식:**
```json
{"success": {"Intro": "...", "Usage": "..."}, "errors": {"Missing": "Section not found"}}
```

### 결과

- 통합 테스트 **42/42 통과** (get_sections 4개 포함)
- CLI, MCP 서버, 데스크톱 앱 빌드 전부 성공
