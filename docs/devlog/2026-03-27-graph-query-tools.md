---
title: "관계 그래프 쿼리 도구 3종 구현 (TDD)"
aliases:
  - graph-query-tools-devlog
  - 그래프 쿼리 도구 개발일지
  - nexus-graph-tools
tags:
  - devlog
  - feature
  - graph
  - mcp
  - tdd
created: "2026-03-27"
updated: "2026-03-27"
---

<!-- docsmith: auto-generated 2026-03-27 -->

# 관계 그래프 쿼리 도구 3종 구현 (TDD)

## 배경

기존 MCP 도구는 1-hop 링크 탐색(nexus_get_links, nexus_get_backlinks)만 제공했다. 멀티홉 관계 탐색이 필요한 경우 이 도구를 10회 이상 반복 호출해야 했고, 두 문서 간 경로 파악이나 관련 문서 추천은 클라이언트에서 별도 구현해야 했다. 이를 해결하기 위해 그래프 쿼리 전용 도구 3개를 TDD 방식으로 설계·구현했다.

## 변경 내용

### 주요 변경사항

#### nexus_get_cluster(path, depth=2)

- `wiki_links` 테이블에 재귀 CTE(UNION)로 앞방향 + 역방향 멀티홉 탐색 구현
- `depth.min(5)` 상한 캡 — core 함수와 MCP 핸들러 양쪽 모두 적용
- 반환 필드: `file_path`, `title`, `distance`, `tags`, `snippet`
- 기존 1-hop 탐색 10회 이상 호출을 1회 쿼리로 대체

#### nexus_find_path(from, to)

- 초기 구현: 재귀 CTE UNION ALL → 코드리뷰에서 사이클 발생 시 지수적 폭발 CRITICAL 이슈 발견
- 수정: Rust BFS + `HashSet<String>` visited 세트로 사이클 안전성 확보
- 최대 6홉 제한, 경로 없으면 `None` 반환
- BFS 루프 내 `prepare()` → `prepare_cached()`로 교체하여 반복 파싱 제거

#### nexus_find_related(path, k=10)

- `get_cluster(depth=2)` + 태그 중복 RRF(k=60) 합산으로 관련 문서 추천
- `optional()?`로 에러 전파 (`unwrap_or` 제거)
- N+1 title 쿼리 → 단일 `IN(...)` batch 쿼리로 개선
- `signals` 필드: `["link", "tag"]` 중 기여한 신호 목록 반환

### TDD 흐름

1. 8개 실패 테스트 선작성
   - `get_cluster`: depth-1, depth-2, backlink 포함
   - `find_path`: direct, 2-hop, no-path
   - `find_related`: 결과 반환, signals 필드 포함
2. 함수 구현으로 테스트 통과
3. 전체 120개 테스트 통과 확인

### 코드리뷰 반영 (4건)

| 심각도 | 이슈 | 수정 |
|--------|------|------|
| HIGH | `find_path` BFS 루프 내 `prepare()` 반복 | `prepare_cached()`로 교체 |
| MEDIUM | `find_related` N+1 title 쿼리 | `IN(...)` batch 쿼리로 통합 |
| MEDIUM | "link" signal 중복 push | `contains` 체크 추가 |
| LOW | 테스트 헬퍼 `TempDir` 누수 | `(Project, TempDir)` 튜플 반환으로 수정 |

### 영향 범위

- `crates/core/src/` — 그래프 쿼리 로직 추가
- `crates/mcp-server/src/main.rs` — 3개 MCP 핸들러 등록, NEXUS_HELP_TEXT 업데이트
- `crates/agent/resources/librarian/` — system.md, app-guide.md, search-strategy.md 업데이트
- `docs/integrations/mcp-tools.md` — 새 도구 문서화
- `crates/core/src/templates/claude_md_section.md` — 온보딩 템플릿 반영
- `README.md` — 도구 목록 업데이트
- `apps/desktop/src/components/views/GuideView.tsx` — UI 가이드 반영
- `crates/core/src/onboard.rs` — NEXUS_TOOLS 상수 업데이트

## 결과

- `cargo build --release` 성공 (전체 crate)
- `pnpm build` (frontend) 성공
- 전체 테스트 120개 통과
- 1-hop 도구 10회 이상 반복 호출이 단일 쿼리 1회로 대체됨
- `find_path`의 사이클 안전성 확보 (재귀 CTE → Rust BFS)

## 교훈

- SQL 재귀 CTE UNION ALL은 사이클이 있는 그래프에서 무한 루프 위험 — 방문 추적이 불가한 경우 Rust 레벨 BFS로 처리해야 한다.
- BFS 루프 내 `prepare()`는 반복마다 SQL 파싱 오버헤드가 발생한다. 루프 밖으로 꺼내거나 `prepare_cached()`를 사용한다.
- N+1 쿼리는 small result set에서도 누적 오버헤드가 크다. 관련 데이터는 항상 batch 쿼리로 가져온다.
- TDD에서 테스트 헬퍼의 임시 디렉토리 소유권을 `(T, TempDir)` 튜플로 같이 반환해야 조기 drop을 방지할 수 있다.

## 트러블슈팅: 데스크톱 앱 사이드카 미갱신

### 현상
데스크톱 앱에서 테스트했더니 `nexus_get_cluster`, `nexus_find_related`가 MCP 도구 목록에 없음 (16개만 노출). `cargo build --release`로 빌드했지만 앱에는 반영 안 됨.

### 원인
Tauri 데스크톱 앱은 `apps/desktop/src-tauri/binaries/`의 **사이드카 바이너리**를 사용하며, `cargo build --release`로는 자동 갱신되지 않음. 기존 구버전 MCP 서버 바이너리가 그대로 번들됨.

### 해결
1. aarch64 타겟으로 별도 빌드:
   ```bash
   cargo build --release --target aarch64-apple-darwin -p nexus-mcp-server -p nexus-cli
   ```
2. 사이드카 파일 교체:
   ```bash
   cp target/aarch64-apple-darwin/release/nexus-mcp-server apps/desktop/src-tauri/binaries/nexus-mcp-server-aarch64-apple-darwin
   cp target/aarch64-apple-darwin/release/obs-nexus apps/desktop/src-tauri/binaries/obs-nexus-aarch64-apple-darwin
   ```
3. Tauri 앱 번들 재빌드 + DMG로 설치

### 교훈
MCP 서버 코드 변경 시 사이드카 교체 → Tauri 빌드 → DMG 설치 순서를 반드시 따라야 함. `.claude/rules/build.md`에 이미 문서화되어 있으나 실수하기 쉬운 포인트.

## 관련 문서

- [[mcp-tools]]
- [[module-map]]
- [[search-system]]
