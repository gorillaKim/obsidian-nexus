---
title: 관계 그래프 쿼리 도구
tags: [idea, roadmap, graph, mcp]
status: draft
date: 2026-03-27
---

# 아이디어: 관계 그래프 쿼리 도구

## 배경

현재 `nexus_get_backlinks` / `nexus_get_links`는 명시적 위키링크 1-hop만 제공한다.
에이전트가 "이 주제 전체 컨텍스트"를 파악하려면 여러 번 호출을 반복해야 하고,
의미적으로 연관된 문서(링크 없이 내용이 유사한 것)는 찾을 수 없다.

`wiki_links` 테이블(`source_doc_id → target_doc_id`)이 이미 존재하므로
추가 스키마 없이 구현 가능하다.

## 제안 도구

### `nexus_get_cluster(path, depth)`

위키링크를 따라 depth 레벨까지 연결된 문서 집합 반환 (앞방향 + 역방향 모두 포함).

**반환 예시:**
```json
[
  { "file_path": "auth/session.md", "title": "세션 관리", "distance": 1 },
  { "file_path": "auth/token.md",   "title": "토큰 갱신", "distance": 2 }
]
```

**구현:** SQLite 재귀 CTE 한 방으로 처리 — Rust 루프 불필요.

```sql
WITH RECURSIVE cluster(doc_id, hops) AS (
  SELECT d.id, 0 FROM documents d
  WHERE d.project_id = ?1 AND d.file_path = ?2

  UNION

  -- 앞방향 확장 (source → target)
  SELECT wl.target_doc_id, c.hops + 1
  FROM wiki_links wl
  JOIN cluster c ON wl.source_doc_id = c.doc_id
  WHERE c.hops < ?3 AND wl.target_doc_id IS NOT NULL

  UNION

  -- 역방향 확장 (backlink 포함)
  SELECT wl.source_doc_id, c.hops + 1
  FROM wiki_links wl
  JOIN cluster c ON wl.target_doc_id = c.doc_id
  WHERE c.hops < ?3
)
SELECT d.file_path, d.title, MIN(c.hops) as distance
FROM cluster c
JOIN documents d ON c.doc_id = d.id
WHERE d.file_path != ?2
GROUP BY d.id
ORDER BY distance, d.file_path
```

---

### `nexus_find_path(from, to)`

두 문서 사이 최단 위키링크 경로 탐색. 경로가 없으면 `null` 반환.

**반환 예시:**
```json
{ "path": ["auth.md", "middleware.md", "session-management.md"], "hops": 2 }
```

**구현:** 재귀 CTE + `json_array` / `json_insert` (SQLite 3.38+ 기본 지원).
무한루프 방지를 위해 최대 hops 6 제한.

```sql
WITH RECURSIVE path_search(doc_id, route, hops) AS (
  SELECT d.id, json_array(d.file_path), 0
  FROM documents d WHERE d.project_id = ?1 AND d.file_path = ?2

  UNION

  SELECT wl.target_doc_id,
         json_insert(ps.route, '$[#]', d2.file_path),
         ps.hops + 1
  FROM wiki_links wl
  JOIN path_search ps ON wl.source_doc_id = ps.doc_id
  JOIN documents d2 ON wl.target_doc_id = d2.id
  WHERE ps.hops < 6
    AND wl.target_doc_id IS NOT NULL
)
SELECT route, hops FROM path_search
JOIN documents d ON doc_id = d.id
WHERE d.file_path = ?3
ORDER BY hops LIMIT 1
```

---

### `nexus_find_related(path, k)`

벡터 유사도 + 공통 태그 + 링크 거리를 RRF로 복합 점수 계산 후 상위 k개 반환.

**반환 예시:**
```json
[
  { "file_path": "auth/session-management.md", "score": 0.91, "signals": ["vector", "tag"] },
  { "file_path": "auth/token-refresh.md",      "score": 0.83, "signals": ["backlink"] }
]
```

**구현:** Rust에서 3개 점수를 합산 (순수 SQL보다 명확).

```rust
// 1) 링크 거리: get_cluster(depth=2) 결과에서 distance 역산
// 2) 벡터 유사도: 기존 KNN search 재사용 (이미 구현됨)
// 3) 공통 태그: tags JSONB overlap 쿼리

// RRF 합산 (k=60 상수)
score = 1/(60 + rank_link) + 1/(60 + rank_vector) + 1/(60 + rank_tag)
```

Ollama 미연결 시 링크 + 태그 점수만으로 graceful fallback.

---

## 구현 위치

| 파일 | 작업 |
|------|------|
| `crates/core/src/search.rs` | `get_cluster()`, `find_path()`, `find_related()` 함수 추가 |
| `crates/mcp-server/src/main.rs` | 핸들러 3개 추가 + match arm 등록 |

새 테이블 / 마이그레이션 불필요. 기존 `tool_get_backlinks` 패턴 복사해서 사용.

## 구현 난이도

| 도구 | 난이도 | 비고 |
|------|--------|------|
| `nexus_get_cluster` | 하 | 재귀 CTE만 작성하면 됨 |
| `nexus_find_path` | 하 | 동일 패턴, json_array 추가 |
| `nexus_find_related` | 중 | 기존 KNN 재사용, RRF 합산 로직 |

## 우선순위

`nexus_get_cluster` → `nexus_find_path` → `nexus_find_related` 순으로 구현 권장.

---

## 에이전트 실사용 검토 (2026-03-27)

### 현재 구조의 병목

에이전트가 "이 주제의 전체 컨텍스트"를 파악할 때 현재 흐름:

1. `nexus_search` → 문서 1개 발견
2. `nexus_get_links` → 연결 문서 목록
3. `nexus_get_backlinks` → 역방향 연결
4. 각 연결 문서에 대해 2~3번 반복...

**2-hop 탐색만으로도 10회 이상의 도구 호출이 필요.** 순차 대기로 인해 응답 지연 및
컨텍스트 토큰 소비가 누적된다. 링크 없이 의미적으로 연관된 문서는 아예 탐색 불가.

### 도구별 에이전트 가치 평가

| 도구 | 에이전트 가치 | 이유 |
|------|------------|------|
| `nexus_get_cluster` | ⭐⭐⭐⭐⭐ | 멀티-홉 탐색을 1회 호출로 대체. 즉각적 효과 |
| `nexus_find_related` | ⭐⭐⭐⭐ | 링크 없는 의미적 연관 문서 발견 가능. 사용자가 모르는 지식도 선제 제안 가능 |
| `nexus_find_path` | ⭐⭐⭐ | 두 개념 간 연결 경로 설명 시 유용. 현재는 불가능한 기능 |

### 추가 제안: `nexus_get_cluster` 응답에 snippet/tags 포함

현재 반환 스펙: `{ file_path, title, distance }`

권장 추가 필드:
```json
{ "file_path": "auth/session.md", "title": "세션 관리", "distance": 1,
  "tags": ["auth", "session"], "snippet": "JWT 기반 세션 관리 정책..." }
```

`tags` + `snippet`이 포함되면 에이전트가 각 문서를 `nexus_get_document`로
다시 열지 않아도 되어, 도구 호출 횟수를 추가로 절감할 수 있다.
