---
tags:
  - overview
  - comparison
aliases:
  - Serena 비교
  - Nexus vs Serena
---

# Obsidian Nexus vs Serena 비교

## 근본적인 목적 차이

| 구분 | **Obsidian Nexus** | **Serena** |
|------|-------------------|------------|
| **핵심 대상** | 마크다운 문서/지식 베이스 | 소스 코드 |
| **한줄 정의** | Obsidian 볼트를 AI Agent가 탐색 가능한 지식 베이스로 만드는 도구 | 코드베이스의 의미적 구조를 AI에게 제공하는 코드 인텔리전스 서버 |
| **타겟 사용자** | 지식 관리자, 문서 작성자, 팀 위키 운영자 | 개발자 (코드 탐색/리팩토링) |

## 기술 아키텍처 비교

| 구분 | **Obsidian Nexus** | **Serena** |
|------|-------------------|------------|
| **검색 방식** | 하이브리드 (FTS5 키워드 + 벡터 의미 검색) | AST 기반 코드 구조 파싱 (Tree-sitter) |
| **인덱싱 단위** | 마크다운 헤딩 기반 청크 | 코드 심볼 (클래스, 함수, 임포트) |
| **그래프 탐색** | 위키링크 백링크 그래프 | 코드 호출/참조 그래프 |
| **데이터 저장** | SQLite 로컬 (`~/.nexus/nexus.db`) | 로컬 인덱스 |
| **멀티 프로젝트** | ✅ 멀티 볼트 통합 검색 | ✅ 멀티 레포 지원 |
| **AI 통합** | MCP 프로토콜 (13개+ 도구) | MCP 프로토콜 |

## Nexus 차별점

### 지식 문서 특화 기능
- **헤딩 기반 섹션 추출** (`nexus_get_section`): 문서 전체를 읽지 않고 필요한 섹션만 정확히 추출
- **TOC 탐색** (`nexus_get_toc`): 문서 목차를 먼저 파악 후 필요한 부분만 조회
- **태그 필터링**: 태그 기반 필터 검색 (OR/AND 모드)

### 하이브리드 검색 엔진
- 키워드 검색 (정확한 용어 매칭) + 벡터 검색 (의미적 유사성) 결합
- Ollama 로컬 임베딩으로 완전한 **로컬 퍼스트** 유지

### 문서 인기도/랭킹 시스템
- `view_count × 0.6 + backlink_count × 0.4` 기반 문서 랭킹
- 자주 참조되는 핵심 문서를 우선 표시

### 백링크 그래프 탐색
- 위키링크 기반 양방향 링크 추적 (`nexus_get_backlinks`, `nexus_get_links`)
- 관련 문서 자동 발견, 고아 문서 감지

### 토큰 효율성
- 기존 방식(Grep/Read): 27개 파일 목록 → 수동 선별 → 전체 읽기 = 노이즈 + 토큰 낭비
- Nexus 방식: `nexus_search` → 랭킹된 스니펫 → 필요한 섹션만 = **토큰 59% 절약**

## Serena 차별점

### LSP 기반 아키텍처 — 별도 인덱스/벡터 DB 없음

Serena는 자체 검색 엔진을 구축하지 않고, **이미 수십 년간 발전해온 LSP(Language Server Protocol) 생태계**를 그대로 활용한다.

> *"벡터 데이터베이스에 의존하지 않는다. LSP 생태계가 이미 구조적 코드 분석을 해결했고, Serena는 이 역량을 깔끔한 프로토콜 경계를 통해 LLM에 제공한다."*

```
Nexus 방식:
  문서 → 청크 분할 → FTS5 인덱스 + 벡터 임베딩 → 하이브리드 검색
  (자체 검색 엔진을 구축)

Serena 방식:
  코드 → LSP 서버 (pyright, typescript-language-server 등) → 심볼 분석
  (IDE가 쓰는 것과 동일한 언어 서버를 MCP로 연결)
```

| 구분 | **Obsidian Nexus** | **Serena** |
|------|-------------------|------------|
| **인덱싱** | FTS5 + 벡터 임베딩 (SQLite) | LSP 서버의 심볼 테이블 |
| **벡터 DB** | ✅ Ollama 임베딩 → SQLite 저장 | ❌ 사용하지 않음 |
| **렉시컬 검색** | ✅ FTS5 전문 검색 | ❌ 텍스트 검색 대신 심볼 쿼리 |
| **저장소** | `~/.nexus/nexus.db` (SQLite) | `.serena/memories/` (마크다운 파일) |
| **검색 방식** | 키워드 + 의미 유사도 매칭 | "Go to Definition", "Find References" 등 IDE 기능 |

### 메모리 시스템

Serena는 `.serena/memories/` 디렉토리에 마크다운 형태의 메모리 파일을 저장한다:
- 첫 실행 시 프로젝트 구조, 핵심 로직을 분석하여 메모리로 저장
- 이후 대화에서 이 메모리를 재활용
- 벡터 유사도가 아닌 **구조적 이해(심볼 관계)**에 기반

### 핵심 기능

| 기능 | 설명 |
|------|------|
| **LSP 통합** | pyright, typescript-language-server 등 기존 언어 서버 활용 |
| **심볼 탐색** | 함수 정의, 클래스 상속, 임포트 관계 추적 (Go to Definition, Find References) |
| **코드 리팩토링 지원** | 심볼 이름 변경, 참조 추적 등 코드 수정에 특화 |
| **다중 언어 지원** | Python, TypeScript, Go 등 여러 프로그래밍 언어 |

## 왜 접근 방식이 다른가

Nexus가 자체 검색 엔진(FTS5 + 벡터)을 구축한 것은, **마크다운 문서에는 LSP 같은 구조적 분석 도구가 존재하지 않기 때문**이다. 반면 Serena는 이미 존재하는 LSP를 활용할 수 있는 코드 도메인이라 별도 인덱스가 불필요하다.

| | **Nexus** | **Serena** |
|---|-----------|------------|
| **강점** | "이 개념에 대해 쓴 문서가 있나?" (의미 검색) | "이 함수를 호출하는 곳이 어디야?" (구조 탐색) |
| **약점** | 코드 심볼 관계를 이해 못함 | 자연어 문서의 의미적 유사도 검색 불가 |

## 사용 시나리오별 선택 가이드

| 시나리오 | 추천 도구 |
|----------|-----------|
| 프로젝트 문서/위키/노트를 AI로 검색 | **Obsidian Nexus** |
| 코드베이스를 AI가 구조적으로 이해 | **Serena** |
| 문서 + 코드 모두 필요 | **Nexus + Serena** 동시 사용 (둘 다 MCP 서버) |

> [!tip] 상호 보완 관계
> 경쟁이 아닌 **상호 보완**. Nexus는 **지식/문서 레이어**, Serena는 **코드 레이어**를 담당한다.
> 코드 프로젝트의 `docs/` 폴더를 Nexus로 인덱싱하면서, 코드 자체는 Serena로 탐색하는 조합이 이상적.

## 관련 문서

- [[project-overview]]
- [[mcp-tools]]
- [[search-system]]
- [[architecture]]

## 참고 문헌

- [GitHub - oraios/serena](https://github.com/oraios/serena) — Serena 공식 저장소
- [Deconstructing Serena's MCP-Powered Semantic Code Understanding Architecture](https://medium.com/@souradip1000/deconstructing-serenas-mcp-powered-semantic-code-understanding-architecture-75802515d116) — Serena 아키텍처 분석 (Medium)
- [Serena MCP Server: A Deep Dive for AI Engineers](https://skywork.ai/skypage/en/Serena-MCP-Server-A-Deep-Dive-for-AI-Engineers/1970677982547734528) — AI 엔지니어 관점 심층 분석 (Skywork AI)
- [Serena MCP: Free AI Coding Agent with Full Codebase Understanding](https://smartscope.blog/en/generative-ai/claude/serena-mcp-coding-agent/) — Serena 기능 소개 (SmartScope)
- [MCP Server Integration | oraios/serena | DeepWiki](https://deepwiki.com/oraios/serena/3.2-mcp-server-integration) — MCP 서버 통합 구조 (DeepWiki)
