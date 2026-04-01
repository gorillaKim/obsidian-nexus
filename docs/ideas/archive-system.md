---
title: "아이디어: 문서 생명주기 관리 시스템 (Freshness + Archive + Health Report)"
aliases:
  - archive-system
  - 아카이빙
  - freshness
  - 신선도
  - document-health
  - 문서건강도
tags:
  - ideas
  - archive
  - freshness
  - search
  - mcp
  - health-check
  - report
  - desktop
created: "2026-04-01"
updated: "2026-04-01"
status: draft
---

# 아이디어: 문서 생명주기 관리 시스템

> **배경**: 볼트에 오래되고 참조되지 않는 문서가 쌓이면 검색 품질이 저하된다. 신선도(Freshness)를 자동 평가하고, 낡은 문서를 아카이빙하여 검색 노이즈를 줄이면서도 히스토리 열람을 보장하고자 한다.
> 아카이빙을 넘어 문서 건강도 진단, 자동 연결 제안, 정기 리포트까지 문서 생명주기 전체를 커버한다.

```
생성 → 활용 → 노후화 감지 → 정리/아카이브
 │       │         │              │
 ▼       ▼         ▼              ▼
품질검증  연결관리   건강도진단      아카이빙
```

---

## 핵심 설계 원칙

### "Soft Archive + 선택적 폴더 이동"

```
[에이전트 워크플로]

1. nexus_get_stale_documents()  → 신선도 낮은 문서 목록
2. nexus_get_documents(paths)   → 내용 읽기
3. 에이전트가 요약본 생성        → LLM 요약
4. nexus_archive_documents()    → 아카이브 처리 + 요약 저장
```

- 에이전트가 사용자와 대화하며 stale 문서를 리뷰하고, 요약 후 아카이브하는 흐름
- 아카이브된 문서는 기본 검색에서 제외되지만, 명시적 요청 시 조회 가능

---

## 아카이빙 방식 결정: 압축 vs 검색 제외

### 결론: **단계별 아카이브 (Soft → Hard)** 채택

일상적으로는 Soft Archive로 검색에서만 제외하되, 정기 리포트를 통해 Soft Archive 상태가 오래된 문서를 Hard Archive(압축/삭제)로 전환할 수 있도록 한다.

### 3단계 아카이브

```
[Level 1] Soft Archive — 검색 제외
  - documents.archived_at 컬럼 추가
  - 기본 검색에서 제외, 명시적 요청 시 검색 가능
  - 파일은 원래 위치에 그대로 유지
  - Obsidian에서 직접 열람 가능
  - 복원: DB 플래그 토글 한 줄

[Level 2] Hard Archive — 압축
  - Soft Archive 상태에서 일정 기간 경과 후 (예: 90일)
  - 문서를 .tar.gz로 압축하여 _archive/ 폴더에 보관
  - 원본 파일 삭제, DB에 archive_path 기록
  - 복원: 압축 해제 + 재인덱싱
  - 정기 리포트에서 사용자 승인 후 진행

[Level 3] Hard Archive — 삭제
  - 압축본도 더 이상 불필요할 때
  - 파일 완전 삭제, DB에서 purged_at 기록
  - 요약본(archive_summary)만 영구 보존
  - 복원 불가 — 실행 전 2차 확인 필수
```

### 단계별 비교

| 관점 | Soft Archive | Hard (압축) | Hard (삭제) |
|------|-------------|-------------|-------------|
| **검색 노출** | ❌ 기본 제외 | ❌ 완전 제외 | ❌ 완전 제외 |
| **Obsidian 열람** | ✅ 바로 가능 | ⚠️ 압축 해제 필요 | ❌ 불가 |
| **백링크 무결성** | ✅ 유지 | ❌ 링크 깨짐 | ❌ 링크 깨짐 |
| **디스크 공간** | 변화 없음 | 절약 (압축) | 최대 절약 |
| **복원 난이도** | 즉시 | 중간 | 불가 |
| **요약본** | 보존 | 보존 | 보존 |
| **적합한 경우** | 최근 아카이브 | 장기 미참조 | 완전 폐기 |

---

## 신선도 평가 (Freshness Scoring)

### 공식

```
freshness_score = w1 × recency + w2 × activity + w3 × connectivity
```

| 요소 | 산출 방식 | 가중치 (제안) |
|------|----------|-------------|
| **recency** (최근성) | `1 - (days_since_modified / max_days)` | 0.5 |
| **activity** (활동성) | `ln(view_count_90d + 1) / ln(max_views + 1)` | 0.3 |
| **connectivity** (연결성) | `ln(backlink_count + 1) / ln(max_backlinks + 1)` | 0.2 |

- 각 요소는 0~1로 정규화
- **threshold** 기본값: 0.2 (이하이면 "stale")
- 기존 DB에 `last_modified`, `document_views`, `wiki_links` 데이터 모두 존재 → 추가 수집 불필요

### 데이터 소스 매핑

| 요소 | 기존 테이블 | 쿼리 |
|------|------------|------|
| recency | `documents.last_modified` | `julianday('now') - julianday(last_modified)` |
| activity | `document_views` | `COUNT(*) WHERE viewed_at > date('now', '-90 days')` |
| connectivity | `wiki_links` | `COUNT(*) WHERE target_doc_id = ?` |

---

## DB 스키마 변경

### Migration V7: Archive Support

```sql
ALTER TABLE documents ADD COLUMN archived_at DATETIME DEFAULT NULL;
ALTER TABLE documents ADD COLUMN archive_level TEXT DEFAULT NULL;  -- 'soft' | 'hard_compressed' | 'hard_deleted'
ALTER TABLE documents ADD COLUMN archive_summary TEXT DEFAULT NULL;
ALTER TABLE documents ADD COLUMN archive_path TEXT DEFAULT NULL;   -- 압축 파일 경로 (hard archive 시)
ALTER TABLE documents ADD COLUMN original_path TEXT DEFAULT NULL;
ALTER TABLE documents ADD COLUMN purged_at DATETIME DEFAULT NULL;  -- 완전 삭제 시점

CREATE INDEX idx_documents_archived ON documents(archived_at);
CREATE INDEX idx_documents_archive_level ON documents(archive_level);
```

| 컬럼 | 용도 |
|------|------|
| `archived_at` | NULL = 활성, 값 있음 = 아카이브됨 |
| `archive_level` | `soft` / `hard_compressed` / `hard_deleted` |
| `archive_summary` | 에이전트가 생성한 요약본 (삭제 후에도 영구 보존) |
| `archive_path` | 압축 파일 경로 (예: `_archive/2026-04/batch-001.tar.gz`) |
| `original_path` | 원본 파일 경로 (복원 시 사용) |
| `purged_at` | 완전 삭제 시점 (hard_deleted만 해당) |

---

## 새 MCP 도구 설계

### `nexus_get_stale_documents`

> 신선도가 낮은 문서 목록 반환

```json
{
  "parameters": {
    "project": "string (optional)",
    "threshold": "number (0.0-1.0, default: 0.2)",
    "max_days": "number (default: 180)",
    "limit": "number (default: 20)",
    "sort_by": "freshness_asc | last_modified_asc"
  },
  "response": {
    "results": [
      {
        "document_id": "...",
        "file_path": "notes/old-design.md",
        "title": "Old Design Doc",
        "freshness_score": 0.12,
        "last_modified": "2025-06-15T...",
        "view_count_90d": 0,
        "backlink_count": 1,
        "tags": ["design"],
        "score_breakdown": {
          "recency": 0.05,
          "activity": 0.0,
          "connectivity": 0.31
        }
      }
    ]
  }
}
```

### `nexus_archive_documents`

> 문서를 아카이브 처리 (Soft / Hard)

```json
{
  "parameters": {
    "project": "string",
    "paths": ["string[] — 아카이브할 문서 경로 목록"],
    "level": "soft | hard_compress | hard_delete (default: soft)",
    "summaries": "object (optional) — { path: summary } 매핑",
    "confirm_hard": "boolean — hard 레벨 시 필수 true (안전장치)"
  },
  "response": {
    "archived": [
      {
        "path": "notes/old-design.md",
        "level": "soft",
        "archive_path": null
      }
    ],
    "failed": [{"path": "...", "reason": "..."}],
    "warnings": [
      {
        "path": "notes/old-design.md",
        "type": "has_backlinks",
        "detail": "3개 문서에서 참조 중"
      }
    ]
  }
}
```

> **안전장치**: `hard_compress` / `hard_delete` 시 `confirm_hard: true` 미전달이면 거부.
> `hard_delete` 시 백링크가 있으면 warnings에 포함하고 사용자 2차 확인 유도.

### `nexus_unarchive_documents`

> 아카이브 해제 (복원)

```json
{
  "parameters": {
    "project": "string",
    "paths": ["string[]"]
  },
  "response": {
    "restored": ["성공한 경로"],
    "failed": [
      {
        "path": "...",
        "reason": "hard_deleted — 복원 불가, 요약본만 존재"
      }
    ]
  }
}
```

> `soft` → 즉시 복원. `hard_compressed` → 압축 해제 + 재인덱싱. `hard_deleted` → 복원 불가.

### `nexus_list_archived`

> 아카이브된 문서 목록 조회

```json
{
  "parameters": {
    "project": "string (optional)",
    "level": "soft | hard_compressed | hard_deleted | all (default: all)",
    "limit": "number (default: 50)"
  },
  "response": {
    "results": [
      {
        "file_path": "notes/old-design.md",
        "title": "Old Design Doc",
        "level": "soft",
        "archived_at": "2026-03-01T...",
        "summary": "JWT 인증 설계 초안. v2로 대체됨.",
        "days_in_archive": 31
      }
    ]
  }
}
```

---

## 기존 검색 파이프라인 수정

### `search.rs` 쿼리 필터 추가

```sql
-- 기본 검색: 아카이브 제외
WHERE d.archived_at IS NULL

-- 아카이브 포함 검색
WHERE 1=1  -- include_archived: true

-- 아카이브만 검색
WHERE d.archived_at IS NOT NULL  -- archived_only: true
```

### `nexus_search` 파라미터 추가

```json
{
  "include_archived": "boolean (default: false)",
  "archived_only": "boolean (default: false)"
}
```

---

## 에이전트 워크플로 시나리오

```
에이전트                          nexus MCP 도구
  │                                    │
  ├─ "오래된 문서 정리해줘"             │
  │                                    │
  ├──nexus_get_stale_documents()──────►│
  │◄── stale 문서 10개 반환 ───────────┤
  │                                    │
  ├─ 사용자에게 목록 보여주기           │
  ├─ "이 중 어떤 것을 아카이브할까요?"  │
  │                                    │
  ├─ (사용자 승인)                      │
  │                                    │
  ├──nexus_get_documents(paths)───────►│  ← 내용 읽기
  │◄── 문서 내용 반환 ────────────────┤
  │                                    │
  ├─ 에이전트가 각 문서 요약 생성       │
  │                                    │
  ├──nexus_archive_documents(         │
  │    paths, summaries)──────────────►│  ← 아카이브 + 요약 저장
  │◄── 완료 ──────────────────────────┤
  │                                    │
  ├─ "5개 문서를 아카이브했습니다.      │
  │    요약본이 저장되었습니다."        │
```

---

## 확장 로드맵

### Phase 1: 신선도 + 아카이빙 (위 섹션)

위에서 설계한 내용. 핵심 도구:
- `nexus_get_stale_documents` — 신선도 평가
- `nexus_archive_documents` — Soft/Hard 아카이브
- `nexus_unarchive_documents` — 복원
- `nexus_list_archived` — 아카이브 목록
- `nexus_search` 필터 확장 — `include_archived`, `archived_only`

---

### Phase 2: 문서 건강도 진단 (Health Check)

신선도는 "이 문서가 오래됐는가"만 본다. 건강도 진단은 **볼트 전체의 구조적 문제**를 감지한다.

#### `nexus_health_check`

> 볼트의 구조적 문제를 종합 진단

```json
{
  "parameters": {
    "project": "string",
    "scope": "full | orphan | broken_links | tag_hygiene | oversized (default: full)"
  },
  "response": {
    "summary": {
      "total_documents": 142,
      "healthy": 118,
      "issues_found": 24
    },
    "orphan_documents": [
      {
        "file_path": "notes/random-thought.md",
        "title": "Random Thought",
        "backlink_count": 0,
        "view_count": 0,
        "last_modified": "2025-11-20T...",
        "suggestion": "link_or_archive"
      }
    ],
    "broken_links": [
      {
        "source_path": "architecture/auth.md",
        "target": "[[auth-v2-design]]",
        "suggestion": "create_or_fix"
      }
    ],
    "tag_issues": {
      "similar_groups": [
        ["deploy", "배포", "deployment"]
      ],
      "rare_tags": [
        { "tag": "misc", "count": 1 }
      ],
      "untagged_documents": ["notes/quick-memo.md"]
    },
    "oversized_documents": [
      {
        "file_path": "guides/mega-doc.md",
        "chunk_count": 68,
        "suggestion": "split"
      }
    ]
  }
}
```

#### 감지 항목 및 데이터 소스

| 진단 항목 | 감지 방법 | 기존 데이터 |
|----------|----------|------------|
| **고아 문서** | 백링크 0 + 조회수 0 | `wiki_links` + `document_views` ✅ |
| **깨진 링크** | `target_doc_id IS NULL`인 wiki_links | `wiki_links` ✅ |
| **태그 혼용** | 레벤슈타인 거리 ≤ 2 또는 번역 매칭 | `tags` ✅ |
| **희귀 태그** | 사용 횟수 1회인 태그 | `document_tags` ✅ |
| **무태그 문서** | `document_tags`에 row 없음 | `document_tags` ✅ |
| **비대 문서** | 청크 수 > 50 (약 25,000자 이상) | `chunks` ✅ |
| **고립 클러스터** | 그래프 컴포넌트 분석 (BFS) | `wiki_links` ✅ |

> **핵심**: 추가 데이터 수집 없이 기존 DB만으로 전부 가능.

---

### Phase 3: 자동 연결 제안 (Link Suggestion)

새 문서가 인덱싱될 때, 관련되었지만 아직 연결되지 않은 문서를 자동 추천한다.

#### `nexus_suggest_links`

> 현재 문서와 연결하면 좋을 미연결 문서 추천

```json
{
  "parameters": {
    "project": "string",
    "path": "string",
    "limit": "number (default: 5)"
  },
  "response": {
    "suggestions": [
      {
        "file_path": "architecture/search-system.md",
        "title": "검색 시스템 상세 분석",
        "reason": "vector_similarity",
        "similarity_score": 0.82,
        "common_tags": ["architecture", "search"],
        "already_linked": false
      }
    ]
  }
}
```

#### 구현 전략

기존 `nexus_find_related` (RRF 기반 관련 문서 탐색)에 **미연결 필터**를 얹으면 된다:

```
nexus_find_related(path) 결과
  → wiki_links에서 이미 연결된 문서 제거
  → 남은 것 = "연결하면 좋을 문서"
```

#### 트리거 시점

- **수동**: 에이전트 또는 사용자가 명시적 호출
- **인덱싱 후 자동** (옵션): 새 문서 인덱싱 완료 시 백그라운드 실행, 결과를 리포트에 축적

---

### Phase 4: 정기 리포트 (★ 핵심)

Phase 1~3의 결과를 종합하여 **주기적으로 문서 건강 리포트를 생성**하고, 데스크톱 앱에서 **클릭 기반으로 즉시 조치**할 수 있게 한다.

#### 리포트 생성 MCP 도구

##### `nexus_generate_report`

> 문서 건강 리포트 생성 (에이전트/앱/스케줄러가 호출)

```json
{
  "parameters": {
    "project": "string",
    "include": ["freshness", "health", "links", "hard_archive_candidates"]
  },
  "response": {
    "report_id": "rpt-2026-04-01-001",
    "generated_at": "2026-04-01T09:00:00Z",
    "project": "my-vault",
    "summary": {
      "total_documents": 142,
      "stale_documents": 8,
      "orphan_documents": 3,
      "broken_links": 2,
      "tag_issues": 5,
      "soft_archived": 12,
      "hard_archive_candidates": 4,
      "link_suggestions": 6
    },
    "sections": {
      "freshness": {
        "stale": [
          {
            "document_id": "...",
            "file_path": "notes/old-design.md",
            "title": "Old Design Doc",
            "freshness_score": 0.08,
            "last_modified": "2025-06-15T...",
            "view_count_90d": 0,
            "backlink_count": 1,
            "recommended_action": "soft_archive"
          }
        ]
      },
      "health": {
        "orphans": [
          {
            "file_path": "notes/random.md",
            "title": "Random",
            "recommended_action": "link_or_archive"
          }
        ],
        "broken_links": [
          {
            "source_path": "architecture/auth.md",
            "target": "[[auth-v2-design]]",
            "recommended_action": "create_or_fix"
          }
        ],
        "tag_issues": {
          "similar_groups": [["deploy", "배포"]],
          "rare_tags": ["misc"]
        }
      },
      "hard_archive_candidates": {
        "description": "Soft Archive 상태로 90일 이상 경과한 문서",
        "documents": [
          {
            "file_path": "notes/deprecated-api.md",
            "title": "Deprecated API Spec",
            "archived_at": "2025-12-01T...",
            "days_in_archive": 121,
            "summary": "v1 API 사양. v2로 전면 대체됨.",
            "recommended_action": "hard_compress",
            "has_backlinks": false
          },
          {
            "file_path": "devlog/2025-05-old-bug.md",
            "title": "Old Bug Report",
            "archived_at": "2025-11-15T...",
            "days_in_archive": 137,
            "summary": "해결 완료된 버그. 재현 불가.",
            "recommended_action": "hard_delete",
            "has_backlinks": false
          }
        ]
      },
      "link_suggestions": [
        {
          "file_path": "architecture/search.md",
          "suggested_target": "architecture/indexer.md",
          "reason": "높은 벡터 유사도 + 공통 태그 3개",
          "similarity_score": 0.85
        }
      ]
    }
  }
}
```

#### 리포트 스케줄링

```toml
# config.toml 확장
[report]
enabled = true
schedule = "weekly"          # daily | weekly | monthly
day_of_week = "monday"       # weekly일 때
include = ["freshness", "health", "links", "hard_archive_candidates"]
hard_archive_threshold_days = 90   # soft archive 후 이 기간 경과 시 hard 후보
```

- **데스크톱 앱**: 앱 시작 시 마지막 리포트 날짜 확인 → 주기 도래 시 자동 생성
- **CLI**: `nexus report [--project]` 커맨드로 수동 생성
- **에이전트**: 대화 시작 시 "리포트 있음" 알림 + 요약 제공

#### DB 스키마 추가

```sql
-- Migration V8: Report History
CREATE TABLE reports (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    generated_at DATETIME NOT NULL DEFAULT (datetime('now')),
    report_data TEXT NOT NULL,  -- JSON (전체 리포트)
    actions_taken TEXT,         -- JSON (사용자가 취한 조치 기록)
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_reports_project_date ON reports(project_id, generated_at DESC);
```

---

## 데스크톱 앱: 리포트 대시보드 UI

### 리포트 탭 개요

앱에 **리포트 탭**을 추가한다. 주간 리포트를 카드 형태로 보여주고, 각 항목에 대해 **클릭으로 즉시 조치**할 수 있다.

```
┌─────────────────────────────────────────────────────────┐
│  📋 문서 건강 리포트  |  2026-04-01 (월)  |  ◀ ▶       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  📊 요약                                                │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐         │
│  │  142 │ │   8  │ │   3  │ │   2  │ │   4  │         │
│  │ 전체 │ │ 노후 │ │ 고아 │ │ 깨진 │ │ Hard │         │
│  │ 문서 │ │ 문서 │ │ 문서 │ │ 링크 │ │ 후보 │         │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘         │
│                                                         │
│  ─── 🗄️ 아카이빙 제안 (8건) ──────────────────────     │
│                                          [전체 선택]    │
│  ☐ notes/old-design.md         freshness: 0.08          │
│    "Old Design Doc"            6개월 미수정 · 조회 0    │
│    [Soft Archive] [무시]                                │
│                                                         │
│  ☐ devlog/2025-08-old-log.md   freshness: 0.05          │
│    "Old Dev Log"               9개월 미수정 · 백링크 0  │
│    [Soft Archive] [무시]                                │
│                                                         │
│  ─── 🔥 Hard Archive 후보 (4건) ─────────────────       │
│  ⚠️ Soft Archive 후 90일+ 경과 문서                     │
│                                          [전체 선택]    │
│  ☐ notes/deprecated-api.md     soft 121일 경과          │
│    "Deprecated API Spec"                                │
│    요약: v1 API 사양. v2로 전면 대체됨.                  │
│    [📦 압축] [🗑️ 삭제] [유지]                           │
│                                                         │
│  ☐ devlog/2025-05-old-bug.md   soft 137일 경과          │
│    "Old Bug Report"            ⚠️ 백링크 1개            │
│    요약: 해결 완료된 버그. 재현 불가.                     │
│    [📦 압축] [🗑️ 삭제] [유지]                           │
│                                                         │
│  ─── 🔗 연결 제안 (6건) ──────────────────────────      │
│                                                         │
│  ☐ search.md ↔ indexer.md      유사도: 0.85             │
│    "공통 태그 3개, 벡터 유사도 높음"                      │
│    [링크 추가] [무시]                                    │
│                                                         │
│  ─── 💔 깨진 링크 (2건) ─────────────────────────       │
│                                                         │
│    architecture/auth.md → [[auth-v2-design]] ❌          │
│    [문서 생성] [링크 수정] [무시]                         │
│                                                         │
├─────────────────────────────────────────────────────────┤
│           [ 선택 항목 일괄 적용 (7건) ]                  │
└─────────────────────────────────────────────────────────┘
```

### UI 인터랙션 흐름

#### 1. Soft Archive 일괄 적용

```
사용자: 노후 문서 8건 중 5건 체크박스 선택
  → [선택 항목 일괄 적용] 클릭
  → 확인 다이얼로그:
    "5개 문서를 Soft Archive합니다.
     - 기본 검색에서 제외됩니다
     - Obsidian에서는 계속 열람 가능합니다
     [확인] [취소]"
  → Tauri invoke("archive_documents", { paths, level: "soft" })
  → 완료 토스트: "✅ 5개 문서 Soft Archive 완료"
  → 리포트 항목에 ✅ 표시 + 취소선
```

#### 2. Hard Archive (압축/삭제) 개별 적용

```
사용자: "Deprecated API Spec" 옆 [📦 압축] 클릭
  → 확인 다이얼로그 (⚠️ 경고 포함):
    "이 문서를 압축 아카이브합니다.
     ━━━━━━━━━━━━━━━━━━━━━━━━
     📄 notes/deprecated-api.md
     📝 요약: v1 API 사양. v2로 전면 대체됨.
     ⚠️ 원본 파일이 삭제되고 .tar.gz로 보관됩니다
     ⚠️ Obsidian에서 직접 열 수 없습니다
     ✅ 압축 해제로 복원 가능합니다
     [압축 실행] [취소]"
  → Tauri invoke("archive_documents", { paths, level: "hard_compress", confirm_hard: true })
  → 완료 토스트: "📦 1개 문서 압축 완료 → _archive/2026-04/"
```

#### 3. Hard Delete (2차 확인)

```
사용자: "Old Bug Report" 옆 [🗑️ 삭제] 클릭
  → 1차 확인 다이얼로그:
    "⚠️ 이 문서를 완전히 삭제합니다.
     ━━━━━━━━━━━━━━━━━━━━━━━━
     📄 devlog/2025-05-old-bug.md
     📝 요약: 해결 완료된 버그. 재현 불가.
     🔗 백링크: architecture/bugfix-log.md에서 참조 중
     ❌ 이 작업은 되돌릴 수 없습니다
     ❌ 요약본만 DB에 남습니다
     [삭제 진행] [취소]"
  → 2차 확인 (백링크 있을 경우):
    "⚠️ 이 문서를 참조하는 문서가 있습니다:
     - architecture/bugfix-log.md
     해당 문서의 링크가 깨집니다. 정말 삭제하시겠습니까?
     [문서명 입력하여 확인: ________] [취소]"
  → (사용자가 문서명 "Old Bug Report" 입력)
  → Tauri invoke("archive_documents", { paths, level: "hard_delete", confirm_hard: true })
  → 완료 토스트: "🗑️ 1개 문서 삭제 완료 (요약본 보존)"
```

#### 4. Hard Archive 일괄 적용

```
사용자: Hard 후보 4건 중 선택:
  - 2건 [📦 압축] 선택
  - 1건 [🗑️ 삭제] 선택
  - 1건 [유지] 선택
  → [선택 항목 일괄 적용] 클릭
  → 종합 확인 다이얼로그:
    "━━━━━━━━━━ 일괄 적용 요약 ━━━━━━━━━━
     📦 압축: 2건
       - notes/deprecated-api.md
       - notes/old-spec.md
     🗑️ 삭제: 1건
       - devlog/2025-05-old-bug.md (⚠️ 백링크 1개)
     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
     삭제는 되돌릴 수 없습니다.
     [일괄 실행] [취소]"
```

### Tauri 커맨드 설계

```rust
// apps/desktop/src-tauri/src/lib.rs 에 추가

#[tauri::command]
async fn generate_report(project: String) -> Result<Report, String>;

#[tauri::command]
async fn get_latest_report(project: String) -> Result<Option<Report>, String>;

#[tauri::command]
async fn archive_documents(
    project: String,
    paths: Vec<String>,
    level: String,           // "soft" | "hard_compress" | "hard_delete"
    confirm_hard: bool,
    summaries: Option<HashMap<String, String>>,
) -> Result<ArchiveResult, String>;

#[tauri::command]
async fn unarchive_documents(
    project: String,
    paths: Vec<String>,
) -> Result<UnarchiveResult, String>;
```

### React 컴포넌트 구조

```
src/components/report/
├── ReportTab.tsx            — 탭 루트 (리포트 로딩 + 네비게이션)
├── ReportSummary.tsx        — 상단 요약 카드 (숫자 대시보드)
├── StaleDocumentSection.tsx — 노후 문서 목록 + 체크박스 + 액션 버튼
├── HardArchiveSection.tsx   — Hard 후보 목록 + 압축/삭제/유지 버튼
├── LinkSuggestionSection.tsx— 연결 제안 목록
├── BrokenLinkSection.tsx    — 깨진 링크 목록
├── TagIssueSection.tsx      — 태그 혼용/희귀 태그
├── BatchActionBar.tsx       — 하단 고정 바 ("선택 항목 일괄 적용")
├── ConfirmDialog.tsx        — 확인 다이얼로그 (일반/경고/2차확인)
└── ReportHistory.tsx        — 이전 리포트 목록 + ◀ ▶ 네비게이션
```

### 리포트 알림

```
[앱 시작 시]
  → 마지막 리포트 날짜 확인
  → 주기 도래 시:
    → 백그라운드로 리포트 생성
    → 앱 상단에 알림 배지 표시:
      "📋 새 리포트가 준비되었습니다 (이슈 24건) [보기]"

[사서 에이전트 대화 시작 시]
  → 미확인 리포트 있으면 자동 요약 제공:
    "📋 이번 주 문서 건강 리포트가 있습니다.
     - 노후 문서 8건 (아카이브 추천)
     - Hard Archive 후보 4건 (90일+ 경과)
     - 깨진 링크 2건
     '리포트 보여줘'라고 말씀하시면 상세 내용을 보여드릴게요."
```

---

## 구현 범위 (전체)

### Phase 1: 신선도 + 아카이빙

#### core 크레이트 (`crates/core/`)

- [ ] `migrations/V7__archive.sql` — 스키마 변경
- [ ] `freshness.rs` — 신선도 계산 모듈
- [ ] `archive.rs` — 아카이브/복원 로직 (Soft + Hard 압축/삭제)
- [ ] `search.rs` — 아카이브 필터 조건 추가

#### MCP 서버 (`crates/mcp-server/`)

- [ ] `nexus_get_stale_documents` 핸들러
- [ ] `nexus_archive_documents` 핸들러 (level 파라미터: soft/hard_compress/hard_delete)
- [ ] `nexus_unarchive_documents` 핸들러
- [ ] `nexus_list_archived` 핸들러
- [ ] `nexus_search` — `include_archived`, `archived_only` 파라미터 추가

#### CLI (`crates/cli/`)

- [ ] `nexus archive <paths> [--level soft|compress|delete]` 커맨드
- [ ] `nexus unarchive <paths>` 커맨드
- [ ] `nexus stale [--project] [--threshold]` 커맨드

### Phase 2: 건강도 진단

#### core 크레이트

- [ ] `health.rs` — 건강도 진단 모듈 (orphan, broken links, tag audit, oversized)

#### MCP 서버

- [ ] `nexus_health_check` 핸들러

#### CLI

- [ ] `nexus health [--project] [--scope full|orphan|broken|tags]` 커맨드

### Phase 3: 자동 연결 제안

#### core 크레이트

- [ ] `link_suggestion.rs` — 미연결 관련 문서 추천 (`find_related` 확장)

#### MCP 서버

- [ ] `nexus_suggest_links` 핸들러

### Phase 4: 정기 리포트 + 데스크톱 UI

#### core 크레이트

- [ ] `migrations/V8__reports.sql` — 리포트 히스토리 테이블
- [ ] `report.rs` — 리포트 생성 (Phase 1~3 결과 종합)

#### MCP 서버

- [ ] `nexus_generate_report` 핸들러

#### CLI

- [ ] `nexus report [--project]` 커맨드

#### 데스크톱 앱

- [ ] `ReportTab.tsx` — 리포트 탭 루트
- [ ] `ReportSummary.tsx` — 상단 요약 카드
- [ ] `StaleDocumentSection.tsx` — 노후 문서 + Soft Archive 체크박스
- [ ] `HardArchiveSection.tsx` — Hard 후보 + 압축/삭제/유지 버튼
- [ ] `LinkSuggestionSection.tsx` — 연결 제안
- [ ] `BrokenLinkSection.tsx` — 깨진 링크
- [ ] `BatchActionBar.tsx` — 하단 일괄 적용 바
- [ ] `ConfirmDialog.tsx` — 확인 다이얼로그 (일반/경고/2차확인)
- [ ] Tauri 커맨드: `generate_report`, `get_latest_report`, `archive_documents`, `unarchive_documents`
- [ ] 앱 시작 시 리포트 알림 배지
- [ ] 리포트 히스토리 ◀ ▶ 네비게이션

---

## 관련 문서

- [[plugin-system]] — 외부 소스 연동 시 아카이빙 정책 확장 가능
- [[graph-traversal]] — connectivity 계산에 그래프 탐색 활용
- [[archiving-strategy]] — 기존 수동 아카이빙 가이드 (frontmatter 기반 인플레이스 방식). 이 문서는 시스템 자동화 관점이고 archiving-strategy는 사용자 가이드 관점. 상호 보완 관계
- [[librarian-legacy]] — 사서 에이전트 설계. Phase 4 리포트 알림은 사서 프롬프트(`doc-maintenance.md`)에 통합

---

## 미결 사항

- [ ] 가중치 (0.5 / 0.3 / 0.2)는 config.toml로 사용자 조정 가능하게 할지?
- [ ] `_archive/` 폴더 이동 시 위키링크 자동 리라이트 필요한지?
- [ ] 아카이브 문서의 FTS5/벡터 인덱스도 제거할지, DB 필터만으로 충분한지?
- [ ] 자동 아카이브 (threshold 미만 시 자동 처리) 지원 여부
- [ ] Hard Delete 시 2차 확인: 문서명 입력 방식 vs 단순 체크박스 방식
- [ ] 리포트 스케줄: 앱 시작 시 체크 vs 백그라운드 타이머
- [ ] 압축 포맷: `.tar.gz` vs `.zip` (Obsidian 플러그인 호환 고려)
- [ ] 리포트 보관 기간: 무제한 vs 최근 N개만 유지
