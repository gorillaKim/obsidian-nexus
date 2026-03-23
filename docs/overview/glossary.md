---
title: 용어 사전
tags:
  - overview
  - glossary
aliases:
  - glossary
  - terminology
  - 용어집
  - 용어정의
created: 2026-03-23
updated: 2026-03-23
---

<!-- docsmith: auto-generated 2026-03-23 -->

# 용어 사전

Obsidian Nexus 프로젝트에서 사용하는 도메인 용어를 정의합니다.
개발자와 사용자 모두를 대상으로 합니다.

---

## A–Z

### FTS5

SQLite에 내장된 전문 검색(Full-Text Search) 엔진. `unicode61` 토크나이저를 사용하여 한국어·영어 혼합 텍스트를 지원한다. Obsidian Nexus에서는 키워드 검색 모드의 기반으로 사용한다.

```sql
SELECT ... FROM chunks_fts WHERE chunks_fts MATCH ?1
```

→ 관련: [[02-검색-시스템]]

---

### MCP (Model Context Protocol)

AI 에이전트가 외부 도구를 호출하기 위한 JSON-RPC 기반 프로토콜. Obsidian Nexus는 `crates/mcp-server`를 통해 MCP 서버를 제공하며, stdin/stdout으로 통신한다. 모든 도구 이름에는 `nexus_` 접두사를 사용한다.

→ 관련: [[03-MCP-도구-레퍼런스]], [[08-서브에이전트-MCP-설정-가이드]]

---

### nomic-embed-text

Ollama에서 실행하는 오픈소스 임베딩 모델. 768차원 벡터를 생성하며, Obsidian Nexus의 벡터 검색에 사용된다. L2 정규화를 적용하여 코사인 유사도를 근사한다.

→ 관련: 임베딩(Embedding), 벡터 검색(Vector Search)

---

### RRF (Reciprocal Rank Fusion)

복수의 검색 결과 목록을 단일 순위로 합산하는 알고리즘. 각 문서의 순위(rank)에 상수 60을 더한 역수를 가중 합산한다.

```
RRF_score = (1 - weight) × 1/(rank_fts + 60)
           + weight × 1/(rank_vec + 60)
```

기본 `hybrid_weight`는 0.7(벡터 70%, 키워드 30%)이다.

→ 관련: 하이브리드 검색(Hybrid Search)

---

### sqlite-vec

SQLite 확장 라이브러리로, KNN(K-Nearest Neighbor) 벡터 검색을 지원한다. 768차원 벡터를 `vec_chunks` 가상 테이블에 저장하고 L2 거리 기반으로 유사 청크를 검색한다.

```sql
SELECT ... FROM vec_chunks WHERE embedding MATCH ?1 AND k = ?2
```

→ 관련: [[04-데이터베이스-스키마]]

---

## 가나다 순

### 볼트 (Vault)

Obsidian이 노트 파일을 저장하는 로컬 폴더. `.obsidian` 디렉토리가 있는 폴더를 볼트로 인식한다. Obsidian Nexus는 여러 볼트를 등록하여 통합 검색할 수 있다.

→ 관련: [[05-설정-가이드]]

---

### 백링크 (Backlink)

특정 문서를 `[[위키링크]]` 형식으로 참조하는 다른 문서의 목록. `backlink_count`가 높은 문서는 검색 결과 리랭킹 시 최대 20% 부스트를 받는다.

→ 관련: [[02-검색-시스템]]

---

### 벡터 검색 (Vector Search)

텍스트를 수치 벡터(임베딩)로 변환한 뒤 벡터 공간에서 의미적으로 유사한 문서를 찾는 검색 방식. 키워드가 일치하지 않아도 의미가 비슷한 문서를 발견할 수 있다. sqlite-vec와 Ollama `nomic-embed-text`를 사용한다.

→ 관련: 임베딩(Embedding), sqlite-vec, [[02-검색-시스템]]

---

### 사서 에이전트 (Librarian Agent)

자연어 명령으로 볼트 문서를 탐색·관리하는 AI 에이전트. MCP 프로토콜을 통해 Obsidian Nexus의 검색·인덱싱 도구를 호출하며, `crates/agent`로 분리 구현할 예정이다.

→ 관련: [[10-사서-에이전트-설계]]

---

### 앨리어스 (Alias)

문서의 대체 이름. frontmatter의 `aliases` 필드에 등록하며, 한글 별칭으로도 영문 본문 문서를 검색할 수 있게 해 준다(Alias Fallback). FTS5 결과가 `limit` 미만일 때 `document_aliases` 테이블을 LIKE 검색하여 결과를 보충한다.

예: aliases에 `데이터독` 등록 → `"데이터독"` 검색으로 `datadog-setup.md` 매칭

→ 관련: [[02-검색-시스템]]

---

### 임베딩 (Embedding)

텍스트를 고차원 수치 벡터로 변환한 표현. 의미적으로 유사한 텍스트는 벡터 공간에서 가까운 위치를 가진다. Obsidian Nexus는 Ollama `nomic-embed-text` 모델로 768차원 임베딩을 생성하고, L2 정규화 후 sqlite-vec에 저장한다.

→ 관련: nomic-embed-text, 벡터 검색(Vector Search)

---

### 인덱싱 (Indexing)

볼트 내 마크다운 파일을 파싱하여 청크 단위로 분할하고, FTS5 및 벡터 테이블에 저장하는 과정. `crates/core`의 `index_engine.rs`가 파일 워킹과 증분 인덱싱을 담당한다.

→ 관련: 청킹(Chunking), [[01-아키텍처]]

---

### 청킹 (Chunking)

마크다운 문서를 색인 가능한 단위(청크)로 분할하는 과정. 헤딩 구조를 기준으로 섹션을 분리하며, `crates/core`의 `indexer.rs`가 담당한다. 각 청크는 `heading_path`(예: `프로젝트 > 기술 스택`)와 본문 텍스트를 포함한다.

→ 관련: 인덱싱(Indexing), [[01-아키텍처]]

---

### 하이브리드 검색 (Hybrid Search)

FTS5 키워드 검색과 벡터 의미 검색의 결과를 RRF 알고리즘으로 합산하는 검색 방식. Obsidian Nexus의 기본 검색 모드이다. 쿼리 길이에 따라 가중치가 자동 조정된다(2자 이하 ×0.3, 4자 이하 ×0.6).

→ 관련: FTS5, 벡터 검색(Vector Search), RRF, [[02-검색-시스템]]

---

## 관련 문서

- [[00-프로젝트-개요]]
- [[01-아키텍처]]
- [[02-검색-시스템]]
- [[03-MCP-도구-레퍼런스]]
- [[04-데이터베이스-스키마]]
- [[10-사서-에이전트-설계]]
