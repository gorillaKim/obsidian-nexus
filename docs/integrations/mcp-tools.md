---
title: MCP 도구 레퍼런스
tags:
  - mcp
  - api
  - agent
aliases:
  - MCP Tools
  - MCP Reference
---

# MCP 도구 레퍼런스

총 13개 도구 제공. AI 에이전트가 문서를 검색, 탐색, 분석하는 데 사용.

## 검색

### nexus_search

문서 검색 (하이브리드/키워드/벡터).

| 파라미터 | 타입 | 필수 | 기본값 | 설명 |
|----------|------|------|--------|------|
| query | string | O | - | 검색어 |
| project | string | - | 전체 | 프로젝트 필터 |
| limit | integer | - | 20 | 최대 결과 수 |
| mode | string | - | hybrid | hybrid, keyword, vector |
| enrich | boolean | - | true | 메타데이터 포함 |
| use_popularity | boolean | - | 프로젝트 필터 시 true | 인기도 부스트 |
| tags | string[] | - | - | 태그 필터 (예: ["rust", "api"]) |
| tag_match_all | boolean | - | false | true: AND 조건 (모든 태그 매칭), false: OR 조건 |
| rewrite_query | boolean | - | false | LLM으로 쿼리 재작성 (Ollama 필요, 모든 검색 모드 지원) |

## 문서 읽기

### nexus_get_document

문서 전문 조회. 호출 시 자동으로 view_count 증가.

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| path | string | O |

### nexus_get_section

heading 기반 섹션 부분 읽기. 토큰 절약에 유용.

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| path | string | O |
| heading | string | O |

### nexus_get_metadata

문서 메타데이터 (frontmatter, 태그, 인덱싱 상태) 조회.

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| path | string | O |

## 탐색

### nexus_list_projects

등록된 모든 프로젝트(볼트) 목록.

### nexus_list_documents

프로젝트 내 문서 목록. 태그 필터 가능.

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| tag | string | - |

## 그래프

### nexus_get_backlinks

이 문서를 참조하는 문서 목록 (역방향 링크).

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| path | string | O |

### nexus_get_links

이 문서가 참조하는 문서 목록 (정방향 링크).

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| path | string | O |

### nexus_get_cluster

앞방향+역방향 링크를 따라 depth 홉 이내 연결된 모든 문서 반환. tags, snippet 포함.

| 파라미터 | 타입 | 필수 | 기본값 |
|----------|------|------|--------|
| project | string | O | |
| path | string | O | |
| depth | integer | - | 2 (max 5) |

### nexus_find_path

두 문서 사이 최단 정방향 링크 경로 탐색 (max 6 hops). 경로 없으면 null 반환.

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| from | string | O |
| to | string | O |

### nexus_find_related

링크 거리와 태그 중복을 RRF로 합산하여 유사 문서 상위 k개 반환. signals 필드로 근거 제공.

| 파라미터 | 타입 | 필수 | 기본값 |
|----------|------|------|--------|
| project | string | O | |
| path | string | O | |
| k | integer | - | 10 |

### nexus_resolve_alias

별칭으로 문서 찾기.

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |
| alias | string | O |

## 관리

### nexus_index_project

인덱싱 트리거 (증분 또는 전체).

| 파라미터 | 타입 | 필수 | 기본값 |
|----------|------|------|--------|
| project | string | O | - |
| full | boolean | - | false |

### nexus_sync_config

볼트 루트의 `on-config.json`을 읽어 프로젝트 설정을 동기화. 이름 변경 후 호출.

| 파라미터 | 타입 | 필수 |
|----------|------|------|
| project | string | O |

## 시스템 상태

### nexus_status

시스템 건강 상태 확인. Ollama 서버, 임베딩 모델, DB, 설정 상태를 JSON으로 반환.

파라미터 없음.

**반환 예시:**
```json
{
  "ollama": {
    "running": true,
    "url": "http://localhost:11434",
    "model": "nomic-embed-text",
    "model_available": true
  },
  "database": {
    "exists": true,
    "path": "~/.nexus/nexus.db",
    "schema_version": 6,
    "project_count": 3,
    "document_count": 142
  },
  "config": {
    "exists": true,
    "path": "~/.nexus/config.toml",
    "embedding_provider": "ollama",
    "embedding_model": "nomic-embed-text",
    "embedding_dimensions": 768
  },
  "overall": "ready"
}
```

`overall`: 모든 필수 항목 정상이면 `"ready"`, 아니면 `"not_ready"` (각 항목에 `error` 필드 포함).

## 온보딩

### nexus_onboard

사서(librarian) 스킬과 서브에이전트를 대상 프로젝트에 설치. `.mcp.json`, `.claude/agents/librarian.md`, `.claude/skills/librarian/SKILL.md`를 생성한다.

| 파라미터 | 타입 | 필수 | 기본값 | 설명 |
|----------|------|------|--------|------|
| project_path | string | - | 현재 디렉토리 | 대상 프로젝트 루트 경로 |
| force | boolean | - | false | 기존 파일 덮어쓰기 |

**사용 예시:**
```json
nexus_onboard({ "project_path": "/path/to/my-project" })
```

생성 후 Claude Code 세션 재시작 필요.

## 에이전트 활용 패턴

```
0. nexus_onboard → 새 프로젝트에 사서 스킬/에이전트 설치
1. nexus_status → 시스템 상태 확인 (Ollama, DB 등)
2. nexus_list_projects → 볼트 파악
3. nexus_search("키워드") → 관련 문서 검색
4. nexus_get_section(path, heading) → 필요한 섹션만 읽기
5. nexus_get_cluster(path, depth=2) → 멀티홉 그래프 탐색 (2-hop 이내 모든 연결 문서)
6. nexus_resolve_alias("별칭") → 별칭으로 문서 접근
7. nexus_sync_config(project) → on-config.json 변경 후 동기화
```

## 관련 문서

- [[02-검색-시스템]]
- [[05-설정-가이드]]
