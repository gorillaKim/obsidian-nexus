---
title: "Dashboard 인기 문서 랭킹 빈 상태 버그 수정"
aliases:
  - ranking-empty-bug
  - 랭킹 빈 상태 버그
tags:
  - devlog
  - troubleshooting
  - sqlite
created: "2026-03-23"
updated: "2026-03-23"
---

<!-- docsmith: auto-generated 2026-03-23 -->

# Dashboard 인기 문서 랭킹 빈 상태 버그 수정

## 배경

인기 문서 랭킹 기능 구현 후 DB에 조회 기록을 직접 INSERT해도 Dashboard에 항상 "아직 조회된 문서가 없습니다"가 표시되는 문제가 발생했다.

## 변경 내용

### 원인 1: indexing_status 값 불일치

`get_popular_documents` 쿼리에서 `WHERE d.indexing_status = 'indexed'`로 필터링하고 있었으나, 실제 DB에 저장된 값은 `'done'`이었다.

**진단**

```sql
SELECT indexing_status, COUNT(*) FROM documents GROUP BY indexing_status;
-- 결과: done|4  (indexed는 0건)
```

**수정**

```rust
WHERE d.indexing_status = 'done'
```

### 원인 2: record_view 함수가 한 번도 호출되지 않음

`document_views` 테이블과 `record_view()` 함수는 V5 마이그레이션에 존재했지만 어떤 유저 플로우에서도 호출하는 코드가 없었다. 결과적으로 `document_views` 테이블이 항상 비어있어 `view_count = 0`이 반환되었다.

**진단**

```bash
sqlite3 ~/.nexus/nexus.db "SELECT COUNT(*) FROM document_views;"
# 결과: 0
```

**수정**

- `record_view_by_path(pool, project_id, file_path)` 헬퍼 함수 추가
- `get_document` Tauri 커맨드(미리보기 클릭)에서 호출
- `open_file` Tauri 커맨드(Obsidian 열기)에서 호출
- 검색 자체는 조회수 증가 안 함 (의도적 설계)

### 영향 범위

- `crates/core/src/search.rs` — `get_popular_documents` 쿼리 조건 수정
- `crates/core/src/db/sqlite.rs` — `record_view_by_path` 헬퍼 추가
- `apps/desktop/src-tauri/src/lib.rs` — `get_document`, `open_file` 커맨드에서 호출

## 결과

Dashboard 인기 문서 랭킹이 실제 조회 기록을 반영하여 정상 표시된다.

## 교훈

- SQL 필터 조건의 enum 값은 반드시 실제 DB 데이터와 크로스체크할 것
- 새 테이블/함수 추가 시 실제 호출 지점까지 연결 여부를 확인할 것 (dead code 위험)

## 관련 문서

- [[2026-03-23-cli-detection-cpu-runaway-fix]]
- [[2026-03-23-mcp-path-mismatch-fix]]
