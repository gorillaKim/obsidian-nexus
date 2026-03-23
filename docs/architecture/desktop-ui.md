# 데스크톱 UI 아키텍처

## 레이아웃 구조

```
┌──────────────────────────────────────────────────────────────┐
│          App (h-screen, flex, overflow-hidden)               │
├────────────┬─────────────────────────────┬───────────────────┤
│  Sidebar   │   Main (flex-1)             │  ChatPanel        │
│  56~200px  │   min-w: 480px              │  280~640px        │
│  접기/펼기  │   overflow-x-auto           │  드래그 리사이즈    │
└────────────┴─────────────────────────────┴───────────────────┘
```

### 동작 규칙
- **Sidebar**: Framer Motion으로 56px(접힘) ↔ 200px(펼침) 애니메이션. flex 자식이므로 메인 영역을 밀어냄
- **Main**: `flex-1 min-w-[480px]`. 480px 미만으로 줄어들지 않음. 내용이 넘칠 경우 `overflow-x-auto`로 가로 스크롤
- **ChatPanel**: flex 자식으로 Main을 왼쪽으로 밀어냄. Main이 480px min-width에 도달하면 부모 `overflow-hidden`에 의해 채팅창이 뒤로 감춰짐

### 채팅 패널 리사이즈
- 범위: 280px ~ 640px (기본값 360px)
- 조작: 채팅창 좌측 1px 보더를 드래그 (`cursor-col-resize`)
- 상태: `chatWidth` state를 App.tsx에서 관리, ChatPanel에 prop으로 전달

---

## 채팅 세션 라이프사이클

### 세션 생성 (유일한 연결 시점)

```
사용자: "새 세션 시작" 클릭
  ↓
chat_new_session (Tauri)
  → DB에 SessionMeta 저장 (id, cli, model, project_id, name)
  ↓
chat_start_session (Tauri)
  → Node.js sidecar 프로세스 시작 (claude-bridge.mjs)
  → SDK createSession() 호출
  ← 사이드카 연결 완료 ← 이 시점에 실제 CLI 연결
```

**사이드카 연결은 세션 생성 시 단 한 번만 발생한다.**

### 메시지 전송

```
sendMessage(content)
  → UI에 user 메시지 + assistant placeholder 추가 (낙관적 업데이트)
  → chat_send_message (Tauri IPC)
  ← chat-stream Tauri 이벤트로 스트리밍 수신
     - "text": assistant placeholder 내용 실시간 업데이트
     - "tool_use": 도구 실행 상태 표시
     - "result": 최종 완료
     - "error": 에러 표시
```

### 세션 전환

```
switchSession(sessionId)
  → activeSessionId state만 변경
  → 백엔드 호출 없음
  → 사이드카 재연결 없음
```

**주의**: 기존 세션의 사이드카 프로세스가 여전히 살아있다고 가정한다. 앱을 재시작하면 사이드카가 종료되므로, 이전 세션에서 메시지를 보내면 오류가 발생한다.

### 세션 삭제

```
deleteSession(sessionId)
  → UI에서 즉시 제거 (낙관적 업데이트)
  → chat_close_session (Tauri) — best-effort, 실패해도 UI는 유지
  → 사이드카 프로세스 종료 시도
```

### 앱 시작 시 세션 복원

```
ChatPanel mount
  → detectAgents(): CLI 설치 여부 확인
  → loadSessions(): DB에서 SessionMeta 목록 로드
     → 세션이 있으면 첫 번째 자동 선택
     → 사이드카 프로세스는 재시작하지 않음
```

**결과**: 앱 재시작 후 기존 세션 목록은 표시되지만, 메시지 전송 시 사이드카가 없어 오류 발생. 새 세션을 시작해야 함.

---

## 세션 상태 머신

```
idle → generating (sendMessage 호출 시)
generating → done (result 이벤트)
generating → error (error 이벤트)
done → idle (500ms 후 자동)
error → idle (새 메시지 전송 시)
generating → idle (cancelMessage 시)
```

상태별 UI:
- `idle`: 입력창 활성
- `generating`: 입력창 비활성, 취소 버튼 표시, 스피너 표시
- `compacting`: "사서가 기억을 정리 중..." (Claude auto-compact)
- `done`: 잠시 후 idle로
- `error`: 에러 메시지 표시

---

## 마크다운 렌더링

assistant 메시지만 `react-markdown` + `remark-gfm`으로 렌더링한다.

| 요소 | 스타일 |
|------|--------|
| h1~h3 | 크기/굵기 구분 |
| p | `mb-2`, `leading-relaxed` |
| ul/ol | disc/decimal, `list-inside` |
| inline code | `--accent` 색상, monospace |
| code block | `--bg-primary` 배경, 가로 스크롤 |
| blockquote | `--accent` 좌측 보더, italic |
| a | `--accent` 색상, underline |

user 메시지는 plain text (개행만 `whitespace-pre-wrap` 처리).

---

## 관련 파일

| 파일 | 역할 |
|------|------|
| `apps/desktop/src/App.tsx` | 레이아웃, chatWidth state, resize 핸들러 |
| `apps/desktop/src/components/layout/Sidebar.tsx` | 좌측 네비게이션 |
| `apps/desktop/src/components/layout/ChatPanel.tsx` | 채팅 UI, 마크다운 렌더링, resize handle |
| `apps/desktop/src/hooks/useChat.ts` | 세션 상태 관리, Tauri IPC, 스트리밍 |
| `apps/desktop/sidecar/claude-bridge.mjs` | Node.js 사이드카, Claude SDK 연동 |
| `crates/agent/` | Rust 사이드카 관리, 세션 DB |
