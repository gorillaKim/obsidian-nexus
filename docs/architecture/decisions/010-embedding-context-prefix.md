---
title: "ADR-010: 임베딩 컨텍스트 Prefix — 프로젝트명+태그로 Hub Vector 차단"
aliases:
  - ADR-010
  - embedding-context-prefix
  - 임베딩 컨텍스트 Prefix
  - hub-vector-prevention
  - 허브 벡터 차단
created: 2026-03-26
updated: 2026-03-26
status: Accepted
tags:
  - architecture
  - decision
  - embedding
  - search
  - vector
---

<!-- docsmith: auto-generated 2026-03-26 -->

# ADR-010: 임베딩 컨텍스트 Prefix — 프로젝트명+태그로 Hub Vector 차단

## 상태

채택 (Accepted)

## 배경

Ollama 기반 벡터 검색 활성화 이후 `mcp-tools.md`의 "등록된 모든 프로젝트(볼트) 목록."이라는 청크가 모든 쿼리에서 rank 1을 점령하는 **hub vector** 문제가 발생했다.

Hub vector란 벡터 임베딩 공간에서 중앙 근처에 위치해 다수의 쿼리 벡터와 높은 코사인 유사도를 가지는 청크다. 의미상 중립적이거나 범용적인 짧은 문장이 주로 hub가 된다. ADR-009에서 단어 수 기반 micro-chunk 스킵으로 일부를 차단했으나, 단어 수가 충분한 중립 문장은 여전히 hub가 될 수 있음을 확인했다.

## 검토한 대안들

### 대안 1: min_vector_score 상향 (0.65 → 0.72)

증상을 치료하는 접근이다. hub 청크의 유사도가 새 임계값 이상이면 여전히 등장한다. 다른 hub가 생길 때마다 임계값을 반복 조정해야 하는 whack-a-mole 패턴이 된다.

**기각.**

### 대안 2: MIN_EMBED_WORD_COUNT 상향 (4 → 8)

단어 수가 아닌 내용의 중립성이 hub의 원인이므로 효과가 불확실하다. 정상적인 짧은 핵심 청크도 임베딩 대상에서 제외될 위험이 있다.

**기각.**

### 대안 3: 프로젝트명+태그 prefix 추가 (채택)

각 청크를 임베딩할 때 해당 문서의 프로젝트명과 태그를 prefix로 붙인다. hub가 되는 원인(내용의 중립성)을 직접 해소하여 청크를 특정 의미 공간으로 앵커링한다.

**채택.**

## 결정

모든 청크 임베딩 생성 시 `build_embed_text()` 함수에서 프로젝트명과 문서 태그를 prefix로 추가한다.

### 구현 형식

```
# 일반 청크 (chunk_N, N > 0)
이전: "{content}"
이후: "프로젝트: {project_name}\n태그: {tag1, tag2, ...}\n{content}"

# 제목 청크 (chunk_0)
이전: "제목: {title}\n별칭: {aliases}\n{content}"
이후: "프로젝트: {project_name}\n태그: {tags}\n제목: {title}\n별칭: {aliases}\n{content}"
```

### 변경된 시그니처

```rust
// crates/core/src/index_engine.rs
fn build_embed_text(
    chunk: &Chunk,
    project_name: &str,   // 추가
    tags: &[String],      // 추가
) -> String

fn index_single_file(
    path: &Path,
    project_name: &str,   // 추가
    // ...
)
```

## 결과

| 쿼리 | 변경 전 | 변경 후 |
|------|---------|---------|
| "설정 가이드" | configuration.md rank 4 | rank 1 |
| "의미 유사도 검색 원리" | mcp-tools.md rank 1, search-system.md 미등장 | search-system.md rank 1 |
| "아키텍처" | benchmark 문서 rank 1 | architecture/database-schema.md rank 1 |

## 트레이드오프

| 항목 | 내용 |
|------|------|
| 크로스 볼트 검색 | 볼트 간 유사도가 자연스럽게 분리된다. 의도된 격리에 가까우나 cross-vault retrieval 품질 일부 하락 가능 |
| 태그 없는 문서 | 프로젝트명만 붙어 부분 앵커링. 효과 제한적 → 문서 태그 보강 권장 |
| 재인덱싱 비용 | 임베딩 형식 변경 시 전체 재인덱싱 필요 (1회성) |

## 후속 조치

- 태그 없는 레거시 문서 frontmatter 보강 (`librarian-legacy.md` 등)
- 크로스 볼트 검색 시나리오 벤치마크 추가 (의도된 격리 vs 의도치 않은 누락 구분)

## 관련 문서

- [[ADR-009: 검색 품질 개선 — Micro-chunk 임베딩 스킵 & Title Fallback]]
- [[임베딩 컨텍스트 Prefix 도입으로 Hub Vector 제거]]
- [[search-system]]
