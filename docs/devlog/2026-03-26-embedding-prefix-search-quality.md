---
title: "임베딩 컨텍스트 Prefix 도입으로 Hub Vector 제거"
aliases:
  - embedding-prefix-search-quality
  - 임베딩 컨텍스트 prefix
  - hub-vector-fix
  - 허브 벡터 제거
  - embedding-context-prefix
created: "2026-03-26"
updated: "2026-03-26"
tags:
  - devlog
  - search
  - embedding
  - bugfix
  - vector
---

<!-- docsmith: auto-generated 2026-03-26 -->

# 임베딩 컨텍스트 Prefix 도입으로 Hub Vector 제거

## 배경

Ollama를 활성화하고 전체 재인덱싱을 실행한 후 벤치마크를 돌리는 과정에서 새로운 hub vector 문제가 발견됐다. `mcp-tools.md`의 청크 중 "등록된 모든 프로젝트(볼트) 목록."이라는 단문 청크가 **모든 쿼리에서 rank 1**을 점령했다.

이는 ADR-009에서 micro-chunk 임베딩 스킵으로 해결한 것과 유사하지만, 단어 수 자체는 충분한(4단어 이상) 청크가 내용상 중립적이어서 벡터 공간 중앙에 위치하는 경우였다. 최소 단어 수를 높이는 것으로는 근본 해결이 안 된다는 판단 아래 **임베딩 텍스트에 프로젝트명과 태그를 prefix로 추가**하는 방식을 채택했다.

## 작업 내용

### 1. 벤치마크 실행 및 문제 확인

Ollama 활성화 후 6개 볼트 전체 재인덱싱 완료. 벤치마크 결과:

| 쿼리 | 결과 |
|------|------|
| "설정 가이드" | configuration.md rank 4 (기대: rank 1) |
| "의미 유사도 검색 원리" | mcp-tools.md rank 1, search-system.md 미등장 |
| "아키텍처" | benchmark 문서 rank 1 |

`mcp-tools.md`의 단문 청크가 hub vector가 된 것이 직접 원인이었다.

### 2. 임베딩 prefix 추가 (`crates/core/src/index_engine.rs`)

`build_embed_text()` 함수에 `project_name: &str`, `tags: &[String]` 파라미터를 추가하고 모든 청크에 프로젝트/태그 컨텍스트를 앞에 붙였다.

변경 전후 비교:

```
# 이전 (일반 청크)
"{content}"

# 이후 (일반 청크)
"프로젝트: {project_name}\n태그: {tag1, tag2}\n{content}"

# 이전 (chunk_0 — 제목 청크)
"제목: {title}\n별칭: {aliases}\n{content}"

# 이후 (chunk_0 — 제목 청크)
"프로젝트: {project_name}\n태그: {tags}\n제목: {title}\n별칭: {aliases}\n{content}"
```

`index_single_file()` 시그니처에도 `project_name: &str`을 추가하고 호출부에서 `&proj.name`을 전달하도록 수정. 관련 테스트 12개 전체 업데이트 후 통과 확인.

### 3. search-system.md alias 보강

"의미 유사도 검색 원리" 쿼리에 search-system.md가 상위에 올라오지 않는 문제를 alias 보강으로 해결했다.

추가한 aliases:
- `의미 유사도 검색`
- `벡터 검색 원리`
- `시맨틱 검색`

이후 동일 쿼리에서 search-system.md rank 1 달성.

### 4. librarian-legacy.md frontmatter 추가

tags가 없는 `librarian-legacy.md`가 prefix 추가 후에도 project명만 앵커링되어 새로운 hub vector로 등장했다. `librarian`, `agent`, `design`, `context` 태그를 추가해 앵커링 강화.

### 5. 전체 6개 볼트 재인덱싱

임베딩 prefix 변경 후 모든 볼트 재인덱싱 완료:
- Obsidian Nexus Docs
- brain
- Obsidian Vault
- test-vault
- xpert-da-web
- xpert-na-web

## 결과

| 쿼리 | 시작 | 최종 |
|------|------|------|
| "설정 가이드" | configuration.md rank 4 | rank 1 |
| "의미 유사도 검색 원리" | 미등장 | search-system.md rank 1 |
| "아키텍처" | benchmark 문서 rank 1 | architecture/database-schema.md rank 1 |

## 교훈

- **단어 수 기반 필터링만으로는 hub vector를 완전히 차단할 수 없다.** 단어 수가 충분해도 내용이 중립적이면 hub가 된다.
- **tags 없는 문서는 prefix 효과가 제한적이다.** 프로젝트명만 붙으면 여전히 공간 중앙 근처에 위치할 수 있다. tags 보강이 필수 보완책이다.
- **임베딩 변경은 전체 재인덱싱을 요구한다.** 변경 전후 청크가 혼재하면 검색 품질이 불균일해지므로 즉시 전체 재인덱싱해야 한다.

## 관련 문서

- [[ADR-010: 임베딩 컨텍스트 Prefix — 프로젝트명+태그로 Hub Vector 차단]]
- [[ADR-009: 검색 품질 개선 — Micro-chunk 임베딩 스킵 & Title Fallback]]
- [[search-system]]
