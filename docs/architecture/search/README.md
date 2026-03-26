---
title: 검색 시스템 아키텍처
aliases:
  - search-architecture
  - search-system-index
  - 검색 시스템 아키텍처
tags:
  - search
  - architecture
  - index
created: 2026-03-26
updated: 2026-03-26
---

<!-- docsmith: auto-generated 2026-03-26 -->

# 검색 시스템 아키텍처

이 폴더는 검색 시스템의 구조, 성능, 개선 히스토리를 추적합니다.
단순 아키텍처 문서가 아니라 시간에 따른 검색 품질 변화와 설계 결정을 누적 관리합니다.

## 현재 검색 시스템 개요

### 검색 모드

| 모드 | 설명 |
|------|------|
| **keyword** | FTS5 전문 검색 (unicode61, 한국어 지원) |
| **vector** | sqlite-vec KNN (768D nomic-embed-text) |
| **hybrid** | RRF(Reciprocal Rank Fusion) + 메타데이터 리랭킹 |

### 핵심 컴포넌트

| 모듈 | 파일 | 역할 |
|------|------|------|
| FTS5 | chunks_fts | content, heading_path, aliases 컬럼, bm25 가중치 |
| 벡터 | vec_chunks | 768D nomic-embed-text, L2 정규화 |
| 하이브리드 | search.rs | RRF 합산, 동적 가중치 |
| 리랭킹 | search.rs | backlink_count, view_count, title 부스트 |

### DB 스키마 버전

현재: **V6** (2026-03-26)

| 버전 | 주요 변경 |
|------|-----------|
| V1 | 기본 (chunks, chunks_fts, documents) |
| V3 | sqlite-vec 도입 |
| V4 | wiki_links, document_aliases 추가 |
| V5 | document_views, created_at 추가 |
| V6 | chunks.aliases 컬럼, chunks_fts aliases 컬럼, bm25 가중치 |

## 이 폴더에서 관리하는 문서 유형

```
search/
├── README.md          # 이 파일 — 인덱스 및 개요
├── performance/       # 검색 품질 벤치마크, 쿼리 유형별 성능 기록
├── improvements/      # 개선 이력, 효과 측정
└── tuning/            # bm25 가중치, RRF 파라미터, 임베딩 설정 등 튜닝 기록
```

## 개선 히스토리

### 2026-03-26: alias 검색 통합

- 임베딩 텍스트에 aliases prefix 추가 — 벡터 검색 재현율 향상
- FTS5 aliases 컬럼 + bm25 5배 가중치 — alias 매칭 정밀도 향상
- 관련: [[008-fts5-aliases-column-strategy]]

## 관련 문서

- [[search-system-deep-dive]] — 검색 알고리즘 상세
- [[search-system]] — 검색 모드 설명
- [[search-alias-improvement-plan]] — alias 개선 계획
