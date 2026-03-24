---
title: "청크 규모 확장과 검색 품질 분석"
aliases:
  - chunk-scalability
  - 청크확장성
  - 검색스케일링
  - chunk-growth-impact
tags:
  - architecture
  - search
  - scalability
  - performance
  - vector-search
  - fts5
created: "2026-03-24"
updated: "2026-03-24"
---

# 청크 규모 확장과 검색 품질 분석

> 문서가 쌓이면서 관리하는 청크 수가 늘어날 때, 검색 시스템의 성능과 품질에 어떤 영향을 미치는지 [[architecture/search-system-deep-dive|검색 시스템 상세 분석]] 기반으로 정리한다.

---

## 요약

| 영역 | 청크 증가 영향 | 심각도 |
|------|--------------|--------|
| FTS5 키워드 검색 | 거의 없음 | ✅ 낮음 |
| RRF 하이브리드 품질 | 거의 없음 | ✅ 낮음 |
| 증분 인덱싱 비용 | 거의 없음 | ✅ 낮음 |
| 벡터 KNN 검색 속도 | 선형 증가 | ⚠️ 중간 |
| 태그 필터 오버패치 | 5배 증폭 | ⚠️ 중간 |
| 검색 노이즈 | 점진적 증가 | ⚠️ 중간 |

---

## 걱정하지 않아도 되는 부분

### 1. FTS5 키워드 검색

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
    content, heading_path,
    content=chunks,
    tokenize='unicode61'
);
```

FTS5는 수백만 건도 무리 없이 처리하도록 설계된 **B-tree 기반 인덱스**다. 청크가 늘어도 검색 속도에 큰 영향이 없고, `INSERT/DELETE` 트리거로 `chunks` 테이블과 자동 동기화되어 관리 부담도 없다.

### 2. 하이브리드 RRF 점수 (검색 품질)

```
RRF_score = (1-weight) × 1/(rank_fts + 60) + weight × 1/(rank_vec + 60)
```

RRF는 **절대 점수가 아닌 상대 순위** 기반 알고리즘이다. 청크 수가 늘어도 상위 결과의 품질 자체는 저하되지 않는다.

### 3. 증분 인덱싱 비용

```rust
// SHA-256 해시 비교 → 변경된 파일만 재인덱싱
let new_hash = indexer::compute_hash(&content);
if existing_hash == new_hash { report.unchanged += 1; continue; }
```

문서를 많이 추가해도 **기존 파일은 재인덱싱하지 않는다.** 인덱싱 비용이 전체 문서 수에 비례하지 않고 신규/변경 파일 수에만 비례한다.

---

## 주의가 필요한 부분

### 1. 벡터 KNN 검색 — 가장 큰 잠재적 병목

```sql
WHERE v.embedding MATCH ?1 AND k = ?2
ORDER BY v.distance
```

`sqlite-vec`는 현재 **선형 스캔(full-scan KNN)** 방식으로, ANN(Approximate Nearest Neighbor) 인덱스가 없다. 청크 수에 비례해 검색 속도가 선형으로 느려진다.

| 청크 수 | 예상 영향 |
|---------|----------|
| ~5,000개 | 체감 없음 ✅ |
| ~50,000개 | 벡터 검색 지연 시작 가능 ⚠️ |
| ~100,000개+ | 유의미한 성능 저하 🔴 |

> 768차원 벡터 기준 청크당 3,072 bytes. 50,000개 = 약 150MB의 벡터 데이터.

### 2. 과다 패치(Over-fetch) 문제 — 더 빨리 체감될 수 있음

sqlite-vec는 SQL WHERE 절로 프로젝트/태그 필터를 수행할 수 없어 Rust 레이어에서 후처리한다:

```rust
let fetch_limit = if tag_id_set.is_some() {
    limit * 5    // 태그 필터: 5배 오버패치
} else if project_id.is_some() {
    limit * 3    // 프로젝트 필터: 3배 오버패치
} else {
    limit
};
```

태그 필터를 사용할 때 KNN을 **최대 5배**로 오버패치한 뒤 후처리하므로, 청크가 많아질수록 이 비용이 빠르게 커진다.

### 3. 검색 노이즈 증가

청크가 많아지면 `limit=20` 안에 관련 없는 청크가 섞일 가능성이 높아진다. 단, **인기도 리랭킹**이 이를 일부 보완한다:

```rust
// view_count 부스트 (로그 함수로 과도한 가중치 방지)
min(ln(vc + 1) × 0.03, 0.15)
// backlink_count 부스트
min(bl × 0.02, 0.20)
```

자주 읽히고 많이 참조되는 문서는 자동으로 상위에 올라오는 구조다. 인기도 리랭킹은 `use_popularity=true`일 때만 동작하며, 프로젝트 내 검색에서는 기본값이 `true`다.

---

## 규모별 진단

```
현재 규모 (수십~수백 문서)
    → 아무 문제 없음 ✅

수천 문서 수준
    → FTS5 / 하이브리드 품질: 문제 없음 ✅
    → 벡터 검색: 체감 가능한 지연 시작 ⚠️
    → 태그 필터 오버패치: 주의 필요 ⚠️

수만 문서 이상
    → ANN 인덱스 도입 검토 필요 🔴
    → 현재 sqlite-vec 아키텍처의 설계 한계
```

---

## 대응 전략

| 방법 | 효과 | 설정 위치 |
|------|------|-----------|
| `chunk_size` 키우기 (512 → 1024) | 청크 수 자체를 줄임 | `guides/configuration.md` → `indexer.chunk_size` |
| `exclude_patterns` 활용 | 불필요한 파일 인덱싱 방지 | `guides/configuration.md` → `indexer.exclude_patterns` |
| 프로젝트 분리 유지 | 검색 범위를 좁혀 오버패치 비용 절감 | 볼트 단위 프로젝트 관리 |
| 태그 체계 정리 | 태그 필터 정확도 향상 → 오버패치 낭비 감소 | 문서 frontmatter |
| ANN 인덱스 도입 (장기) | 벡터 검색 O(n) → O(log n) | sqlite-vec HNSW 지원 대기 |

---

## 관련 문서

- [[architecture/search-system-deep-dive|검색 시스템 상세 분석]]
- [[architecture/search-system|검색 시스템 개요]]
- [[architecture/database-schema|데이터베이스 스키마]]
- [[guides/configuration|설정 가이드]]
