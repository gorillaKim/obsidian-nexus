---
title: "문서 조회수 중복 카운트 방지 — 30분 쿨다운을 원자적 SQL로 구현"
aliases:
  - view-cooldown
  - atomic-sql
  - 조회수 쿨다운
  - 뷰 쿨다운
tags:
  - decision
  - sqlite
  - performance
created: 2026-03-23
updated: 2026-03-23
---

<!-- docsmith: auto-generated 2026-03-23 -->

# 문서 조회수 중복 카운트 방지 — 30분 쿨다운을 원자적 SQL로 구현

인기 문서 랭킹 기능에서 단순 반복 열람이 랭킹을 왜곡하는 문제를 방지하기 위해, 동일 문서의 30분 이내 중복 조회수 기록을 원자적 단일 SQL 쿼리로 차단하는 방식을 채택한 결정을 기록합니다.

## 상태

채택 (Accepted)

## 배경

인기 문서 랭킹 기능(`popular_docs`)은 `document_views` 테이블의 조회 기록을 집계하여 랭킹을 산출합니다. 문서를 열 때마다 조회수가 증가하면 같은 문서를 단기간에 반복해서 열람하는 것만으로도 랭킹이 왜곡됩니다. 이를 방지하기 위해 같은 문서를 30분 이내에 다시 열 경우 카운트를 증가시키지 않는 쿨다운 로직이 필요했습니다.

## 검토한 방법

### 방법 A: SELECT COUNT + INSERT (2-query check-then-act)

```rust
let already_viewed = conn.query_row(
    "SELECT COUNT(*) FROM document_views \
     WHERE document_id = ?1 AND viewed_at > datetime('now', '-30 minutes')",
    params![document_id],
    |row| row.get::<_, i64>(0),
)? > 0;

if !already_viewed {
    conn.execute("INSERT INTO document_views (document_id) VALUES (?1)", params![document_id])?;
}
```

**문제**: TOCTOU(Time-of-Check-Time-of-Use) 레이스 컨디션이 존재합니다. 동시 요청이 들어올 경우 두 요청 모두 `already_viewed = false`를 읽고 각각 INSERT를 실행할 수 있습니다.

### 방법 B: INSERT WHERE NOT EXISTS (단일 원자 쿼리) — 채택

```sql
INSERT INTO document_views (document_id)
SELECT ?1 WHERE NOT EXISTS (
    SELECT 1 FROM document_views
    WHERE document_id = ?1 AND viewed_at > datetime('now', '-30 minutes')
)
```

**장점**:
- 단일 쿼리로 원자성 보장 — TOCTOU 레이스 컨디션 완전 제거
- 쿼리 횟수 절반으로 감소 (성능 개선)
- SQLite의 단일 write lock 내에서 실행되므로 직렬화 보장

## 결정

방법 B를 채택합니다. SQLite는 write 연산이 직렬화되므로 `INSERT WHERE NOT EXISTS` 패턴이 완전한 원자성을 보장합니다. 애플리케이션 레벨의 분기 없이 데이터베이스 레벨에서 중복 방지가 완결됩니다.

## 구현

`crates/core/src/search.rs`의 `record_view()` 함수에 적용되었습니다.

```rust
pub fn record_view(pool: &DbPool, document_id: &str) -> Result<()> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO document_views (document_id)
         SELECT ?1 WHERE NOT EXISTS (
             SELECT 1 FROM document_views
             WHERE document_id = ?1 AND viewed_at > datetime('now', '-30 minutes')
         )",
        params![document_id],
    )?;
    Ok(())
}
```

호출부(`record_view_by_path`)에서는 fire-and-forget 패턴으로 처리하되, 실패 시 `tracing::warn!`으로 가시성을 확보합니다.

## 결과

- 반복 열람에 의한 랭킹 왜곡 방지
- 레이스 컨디션 없는 원자적 쿨다운 구현
- 별도 트랜잭션이나 잠금 없이 SQLite 기본 직렬화 활용

## 관련 문서

- [[search-system]]
- [[module-map]]
