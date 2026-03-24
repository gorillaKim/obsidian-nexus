---
title: "관심 필요 문서 기준값 상수화 및 초기값 설정"
aliases:
  - attention-docs-thresholds
  - attention-constants
  - 관심문서 기준값
  - 관심 필요 문서 상수
tags:
  - decision
  - architecture
  - dashboard
created: 2026-03-24
updated: 2026-03-24
---

<!-- docsmith: auto-generated 2026-03-24 -->

# 관심 필요 문서 기준값 상수화 및 초기값 설정

대시보드 관심 필요 문서 섹션에서 사용하는 미열람/고아/오래됨 분류 기준값을 Rust 상수로 선언하고 SQL 파라미터 바인딩으로 전달하는 방식을 채택한 결정을 기록합니다.

## 상태

채택 (Accepted)

## 배경

대시보드에 관심 필요 문서(`attention_needed`) 섹션을 추가하면서 세 가지 분류 기준이 필요했습니다.

- **미열람(never-viewed)**: 생성 후 일정 기간이 지났으나 한 번도 열람되지 않은 문서
- **고아(orphan)**: 백링크가 없고 조회수가 낮아 연결이 끊긴 문서
- **오래됨(stale)**: 최근 업데이트 후 오랜 시간이 지났으나 조회수가 낮아 방치된 문서

이 기준값들을 코드 어디에, 어떤 방식으로 관리할지 결정이 필요했습니다.

## 검토한 방법

### 방법 A: SQL 문자열 하드코딩

기준값을 SQL 쿼리 문자열 안에 리터럴로 직접 삽입합니다.

```sql
WHERE created_at < datetime('now', '-7 days') AND view_count = 0
```

**문제**: 기준값 변경 시 SQL 문자열 내부를 직접 수정해야 합니다. 같은 값이 여러 쿼리에 분산되면 일부만 수정되는 오류가 발생할 수 있으며, SQL injection 방어 패턴(파라미터 바인딩)과도 맞지 않습니다.

### 방법 B: 설정 파일 (settings.json)

사용자가 UI에서 조정할 수 있도록 설정 파일에 기준값을 저장합니다.

**문제**: 초기 버전에는 설정 파일 읽기/쓰기 인프라, UI 설정 화면, 기본값 마이그레이션 등 과도한 복잡도가 추가됩니다. 현 단계에서 사용자가 이 기준값을 조정할 필요성이 검증되지 않았습니다.

### 방법 C: Rust 상수 선언 + SQL 파라미터 바인딩 — 채택

`crates/core/src/search.rs` 상단에 상수를 선언하고 SQL 파라미터로 전달합니다.

```rust
const ATTENTION_NEVER_VIEWED_GRACE_DAYS: i64 = 7;
const ATTENTION_ORPHAN_GRACE_DAYS: i64 = 30;
const ATTENTION_ORPHAN_MAX_VIEWS: i64 = 3;
const ATTENTION_STALE_DAYS: i64 = 30;
const ATTENTION_STALE_MAX_VIEWS: i64 = 5;
```

**장점**:
- 한 파일 한 위치에서 모든 기준값 관리
- SQL 파라미터 바인딩을 통해 안전하게 전달
- 변경 시 상수 수정 후 재컴파일만으로 반영
- 코드 가독성: 상수명으로 의미가 명확하게 드러남

## 결정

방법 C를 채택합니다. 초기 버전에서는 기준값의 사용자 조정 필요성이 검증되지 않았으므로 설정 파일 방식은 과도합니다. Rust 상수로 한 곳에서 관리하면 변경 지점이 명확하고, SQL 파라미터 바인딩과 자연스럽게 결합됩니다.

## 상수 초기값 결정

`ATTENTION_STALE_DAYS`는 초기 제안값 180일에서 30일로 조정했습니다.

- **180일 제안 근거**: 장기 방치 문서만 노출하여 노이즈를 줄임
- **30일 채택 근거**: 지식 관리 관점에서 30일 이내 조기 관심 유도가 더 실용적입니다. 180일이면 사실상 이미 완전히 잊힌 문서만 노출되어 개선 행동을 이끌기 어렵습니다.

| 상수 | 값 | 의미 |
|------|-----|------|
| `ATTENTION_NEVER_VIEWED_GRACE_DAYS` | 7 | 생성 후 7일 유예, 이후에도 미열람이면 노출 |
| `ATTENTION_ORPHAN_GRACE_DAYS` | 30 | 생성 후 30일 유예, 이후 백링크 없으면 고아로 분류 |
| `ATTENTION_ORPHAN_MAX_VIEWS` | 3 | 조회수 3 이하인 백링크 없는 문서를 고아로 분류 |
| `ATTENTION_STALE_DAYS` | 30 | 마지막 수정 후 30일 경과 시 오래됨으로 분류 |
| `ATTENTION_STALE_MAX_VIEWS` | 5 | 조회수 5 이하인 오래된 문서를 관심 필요로 분류 |

## 구현

`crates/core/src/search.rs` 상단에 선언되었으며, `attention_needed()` 함수 내 SQL 쿼리에 파라미터로 바인딩됩니다.

## 결과

- 기준값 변경 시 `search.rs` 상수 수정 한 곳만 변경하면 됨
- 사용자 설정화는 v2 고려 사항으로 보류
- SQL 파라미터 바인딩 패턴 일관성 유지

## 관련 문서

- [[001-view-cooldown-atomic-sql]]
- [[search-system]]
- [[module-map]]
