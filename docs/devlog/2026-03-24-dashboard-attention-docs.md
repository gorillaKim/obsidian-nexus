---
title: "Dashboard 관심 필요 문서 섹션 추가 및 v0.4.0 릴리즈"
aliases:
  - dashboard-attention-docs
  - 대시보드 관심 필요 문서
  - attention-docs
created: "2026-03-24"
updated: "2026-03-24"
tags:
  - devlog
  - feature
  - dashboard
---

<!-- docsmith: auto-generated 2026-03-24 -->

# Dashboard 관심 필요 문서 섹션 추가 및 v0.4.0 릴리즈

## 배경

인기 문서 랭킹으로 "자주 보는 문서"를 확인할 수 있게 되었으나, 반대로 "방치되고 있는 문서"를 발견할 수 있는 수단이 없었다. 문서 품질 유지를 위해 열람이 전혀 없거나, 다른 문서에서 참조되지 않거나, 오랫동안 갱신되지 않은 문서를 한눈에 파악할 수 있는 "관심 필요 문서" 섹션을 Dashboard에 추가했다.

## 변경 내용

### Rust — crates/core/src/search.rs

기준값 상수 5개를 한 곳에서 관리하도록 선언했다.

```rust
const ATTENTION_NEVER_VIEWED_GRACE_DAYS: i64 = 7;
const ATTENTION_ORPHAN_GRACE_DAYS: i64 = 30;
const ATTENTION_ORPHAN_MAX_VIEWS: i64 = 3;
const ATTENTION_STALE_DAYS: i64 = 30;
const ATTENTION_STALE_MAX_VIEWS: i64 = 5;
```

`AttentionDoc` 구조체를 추가하고, `get_attention_documents()` 함수를 CTE 패턴으로 구현했다. SQL은 `julianday('now') - julianday(col) > ?` 형태로 날짜 비교 방향을 통일했으며, 세 분류의 우선순위는 `never_viewed > orphan > stale` 순으로 적용했다.

대량 임포트 직후 미열람 문서가 폭발적으로 늘어나는 상황을 방지하기 위해 `ORDER BY created_at DESC`로 최신 문서를 먼저 노출하도록 정렬했다.

### Tauri IPC — apps/desktop/src-tauri/src/main.rs

`get_attention_documents` 커맨드를 등록했다. 기본 limit은 8이다.

### TypeScript — apps/desktop/src/

- `types/index.ts`: `AttentionReason` 유니언 타입(`"never_viewed" | "orphan" | "stale"`)과 `AttentionDoc` 인터페이스 추가
- `App.tsx`: `attentionDocs` state 추가, 기존 `Promise.all` 병렬 fetch에 `get_attention_documents` 호출 포함
- `DashboardView.tsx`: `ReasonBadge` 컴포넌트(미열람=파랑, 고아=주황, 오래됨=노랑)와 `AttentionList` 컴포넌트 추가, 빈 상태 메시지 "관심이 필요한 문서가 없습니다. 문서가 잘 관리되고 있습니다!" 표시

### 영향 범위

- `crates/core/src/search.rs`: `AttentionDoc` struct 및 `get_attention_documents` 함수 추가
- `apps/desktop/src-tauri/src/main.rs`: Tauri 커맨드 등록
- `apps/desktop/src/types/index.ts`: `AttentionReason`, `AttentionDoc` 타입 추가
- `apps/desktop/src/App.tsx`: 병렬 fetch에 관심 문서 로딩 추가
- `apps/desktop/src/components/views/DashboardView.tsx`: `ReasonBadge`, `AttentionList` 컴포넌트 추가

## 설계 결정

- **stale 기준 30일**: 초기에 180일로 설정했으나 너무 오래된 기준은 실용성이 낮다고 판단하여 30일로 조정했다.
- **상수 + SQL 파라미터 바인딩**: 모든 기준값을 Rust 상수로 선언하고 SQL 파라미터 바인딩으로 전달했다. 하드코딩 대신 이 방식을 택해 추후 기준값 조정이 용이하다.
- **프로젝트 탭 필터 없음**: 초기 버전은 전체 프로젝트를 대상으로 표시한다. 프로젝트별 필터는 이후 버전에서 추가 예정이다.
- **새 DB 마이그레이션 없음**: `view_count`, `backlink_count`, `created_at`, `last_modified` 등 기존 컬럼만 활용하므로 스키마 변경이 불필요했다.

## CCG 리뷰 (Gemini) 반영 사항

- 날짜 비교 방향을 `julianday('now') - julianday(col) > N` 형태로 명확히 정렬했다.
- 대량 임포트 시 미열람 폭발 대비로 `ORDER BY created_at DESC` 적용.
- 빈 상태 메시지를 긍정적 문구로 작성했다.

## 릴리즈

- 버전: `0.3.14` → `0.4.0` (minor bump)
- `cargo tauri build` 성공
- GitHub Release: `v0.4.0` 생성
- 빌드 산출물: `Obsidian Nexus_0.4.0_aarch64.dmg`

## 교훈

- 날짜 비교 SQL에서 `julianday()` 함수를 쓸 때 뺄셈 방향(`now - col`)을 일관되게 유지하지 않으면 음수 결과로 필터가 작동하지 않는다. 상수와 함께 파라미터 바인딩을 쓰면 방향 오류를 테스트로 잡기 쉬워진다.
- 대량 데이터 임포트 시나리오를 미리 고려해야 UI 폭발 문제를 설계 단계에서 방지할 수 있다.
- CTE 패턴은 분류 로직이 복잡할 때 SQL 가독성을 크게 높인다.

## 관련 문서

- [[Dashboard 인기 문서 랭킹 기능 구현]]
- [[search-system]]
- [[module-map]]
