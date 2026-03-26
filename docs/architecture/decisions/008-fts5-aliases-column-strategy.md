---
title: "FTS5 aliases 컬럼 통합 전략: chunks 테이블 옵션 A 선택"
aliases:
  - fts5-aliases-column-strategy
  - aliases column adr
  - FTS5 aliases 전략
  - aliases 컬럼 결정
tags:
  - decision
  - architecture
  - search
  - fts5
  - sqlite
created: 2026-03-26
updated: 2026-03-26
---

<!-- docsmith: auto-generated 2026-03-26 -->

# FTS5 aliases 컬럼 통합 전략: chunks 테이블 옵션 A 선택

FTS5 전문 검색에서 alias 기반 매칭 품질을 높이기 위해 aliases 컬럼을 추가하는 방식으로 옵션 A(chunks 테이블 컬럼 추가)를 선택한 결정을 기록합니다.

## 상태

채택 (Accepted)

## 배경

한국어 alias로 영문 본문 문서를 검색하는 시나리오(예: "데이터독" → datadog-setup.md)에서 FTS5 검색이 alias를 직접 색인하지 못하는 문제가 있었습니다. 기존 구조에서 `document_aliases` 테이블은 MCP 도구(`nexus_resolve_alias`, `nexus_get_backlinks`)에서 사용하지만, FTS5 색인에는 포함되지 않았습니다.

이를 해결하기 위해 두 가지 옵션이 검토되었습니다.

- **옵션 A**: `chunks` 테이블에 `aliases TEXT` 컬럼을 추가하고, FTS5 외부 콘텐츠 테이블(`content=chunks`) 구조를 그대로 유지
- **옵션 B**: FTS5를 contentless 독립 테이블로 전환하고, 트리거 동기화 로직 전면 수정

## 결정

**옵션 A 선택** — `chunks` 테이블에 `aliases TEXT` 컬럼 추가

## 근거

| 항목 | 옵션 A | 옵션 B |
|------|--------|--------|
| 기존 트리거 재사용 | 가능 | 전면 수정 필요 |
| 마이그레이션 범위 | 컬럼 추가 + FTS rebuild | 테이블 재설계 전체 |
| 구현 복잡도 | 낮음 | 높음 |
| aliases 중복 저장 | 첫 번째 청크에만 저장 | 없음 |

`content=chunks` 외부 콘텐츠 테이블 구조를 유지하면 기존 INSERT/UPDATE/DELETE 트리거 패턴을 그대로 재사용할 수 있습니다. contentless FTS5로 전환할 경우 트리거와 쿼리 로직을 전면 재작성해야 하므로 변경 범위가 지나치게 커집니다.

## 트레이드오프

- aliases는 문서의 **첫 번째 청크에만** 저장되고 나머지 청크는 NULL입니다. 따라서 두 번째 이상 청크에서 매칭된 결과에는 alias 가중치가 적용되지 않습니다. 실용적으로 허용 가능한 수준으로 판단했습니다.
- `document_aliases` 테이블은 그대로 유지됩니다. MCP 도구(`nexus_resolve_alias`, `nexus_get_backlinks`)가 이 테이블에 의존하므로 FTS 색인 통합과 별개로 운영됩니다.

## 마이그레이션

`crates/core/migrations/V6__fts_aliases.sql`에서 처리합니다.

1. `chunks` 테이블에 `aliases TEXT` 컬럼 추가
2. `index_engine.rs`에서 첫 번째 청크 저장 시 aliases 값을 함께 저장
3. FTS5 색인 재구축: `INSERT INTO chunks_fts(chunks_fts) VALUES('rebuild')`

## 관련 파일

- `crates/core/migrations/V6__fts_aliases.sql`
- `crates/core/src/index_engine.rs`
- `crates/core/src/search.rs`

## 관련 문서

- [[검색 시스템 아키텍처]]
- [[module-map]]
- [[007-desktop-onboarding-button]]
