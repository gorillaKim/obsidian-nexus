---
title: 추론 지원 도구 (RAG + 모순 탐지)
tags: [idea, roadmap, rag, reasoning, mcp]
status: draft
date: 2026-03-27
---

# 아이디어: 추론 지원 도구

## 배경

현재 obsidian-nexus는 "검색"만 제공한다. 에이전트는 검색 결과를 받아서 스스로 읽고 판단해야 한다.
Ollama 연동이 이미 있으므로, 지식베이스 위에서 직접 추론하는 도구를 추가할 수 있다.

에이전트가 "이 질문에 대한 답이 문서에 있나?"를 확인하기 위해 여러 번 검색하고 읽는 대신,
단일 MCP 호출로 답을 얻을 수 있다면 토큰과 시간을 크게 아낄 수 있다.

## 제안 도구

### Phase 1: `nexus_ask(question, project?)`

RAG 파이프라인: 검색 → 청크 수집 → Ollama 추론 → 답변 + 출처 반환.

```json
// 요청
{ "question": "auth 모듈의 세션 만료 정책은?", "project": "my-project" }

// 반환
{
  "answer": "JWT 토큰은 24시간 후 만료되며, refresh token은 7일 유효합니다.",
  "sources": [
    { "path": "auth/session-management.md", "section": "## Expiry Policy", "score": 0.94 },
    { "path": "auth/token-refresh.md", "section": "## TTL", "score": 0.87 }
  ]
}
```

**구현 계획:**
- `crates/core/src/rag.rs` 신규 모듈
- `nexus_search` 상위 k개 청크 수집 → Ollama `/api/generate` 호출
- Ollama 미연결 시 답변 없이 `sources`만 반환 (graceful fallback)
- timeout 설정 필수 (로컬 LLM은 느릴 수 있음)

### Phase 2: `nexus_contradiction_check(statement, project?)`

주어진 진술과 상충하는 내용을 지식베이스에서 탐색.

```json
// 요청
{ "statement": "이 서비스는 OAuth만 지원한다", "project": "my-project" }

// 반환
{
  "contradictions": [
    {
      "path": "auth/api-key-guide.md",
      "content": "API Key 인증도 지원합니다...",
      "confidence": 0.82
    }
  ]
}
```

**구현 계획:**
- statement를 벡터화 → 유사 청크 검색
- Ollama에 "이 진술과 아래 내용이 모순인가?" 판단 요청
- 에이전트가 새 결정을 내리기 전 기존 지식과 충돌 여부 확인에 활용

## 구현 난이도

| 도구 | 난이도 | 의존성 |
|------|--------|--------|
| `nexus_ask` | 중 | Ollama 재사용, `rag.rs` 신규 |
| `nexus_contradiction_check` | 상 | 벡터 검색 + LLM 판단 루프 |

## 제약 및 고려사항

- **Ollama 의존**: 두 도구 모두 로컬 LLM 필요. 연결 실패 시 fallback 필수.
- **지연**: 로컬 추론은 수 초 소요 가능 → 에이전트가 timeout을 인지해야 함.
- **정확도**: Ollama 소형 모델 기준으로 한국어 추론 품질 검증 필요.
- **nexus_ask 먼저**: 모순 탐지보다 Q&A가 더 범용적이고 구현도 단순함.

## 우선순위

`nexus_ask` → `nexus_contradiction_check` 순으로 단계적 구현 권장.
