---
title: 사서 에이전트 (Librarian Agent) 설계
tags:
  - librarian
  - agent
  - design
  - context
---

# 사서 에이전트 (Librarian Agent) 설계

## Context
obsidian-nexus 데스크톱 앱에 AI 사서 에이전트를 추가한다. 사용자가 채팅으로 문서를 검색·요약·분석하고, 문서 생성/유지보수 제안까지 받을 수 있게 한다. agent-company 프로젝트의 CLI 자동 감지 패턴을 참고하되, Rust 네이티브로 구현하여 Python 의존성 없이 동작한다.

## 핵심 가치
- **검색의 문턱을 낮춤**: 키워드를 잘 골라야 하는 FTS5 대신, 자연어 질문으로 정보 접근
- **검색 엔진 활용 극대화**: 하이브리드 검색, 백링크 그래프, 태그 필터링을 에이전트가 조합
- **통합 UX**: 질문→검색→열람→작업이 하나의 앱에서 완결 (Claude Desktop + MCP 조합 대비 차별점)

## 배경
- **agent-company**: Electrobun + Python(CrewAI) 기반 데스크톱 에이전트 플랫폼. CLI 자동 감지(claude/gemini), JSONL IPC, 다중 에이전트 팀 구조
- **obsidian-nexus**: Tauri v2 + React 기반 Obsidian 볼트 검색 엔진. MCP 서버 11개 도구, FTS5/벡터/하이브리드 검색

### Claude Desktop + MCP 대비 내장 사서의 우위
| | Claude Desktop + MCP | 내장 사서 |
|---|---|---|
| 프로젝트 인지 | 매번 명시 | 자동 주입 |
| 문서 열기 | 경로 복사 → Obsidian | 클릭 한 번 (URI) |
| 검색 탭 연동 | 불가 | 채팅 결과 ↔ 검색 탭 연계 |
| 설치 허들 | MCP 설정 필요 | 앱 내장 |
| 문서 유지보수 | 수동 | 에이전트가 관리 제안 |

---

## 핵심 설계 결정

| 항목 | 결정 | 근거 |
|------|------|------|
| LLM 백엔드 | Claude CLI 우선 → Gemini CLI 후순위 | 사용자 PC의 CLI 활용, API 키 불필요 |
| SDK | **Claude Agent SDK V2** + V1 폴백 | V2 `createSession/send/stream`으로 프로세스 상주. V2 unstable 시 V1 `query(resume)` 폴백 |
| 통신 방식 | **Node.js sidecar + JSONL** | Rust → `Command::new("node")` → claude-bridge.mjs. stdin/stdout JSONL 프로토콜 |
| Tool Use 방식 | **MCP 위임** | SDK `mcpServers` 옵션으로 nexus MCP 서버 연결. `allowedTools`로 nexus 도구만 허용 |
| 모듈 배치 | **`crates/agent/` (새 crate)** | core에 subprocess 의존성 유입 방지 (아키텍처 규칙 준수) |
| 핵심 집중점 | **프롬프트 하네스** | 검색 시스템 활용 + 문서 생성/유지보수 관리를 잘하도록 프롬프트 설계에 집중 |
| 프롬프트 관리 | **외부 마크다운 파일** | `~/.obsidian-nexus/agents/`에 .md로 관리. 재컴파일 없이 수정 즉시 반영 |
| 세션 관리 | **SDK 세션 위임** | V2 session 객체가 프로세스 상주, 컨텍스트 압축(auto-compact) 자동 처리 |
| 대화 기록 | **세션 resume만** | 이전 대화 내용 파싱/표시는 불필요. SDK가 세션을 resume으로 이어가기 |
| 보안 | **allowedTools 화이트리스트** | `dangerously-skip-permissions` 대신 `allowedTools`로 nexus_* 도구만 허용 |

---

## 세션 & 모델 관리

### 설계 원칙
CLI(Claude/Gemini)가 이미 세션 관리, 컨텍스트 압축(auto-compact), 히스토리 저장을 자체 수행하므로 우리 앱은 **메타데이터만 관리**한다.

### CLI 세션 기능 활용

| 기능 | Claude CLI | Gemini CLI |
|------|-----------|------------|
| 세션 시작 | `--session-id {uuid}` | 자동 생성 |
| 세션 이어가기 | `--resume {uuid}` | `--resume {index\|latest}` |
| 세션 이름 | `--name {name}` | - |
| 모델 선택 | `--model sonnet` / `--model opus` | `--model gemini-2.5-pro` |
| 컨텍스트 압축 | 자동 (auto-compact) | 자동 |
| 세션 포크 | `--fork-session` | - |
| MCP 도구 | `--mcp-config nexus.json` | 별도 설정 |

### 우리 앱이 관리하는 것
```json
// sessions.json (로컬 파일, 가벼운 메타데이터만)
[
  {
    "id": "a1b2c3d4-...",
    "cli": "claude",
    "model": "sonnet",
    "name": "AWS 비용 분석",
    "projectId": "vault-001",
    "createdAt": "2026-03-20T10:00:00Z"
  }
]
```

### CLI가 관리하는 것 (전부 위임)
- 대화 히스토리 저장/복원
- 컨텍스트 윈도우 관리 & auto-compact
- 토큰 카운팅
- Claude: `~/.claude/projects/{project}/{session-id}.jsonl`
- Gemini: `~/.gemini/history/{hash}/`

### 멀티 세션 흐름
```
[새 세션] → UUID 생성 → sessions.json에 메타 저장
         → claude --session-id {uuid} --model {model} --mcp-config nexus.json
         → 대화형 모드로 상주 (stdin/stdout)

[세션 이어가기] → sessions.json에서 ID 조회
               → claude --resume {uuid}
               → CLI가 자체 히스토리 복원 + auto-compact

[세션 전환] → UI에서 탭 클릭 → 해당 세션의 CLI 프로세스로 전환
           → (프로세스가 없으면 --resume으로 재시작)

[모델 선택] → 세션 생성 시 선택: claude(sonnet), claude(opus), gemini
           → 세션 중간에 모델 변경은 불가 (새 세션 생성)
```

### 이전 대화 기록에 대한 결정
- **이전 대화 내용 표시**: 하지 않음. CLI의 JSONL/git 파싱 복잡도 대비 가치 낮음
- **세션 목록**: sessions.json에서 이름/시간만 표시
- **세션 resume**: CLI가 처리. 사용자는 세션 탭을 클릭하면 이어서 대화
- 사서의 답변이 아니라 **찾아준 문서가 가치의 본체** — 대화 원문 보관 의미 약함

---

## 아키텍처

```
┌─────────────────────────────────────────────┐
│  React Frontend (ChatPanel)                 │
│  - 멀티 세션 탭 (생성/전환/닫기)              │
│  - 세션별 모델 선택 (claude/gemini + model)  │
│  - 채팅 UI (메시지 목록, 입력창)              │
│  - 실시간 스트리밍 토큰 표시                  │
│  - tool_use 상태 ("nexus_search 검색 중...") │
│  - thinking block 표시                      │
│  - 문서 참조 링크 렌더링 (→ Obsidian URI)     │
│  - 에이전트 상태 표시 (generating/done/error) │
└──────────────┬──────────────────────────────┘
               │ Tauri invoke + listen("chat-stream")
┌──────────────▼──────────────────────────────┐
│  Tauri Commands (main.rs)                   │
│  - chat_start_session: sidecar 시작         │
│  - chat_send_message: 메시지 → sidecar stdin │
│  - chat_cancel: 취소 요청                    │
│  - chat_close_session: sidecar 종료          │
│  - detect_cli_agents: CLI 감지 (async)       │
│  - chat_list_sessions: 세션 목록             │
│  + emit("chat-stream", payload) 스트리밍      │
└──────────────┬──────────────────────────────┘
               │ stdin/stdout JSONL
┌──────────────▼──────────────────────────────┐
│  Node.js Sidecar (claude-bridge.mjs)        │
│  - @anthropic-ai/claude-agent-sdk           │
│  - V2 createSession/send/stream (기본)       │
│  - V1 query(resume) 폴백                    │
│  - stdout = JSONL 프로토콜 전용              │
│  - stderr = 디버그 로그                      │
│  - 크래시 시 자동 재시작 (exponential backoff)│
└──────────────┬──────────────────────────────┘
               │ SDK 내부 관리
┌──────────────▼──────────────────────────────┐
│  Claude Code Process (상주)                  │
│  - MCP 서버 1회 초기화 (세션 동안 유지)       │
│  - allowedTools: nexus_* 도구만 허용         │
│  - auto-compact 자동 처리                    │
└──────────────┬──────────────────────────────┘
               │ MCP protocol
┌──────────────▼──────────────────────────────┐
│  nexus-mcp-server (상주)                     │
│  - nexus_search, nexus_get_document, ...     │
│  - 11개 도구                                 │
└─────────────────────────────────────────────┘
```

### Gemini 확장 구조
```
apps/desktop/sidecar/
├── claude-bridge.mjs    ← Claude Agent SDK
├── gemini-bridge.mjs    ← 향후: Gemini SDK 또는 CLI 래퍼
└── bridge-protocol.d.ts ← 공통 JSONL 프로토콜 (양쪽 동일)
```
- Rust/프론트엔드는 bridge 프로토콜만 알면 됨 → CLI별 코드 변경 없음

### crates/agent 내부 구조

```
crates/agent/src/
├── cli_detector.rs   — CLI 감지 (which + version + OAuth)
├── session.rs        — 세션 메타데이터 (sessions.json CRUD)
├── cli_bridge.rs     — SidecarManager (Node.js 프로세스 관리)
├── prompt.rs         — 프롬프트 로더 (외부 .md + fallback + validation)
├── error.rs          — AgentError (thiserror)
└── lib.rs            — 모듈 선언
```

### Bridge JSONL 프로토콜

**요청 (Rust → Node stdin)**
```jsonl
{"type":"start","sessionId":"uuid","model":"sonnet","systemPrompt":"...","mcpServers":{"nexus":{"command":"nexus-mcp-server"}},"allowedTools":["nexus_*"]}
{"type":"message","sessionId":"uuid","content":"AWS 관련 문서 있어?"}
{"type":"cancel","sessionId":"uuid"}
{"type":"close","sessionId":"uuid"}
```

**응답 (Node stdout → Rust → Tauri event)**
```jsonl
{"type":"init","sessionId":"uuid","model":"sonnet","mcpServers":["nexus"]}
{"type":"thought","sessionId":"uuid","content":"사용자가 AWS 문서를 요청..."}
{"type":"tool_use","sessionId":"uuid","toolName":"nexus_search","input":{"query":"AWS","mode":"hybrid"},"status":"running"}
{"type":"tool_use","sessionId":"uuid","toolName":"nexus_search","status":"done"}
{"type":"text","sessionId":"uuid","content":"3개 문서를 찾았습니다...","done":false}
{"type":"result","sessionId":"uuid","content":"최종 답변","cost":0.05,"duration":2100,"usage":{"input":1500,"output":200,"cache_read":5000}}
{"type":"error","sessionId":"uuid","code":"auth_expired","message":"인증 만료","retryable":false}
```

### 세션 흐름 (Agent SDK V2)
```
[세션 시작]
  → prompt.rs: 시스템 프롬프트 + 프로젝트 컨텍스트 조합
  → SidecarManager: node claude-bridge.mjs 스폰
  → stdin: {"type":"start","sessionId":"...","model":"sonnet","systemPrompt":"...","mcpServers":{...}}
  → SDK V2: createSession() → Claude Code 프로세스 + MCP 서버 초기화 (1회)
  → stdout: {"type":"init",...}

[메시지 전송]
  → stdin: {"type":"message","sessionId":"...","content":"질문"}
  → SDK V2: session.send("질문") → session.stream()
  → stdout: {"type":"thought",...} → {"type":"tool_use",...} → {"type":"text",...} → {"type":"result",...}
  → Tauri: emit("chat-stream", payload) → React ChatPanel 실시간 표시

[같은 세션에서 후속 질문]
  → stdin: {"type":"message","sessionId":"...","content":"후속 질문"}
  → SDK V2: session.send() (같은 프로세스, MCP 재시작 없음)
  → stdout: 스트리밍 응답

[취소]
  → stdin: {"type":"cancel","sessionId":"..."}
  → SDK: AbortController.abort()
  → stdout: {"type":"result","sessionId":"...","content":"취소됨"}

[세션 종료]
  → stdin: {"type":"close","sessionId":"..."}
  → SDK: session.close()
  → sidecar 프로세스 종료
```

---

## 프롬프트 하네스 설계 (★ 핵심)

### 설계 원칙
- **코드 변경 없이 에이전트 동작 변경 가능**: 프롬프트는 외부 마크다운 파일로 관리
- **지속적 개선**: .md 파일만 수정하면 즉시 반영 (재컴파일 불필요)
- **모듈화**: 역할별로 파일 분리하여 독립적으로 수정/교체 가능

### 디렉토리 구조
```
~/.obsidian-nexus/agents/
├── librarian/                    # 사서 에이전트
│   ├── system.md                 # 역할 정의 + 성격
│   ├── search-strategy.md        # 검색 도구 활용 가이드
│   ├── doc-maintenance.md        # 문서 유지보수 지침
│   ├── app-guide.md              # 앱/MCP/CLI 사용 가이드
│   └── output-rules.md           # 출력 포맷 규칙
├── config.json                   # 에이전트 설정 (프롬프트 조합 순서, 활성화 등)
└── custom/                       # 사용자 커스텀 에이전트 (향후)
```

### config.json
```json
{
  "agents": {
    "librarian": {
      "name": "사서",
      "prompts": [
        "librarian/system.md",
        "librarian/search-strategy.md",
        "librarian/doc-maintenance.md",
        "librarian/app-guide.md",
        "librarian/output-rules.md"
      ],
      "enabled": true
    }
  }
}
```

### 프롬프트 파일 예시

**librarian/system.md**
```markdown
---
name: 사서 역할 정의
version: 1.0
---

당신은 Obsidian 볼트의 전문 사서입니다.
사용자의 질문에 대해 볼트의 문서를 검색하고, 요약하고, 분석하여 답변합니다.
문서 간의 관계를 파악하고, 지식의 빈틈을 발견하면 적극적으로 알려줍니다.
```

**librarian/search-strategy.md**
```markdown
---
name: 검색 전략 가이드
version: 1.0
---

## 검색 도구 사용 가이드
- 자연어 질문 → 키워드 추출 → nexus_search(mode: hybrid) 우선 사용
- 결과 부족 시: 태그 필터링, 백링크 탐색, alias 검색 순으로 확장
- 짧은 쿼리(≤2자)는 prefix 매칭 적용
- 언더스코어 토큰은 분리하여 OR 검색
- 여러 문서를 종합해야 할 때: nexus_get_backlinks로 관련 문서 그래프 탐색
```

**librarian/doc-maintenance.md**
```markdown
---
name: 문서 유지보수 지침
version: 1.0
---

## 문서 관리 제안 규칙
- 문서 생성 제안 시: nexus_search로 유사 문서 탐색하여 중복 확인 필수
- 문서 구조: Obsidian 호환 마크다운 (위키링크, 태그, frontmatter)
- 링크 관리: 새 문서 작성 시 관련 기존 문서에 백링크 추가 제안
- 태그 일관성: nexus_get_metadata로 기존 태그 체계 확인 후 제안

## 자동 감지 항목
- 고아 문서: 백링크가 0개인 문서 발견 시 연결 제안
- 중복 문서: 유사 내용의 문서 발견 시 통합/정리 제안
- 깨진 링크: 존재하지 않는 문서로의 위키링크 발견 시 알림
- 태그 혼용: 유사 태그(예: #deploy, #배포) 통일 제안
- 네이밍 패턴: 기존 구조에 맞춘 파일명/디렉토리 제안
```

**librarian/app-guide.md**
```markdown
---
name: 앱/MCP/CLI 가이드
version: 1.0
---

## obsidian-nexus 앱 가이드
사용자가 앱, MCP 도구, CLI 사용법에 대해 질문하면 안내해주세요.

### 앱 기능
- 검색 탭: 키워드/벡터/하이브리드 검색, 태그 필터링, 프로젝트별 탐색
- 프로젝트 관리: 볼트 추가/제거, 인덱싱, 프로젝트 정보 조회
- 대시보드: 전체 현황 요약
- 설정: CLI 감지 상태, 검색 설정, 볼트 동기화

### MCP 도구 레퍼런스
nexus MCP 서버는 다음 도구를 제공합니다:
- nexus_search: 하이브리드/키워드/벡터 검색 (mode, tags, limit 파라미터)
- nexus_get_document: 문서 전문 조회
- nexus_get_section: 헤딩 기반 섹션 추출
- nexus_get_metadata: 문서 메타데이터 (태그, alias, frontmatter)
- nexus_get_backlinks: 역방향 링크 조회
- nexus_get_links: 순방향 링크 조회
- nexus_list_documents: 프로젝트 내 문서 목록
- nexus_list_projects: 등록된 프로젝트 목록
- nexus_index_project: 프로젝트 인덱싱 실행
- nexus_resolve_alias: 별칭으로 문서 찾기
- nexus_status: 시스템 상태 확인
- nexus_help: 도구 사용법 안내
- nexus_onboard: AI 도구에 nexus 등록 가이드

### CLI 사용법
- `nexus search "쿼리"` — 터미널에서 검색
- `nexus index /path/to/vault` — 볼트 인덱싱
- `nexus list` — 프로젝트 목록
- `nexus status` — 시스템 상태

### 자주 묻는 질문 패턴
- "검색이 안 돼요" → 인덱싱 상태 확인 (nexus_status), 재인덱싱 안내
- "MCP 설정 어떻게 해요" → nexus_onboard 도구 안내
- "벡터 검색이 안 돼요" → Ollama 연결 상태 확인 안내
- "태그 필터가 안 먹어요" → 태그 형식 확인 (#태그 vs 태그)
```

**librarian/output-rules.md**
```markdown
---
name: 출력 규칙
version: 1.0
---

- 참조 문서는 반드시 경로와 함께 표시
- 인용 시 해당 섹션 명시 (nexus_get_section 활용)
- 답변 언어는 사용자 질문 언어에 맞춤
```

### 프롬프트 로딩 & 변수 치환 (prompt.rs)

`prompt.rs`는 프롬프트를 작성하지 않고 **로딩+조합**만 담당:

```
1. config.json 읽기 → 활성 에이전트의 프롬프트 파일 목록 획득
2. 각 .md 파일 읽기 → frontmatter 파싱 → 본문 추출
3. 프로젝트 컨텍스트 변수 치환:
   - {project_name} → "my-vault"
   - {project_path} → "/Users/.../my-vault"
   - {doc_count} → "142"
   - {top_tags} → "#dev, #infra, #meeting"
4. 전체 조합 → --system-prompt로 CLI에 전달
```

### 프롬프트 업데이트 워크플로우
```
개발자가 .md 파일 수정 → 앱 재시작 or 새 세션 생성 → 반영 완료
(재컴파일 불필요, 앱 업데이트 시 기본값 동봉)
```

### Fallback & Validation
- **Fallback 내장**: 앱 바이너리에 기본 프롬프트를 `include_str!`로 내장. 외부 파일이 없거나 깨졌을 때 자동으로 내장 기본값 사용
- **Prompt Validation**: 세션 시작 전 검증 수행
  - 필수 변수(`{project_name}` 등)가 치환되었는지
  - 필수 섹션(역할 정의, 검색 전략)이 존재하는지
  - frontmatter 문법이 올바른지
  - 검증 실패 시 fallback 프롬프트로 대체 + 사용자에게 경고 표시

### 앱 업데이트 시 기본값 관리
- 앱에 기본 프롬프트 파일을 리소스로 동봉
- 첫 실행 시 `~/.obsidian-nexus/agents/`에 복사
- 업데이트 시: 사용자가 수정하지 않은 파일만 덮어쓰기 (version 비교)

---

## 구현 단계

### Step 0: IPC 파이프라인 Mock 검증 (스파이크)
- Mock Node.js 스크립트로 Rust ↔ sidecar JSONL 통신 검증
- stdin 요청 → stdout 응답 → Tauri event emit → React 수신 전체 파이프라인 확인
- Agent SDK V2 `createSession/send/stream` 실제 동작 확인
- V2 실패 시 V1 `query(resume)` 폴백 동작 확인

### Step 1: Sidecar 생성 (claude-bridge.mjs)
**파일**: `apps/desktop/sidecar/`

1. `package.json` — `@anthropic-ai/claude-agent-sdk` exact 버전 고정
2. `bridge-protocol.d.ts` — 공통 요청/응답 타입 정의 (참조용)
3. `claude-bridge.mjs` — 메인 로직:
   - stdin JSONL 수신 → 요청 타입별 분기
   - `start`: V2 `createSession()` 호출 (실패 시 V1 폴백 플래그 설정)
   - `message`: V2 `session.send()` + `session.stream()` (V1: `query(resume)`)
   - `cancel`: `AbortController.abort()`
   - `close`: `session.close()`
   - stdout = JSONL 프로토콜 전용 (프로토콜 오염 방지)
   - stderr = 디버그 로그
   - SDK 메시지 → bridge 프로토콜 변환:
     - `assistant` → `text` (content 추출)
     - `assistant.thinking` → `thought`
     - `assistant.tool_use` → `tool_use` (toolName + input 포함)
     - `result` → `result` (usage/cost 포함)
     - 에러 → `error` (retryable 플래그 포함)
   - `allowedTools`로 nexus_* 도구만 허용 (dangerously-skip-permissions 대체)

### Step 2: Rust — SidecarManager
**파일**: `crates/agent/src/cli_bridge.rs` 전면 교체

1. `SidecarManager` struct:
   - `start(sidecar_path, model, system_prompt, mcp_config)` → Node.js 프로세스 스폰
   - `send(session_id, message)` → stdin에 JSONL 쓰기
   - `read_response()` → stdout에서 JSONL 한 줄 읽기
   - `cancel(session_id)` / `close(session_id)`
   - `shutdown()` — 앱 종료 시 PGID 기반 전체 정리
2. `BufReader<ChildStdout>` 보존 (UTF-8 경계 안전)
3. 크래시 감지 + 자동 재시작 (exponential backoff)
4. 비활성 세션 auto-kill (10분)
5. Drop impl으로 프로세스 정리 보장

### Step 3: Tauri 커맨드 — 스트리밍 이벤트
**파일**: `apps/desktop/src-tauri/src/main.rs`

1. `chat_start_session`: sidecar 시작 + "start" JSONL 전송
2. `chat_send_message`: "message" JSONL 전송 + 백그라운드 스레드에서 stdout 읽기 → `emit("chat-stream", payload)`
3. `chat_cancel`: "cancel" JSONL 전송
4. `chat_close_session`: "close" JSONL 전송 + sidecar 종료
5. 기존 유지: `detect_cli_agents` (async), `chat_list_sessions`, `chat_delete_session`, `chat_build_prompt`

### Step 4: 프론트엔드 — 스트리밍 수신
**파일**: `apps/desktop/src/hooks/useChat.ts`, `ChatPanel.tsx`

1. `useChat.ts`:
   - `invoke("chat_start_session")` → 세션 시작
   - `invoke("chat_send_message")` → 메시지 전송
   - `listen("chat-stream")` → 실시간 응답 수신
   - 메시지 타입별 처리:
     - `thought` → 추론 과정 접힘/펼침 표시
     - `tool_use` → "nexus_search('AWS') 검색 중..." 상태 표시
     - `text` → 답변 텍스트 실시간 누적
     - `result` → 완료 + usage/cost 표시
     - `error` → 에러 메시지 + retryable이면 "재시도" 버튼
2. `ChatPanel.tsx`:
   - 스트리밍 텍스트 실시간 표시 (토큰 단위)
   - tool_use 시 도구명 + 입력 인자 표시
   - thinking block 접힘/펼침
   - 취소 버튼 → `invoke("chat_cancel")`
   - 비용/토큰 정보 하단 표시 (선택적)

### Step 5 (기존 Phase 1/3 — 이미 구현됨)
- ✅ `crates/agent/` crate (cli_detector, session, prompt, error)
- ✅ 기본 프롬프트 파일 5개 (librarian/*.md)
- ✅ ChatPanel UI 기본 구조
- ✅ useChat 훅

### Phase 5: 고도화
- Gemini 지원: `gemini-bridge.mjs` 추가 (같은 bridge 프로토콜)
- 문서 작성/편집: Diff 표시 + 사용자 승인 방식
- "오늘의 정리 제안": 앱 시작 시 능동적 유지보수 제안
- 다중 프로젝트 크로스 검색
- "자주 참조한 문서 랭킹" / "최근 탐색 주제" 집계
- 세션 이름 자동 생성 (첫 질문 기반)

---

## 검색 탭 vs 사서 채팅 포지셔닝

| | 검색 탭 | 사서 채팅 |
|---|---|---|
| 용도 | 정밀 필터링 + 직접 탐색 | 자연어 질문 → 종합 답변 |
| 입력 | 키워드, 태그, 검색 모드 선택 | 자연어 문장 |
| 출력 | 문서 목록 (결과 리스트) | 요약/분석 + 참조 문서 링크 |
| 적합한 경우 | "정확한 문서명/태그를 알 때" | "모호한 질문, 여러 문서 종합 필요 시" |
| 연동 | 독립 | 참조 문서 클릭 → Obsidian 열기 |

---

## 수정 대상 파일

### 이미 구현됨 (✅)
| 파일 | 상태 |
|------|------|
| `crates/agent/Cargo.toml` | ✅ agent crate 정의 |
| `crates/agent/src/lib.rs` | ✅ 모듈 선언 |
| `crates/agent/src/cli_detector.rs` | ✅ CLI 감지 (14개 테스트 통과) |
| `crates/agent/src/session.rs` | ✅ 세션 메타데이터 관리 |
| `crates/agent/src/prompt.rs` | ✅ 프롬프트 로더 (fallback + validation) |
| `crates/agent/src/error.rs` | ✅ AgentError |
| `crates/agent/resources/librarian/*.md` | ✅ 기본 프롬프트 5개 |
| `apps/desktop/src/hooks/useChat.ts` | ✅ 채팅 훅 (스트리밍 전환 필요) |
| `apps/desktop/src/components/layout/ChatPanel.tsx` | ✅ 기본 UI (스트리밍 전환 필요) |

### 신규 생성 (Step 0-4)
| 파일 | 역할 |
|------|------|
| `apps/desktop/sidecar/package.json` | sidecar 의존성 (SDK exact 버전) |
| `apps/desktop/sidecar/claude-bridge.mjs` | Agent SDK V2 래퍼 + V1 폴백 |
| `apps/desktop/sidecar/bridge-protocol.d.ts` | 공통 JSONL 타입 정의 |

### 수정 (Step 2-4)
| 파일 | 변경 |
|------|------|
| `crates/agent/src/cli_bridge.rs` | SidecarManager로 전면 교체 |
| `apps/desktop/src-tauri/src/main.rs` | sidecar IPC + emit("chat-stream") 스트리밍 |
| `apps/desktop/src/hooks/useChat.ts` | listen("chat-stream") 스트리밍 수신 |
| `apps/desktop/src/components/layout/ChatPanel.tsx` | 실시간 토큰 + tool_use + thought 표시 |
| `apps/desktop/src/types/index.ts` | Bridge 응답 타입 추가 |

---

## 검증 방법

1. **Step 0**: Mock sidecar로 Rust ↔ Node JSONL IPC 파이프라인 검증
2. **Step 1**: `echo '{"type":"start",...}' | node claude-bridge.mjs` — sidecar 단독 동작 확인
3. **Step 1**: Agent SDK V2 `createSession/send/stream` 동작 + V1 폴백 확인
4. **Step 2**: `cargo test -p nexus-agent` — SidecarManager 단위 테스트
5. **Step 3**: 앱에서 채팅 → Tauri event 수신 → 프론트 표시 확인
6. **Step 4**: 같은 세션 3+ 메시지 → 맥락 유지 + MCP 재시작 없음 확인
7. **E2E**: 문서 검색 질문 → nexus MCP 도구 호출 → 스트리밍 답변 + 참조 링크
8. **취소**: 진행 중 취소 → 즉시 중단 확인

---

## 리스크 & 완화

| 리스크 | 완화 방안 |
|--------|----------|
| SDK V2 unstable API 변경 | SDK exact 버전 고정 + V1 `query(resume)` 자동 폴백 |
| Node.js sidecar 크래시 | 자동 재시작 (exponential backoff) + 에러 이벤트 프론트 전달 |
| CLI 미설치 사용자 | 온보딩 화면 + 설치 가이드 + "다시 감지" 버튼 |
| 보안: MCP 도구 남용 | `allowedTools: ["nexus_*"]`로 nexus 도구만 허용 |
| 고아 프로세스 | PGID 기반 kill + Drop impl + Tauri ExitPayload 이벤트 |
| 인증 만료 | `error.retryable` 플래그 + CLI 재인증 안내 UI |
| 비활성 세션 메모리 | 10분 auto-kill + resume으로 필요 시 복원 |
| stdout 프로토콜 오염 | stdout=JSONL 전용, stderr=디버그 로그 분리 |

---

## agent-company에서 차용하는 것 / 하지 않는 것

**차용**:
- `detector.ts` → `cli_detector.rs`: CLI 감지 로직 (which + version + OAuth 확인)
- stdout 스트리밍 파싱 개념
- subprocess lifecycle 관리 (heartbeat/crash recovery 개념)

**차용하지 않음**:
- Python/CrewAI 런타임 (Rust로 대체)
- 다중 에이전트 팀 구조 (단일 사서)
- API 키 암호화 저장 (CLI 인증에 위임)
- 대화 히스토리 DB/파싱 (CLI 세션 resume으로 대체)
- JSONL IPC 프로토콜 (대화형 CLI stdin/stdout으로 대체)
