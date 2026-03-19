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

총 11개 도구 제공. AI 에이전트가 문서를 검색, 탐색, 분석하는 데 사용.

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
| tags | string[] | - | - | 태그 필터 (OR 조건, 예: ["rust", "api"]) |

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

## 에이전트 활용 패턴

```
1. nexus_list_projects → 볼트 파악
2. nexus_search("키워드") → 관련 문서 검색
3. nexus_get_section(path, heading) → 필요한 섹션만 읽기
4. nexus_get_backlinks(path) → 관련 문서 그래프 탐색
5. nexus_resolve_alias("별칭") → 별칭으로 문서 접근
6. nexus_sync_config(project) → on-config.json 변경 후 동기화
```

## 관련 문서

- [[02-검색-시스템]]
- [[05-설정-가이드]]
