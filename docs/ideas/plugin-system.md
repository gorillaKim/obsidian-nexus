---
title: "아이디어: IPC Sidecar 기반 플러그인 시스템"
aliases:
  - plugin-system
  - 플러그인 시스템
  - external-source-integration
tags:
  - ideas
  - plugin
  - architecture
  - confluence
  - github
created: "2026-03-30"
updated: "2026-03-30"
status: draft
---

# 아이디어: IPC Sidecar 기반 플러그인 시스템

> **배경**: 현재 로컬 Obsidian 볼트만 인덱싱 가능. Confluence, GitHub 등 외부 문서 소스 연동을 플러그인 방식으로 지원하고자 한다.

---

## 핵심 설계 원칙

### "IPC Sidecar + DB 직행 + Lazy 열람"

```
[플러그인 = 독립 프로세스]

nexus-core
  └─ PluginManager
        └─ spawn("confluence-plugin")
               stdin  → JSON-RPC 요청
               stdout ← NDJSON 스트리밍 응답 (문서 하나씩)
                         → index_engine 직접 호출 → DB 저장

[검색 시]   SQLite 조회만 (프로세스 호출 없음)
[열람 시]   fresh → DB chunks 재조합
            stale → 플러그인 프로세스 재호출 (fetch_one)
```

### dylib 대신 IPC Sidecar를 선택한 이유

| 기준 | dylib | IPC Sidecar |
|------|-------|-------------|
| 서드파티 안전성 | 위험 (같은 메모리) | OS 프로세스 격리 |
| 플러그인 언어 | Rust만 | Python, Go, Node 뭐든 |
| 팀 친숙도 | 중간 | MCP 서버와 동일 패턴 |
| 서드파티 DX | 어려움 | JSON 읽고 쓰면 끝 |

---

## 플러그인 통신 프로토콜 (JSON-RPC over stdio)

### 요청 (Host → Plugin stdin)

```json
{"jsonrpc": "2.0", "id": 1, "method": "handshake",     "params": {"host_version": "0.6.0", "protocol_version": 1}}
{"jsonrpc": "2.0", "id": 2, "method": "validate_auth", "params": {"config": {...}, "secrets": {...}}}
{"jsonrpc": "2.0", "id": 3, "method": "fetch_all",     "params": {"config": {...}, "secrets": {...}}}
{"jsonrpc": "2.0", "id": 4, "method": "fetch_since",   "params": {"config": {...}, "secrets": {...}, "since": 1743000000, "cursor": "abc123"}}
{"jsonrpc": "2.0", "id": 5, "method": "fetch_one",     "params": {"config": {...}, "secrets": {...}, "doc_id": "page-123"}}
{"jsonrpc": "2.0", "id": 6, "method": "oauth_start",    "params": {"config": {...}}}
{"jsonrpc": "2.0", "id": 7, "method": "oauth_exchange", "params": {"config": {...}, "code": "...", "state": "..."}}
```

### 응답 (Plugin stdout → Host)

**handshake 응답 — 버전 교환:**
```json
{
  "jsonrpc": "2.0", "id": 1,
  "result": {
    "plugin_version": "0.1.0",
    "protocol_version": 1,
    "min_host_version": "0.5.0",
    "methods": ["fetch_all", "fetch_since", "fetch_one"]
  }
}
```

> **호스트 버전 의존성**: 플러그인은 `min_host_version`으로 동작 가능한 최소 호스트 버전을 선언한다. 호스트가 해당 버전 미만이면 플러그인 실행을 거부하고 UI에 "호스트 업데이트 필요" 표시. `protocol_version` 불일치 시에도 동일하게 실행 거부.

**fetch_all / fetch_since — 스트리밍:**
```jsonl
{"jsonrpc":"2.0","method":"progress","params":{"current":1,"total":100,"title":"Engineering Overview"}}
{"jsonrpc":"2.0","method":"document","params":{"id":"page-1","title":"Engineering Overview","content_md":"# ...","source_url":"https://...","last_modified":1743000000,"tags":["engineering"]}}
{"jsonrpc":"2.0","id":3,"result":{"synced_count":100,"deleted_ids":["page-old-1"],"next_cursor":"def456"}}
```

**에러 타입:**
```json
{"id":3, "error": {"code": -32001, "message": "AuthExpired"}}
{"id":3, "error": {"code": -32002, "message": "RateLimited", "data": {"retry_after_secs": 60}}}
{"id":3, "error": {"code": -32003, "message": "HostVersionTooOld", "data": {"required": "0.6.0", "current": "0.5.0"}}}
```

---

## 호스트 버전 의존성 관리

플러그인은 동작에 필요한 최소 호스트 버전을 명시한다. 이는 플러그인 SDK API가 버전마다 달라질 수 있기 때문이다.

### 버전 선언 위치

**1. `plugin.json` 매니페스트 (설치 전 검증용):**
```json
{
  "id": "confluence",
  "version": "0.1.0",
  "min_host_version": "0.5.0",
  "protocol_version": 1
}
```

**2. handshake 응답 (런타임 검증용):**
```json
{"result": {"min_host_version": "0.5.0", "protocol_version": 1}}
```

### 호환성 검증 흐름

```
[설치 시]
레지스트리에서 plugin.json 다운로드
→ min_host_version ≤ 현재 호스트 버전? → 설치 허용
→ 아니면 "이 플러그인은 Obsidian Nexus 0.6.0 이상이 필요합니다" 표시

[실행 시]
handshake 요청 → min_host_version, protocol_version 재검증
→ 불일치 시 플러그인 프로세스 종료 + UI 경고
```

### 버전 정책

| 변경 종류 | protocol_version 변경 | 예시 |
|----------|----------------------|------|
| 하위 호환 메서드 추가 | 유지 | fetch_one 추가 |
| 기존 메서드 파라미터 변경 | +1 | secrets 필드 구조 변경 |
| 핵심 동작 변경 | +1 | 문서 ID 포맷 변경 |

---

## 신선도(Freshness) 정책

React Query의 staleTime 개념. 플러그인 소스별 설정.

| 상태 | 판단 기준 | 동작 |
|------|----------|------|
| fresh | last_synced_at + stale_time > 현재 | DB chunks 바로 표시 |
| stale | last_synced_at + stale_time ≤ 현재 | fetch_one 호출 후 갱신 |
| 오프라인 | 네트워크 없음 | DB 표시 + "오프라인" 경고 |

### 문서 열람 시 재인덱싱 흐름

```
fetch_one 응답의 last_modified
  ↓
DB remote_last_modified와 비교
  ↓ 다르면
재인덱싱 → UI에 알림 표시

ℹ️ 원본이 업데이트되어 최신 내용을 가져왔습니다. (2026-03-30 → 2026-03-31)
```

---

## 인증(Auth) 설계

### 인증 타입

| auth_type | UI | 처리 |
|-----------|-----|------|
| `"none"` | 폼 없음 | 바로 소스 추가 |
| `"api_token"` | config_schema 폼 | Keychain 저장 |
| `"personal_token"` | config_schema 폼 | Keychain 저장 |
| `"oauth2"` | 로그인 버튼 | Redirect flow |

### Credentials 저장 원칙

```
비밀 아닌 설정 (base_url, space_key 등) → DB config_json (평문)
secrets (api_token, PAT 등)            → OS Keychain
  service: "obsidian-nexus"
  key:     "{plugin_id}/{source_id}/{field_key}"
```

**플러그인은 secrets를 저장하지 않는다.** 호스트가 Keychain에서 읽어 매 요청마다 전달.

### OAuth2 Redirect Flow

```
[로그인] 클릭
→ oauth_start → auth_url + state
→ 시스템 브라우저 열기
→ obsidian-nexus://plugins/{plugin_id}/callback?code=...&state=...
→ Tauri deep link 수신 → state 검증
→ oauth_exchange → access_token → Keychain 저장
```

### Token 만료 처리

```json
{"error": {"code": -32001, "message": "AuthExpired"}}
```
→ PluginManager가 `plugin-auth-required:{source_id}` 이벤트 발행 → UI 재인증 팝업

---

## DB Schema 변경 (V7)

```sql
-- 설치된 플러그인
CREATE TABLE plugins (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    version      TEXT NOT NULL,
    enabled      INTEGER NOT NULL DEFAULT 1,
    installed_at TEXT NOT NULL
);

-- 플러그인 소스 인스턴스
CREATE TABLE plugin_sources (
    id                  TEXT PRIMARY KEY,
    plugin_id           TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    project_id          TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    display_name        TEXT NOT NULL,
    config_json         TEXT,                    -- 비밀 아닌 설정만
    stale_time_secs     INTEGER NOT NULL DEFAULT 3600,
    sync_schedule       TEXT,                    -- cron 표현식, null이면 수동만
    missed_sync_policy  TEXT NOT NULL DEFAULT 'run_immediately',
    is_syncing          INTEGER NOT NULL DEFAULT 0,
    last_synced_at      TEXT,
    UNIQUE(plugin_id, project_id)
);

-- documents 테이블 확장
ALTER TABLE documents ADD COLUMN external_id         TEXT;
ALTER TABLE documents ADD COLUMN source_url          TEXT;
ALTER TABLE documents ADD COLUMN plugin_source_id    TEXT REFERENCES plugin_sources(id);
ALTER TABLE documents ADD COLUMN remote_last_modified INTEGER;  -- unix timestamp

-- sync 이력
CREATE TABLE sync_history (
    id            TEXT PRIMARY KEY,
    source_id     TEXT NOT NULL REFERENCES plugin_sources(id) ON DELETE CASCADE,
    started_at    TEXT NOT NULL,
    finished_at   TEXT,
    synced_count  INTEGER,
    deleted_count INTEGER,
    next_cursor   TEXT,    -- 다음 fetch_since에 전달할 cursor/etag
    error_msg     TEXT
);
```

---

## 정기 인덱싱 스케줄

- 앱 실행 중: tokio 백그라운드 타이머 (`cron` 크레이트)
- 앱 종료 중: macOS launchd / Linux systemd timer 자동 등록
- 소스 추가/삭제 시 OS 스케줄러 자동 갱신/제거
- UI: 소스 설정 패널에 드롭다운 (매시간 / 매일 / 매주 / cron 직접 입력)

---

## 설정 공유 (데스크톱 ↔ CLI ↔ MCP)

세 컴포넌트가 `~/.nexus/nexus.db`와 OS Keychain을 공유하므로 데스크톱에서 설치/설정한 플러그인이 CLI와 MCP에 **자동으로 적용**된다.

```bash
# 데스크톱에서 Confluence 연동 후 CLI/MCP에서 바로 사용
obs-nexus plugin sync confluence-source-id
obs-nexus search "Engineering 배포 가이드"
```

---

## 퍼스트파티 플러그인

| 플러그인 | 특징 | auth_type |
|---------|------|----------|
| `plugin-local-vault` ⭐ | 디폴트, 삭제 불가, 기존 indexer 래핑 | `none` |
| `plugin-confluence` | Confluence REST API v2, HTML→MD 변환 | `api_token` |
| `plugin-github` | GitHub Contents/Trees API | `personal_token` |

---

## Desktop UI

**Plugins 탭 (신규):**
```
┌─────────────────────────────────────────────┐
│  🔍 [플러그인 검색창                          ]│
│                                             │
│  ── 설치된 플러그인 (2) ──────────────────── │
│  [📁 Local Vault] [기본] 활성              │
│  [🔷 Confluence]        활성 · 1개 소스    │
│                                             │
│  ── 설치 가능한 플러그인 ──────────────────── │
│  [GitHub]   GitHub 레포 .md 파일 동기화  [설치]│
└─────────────────────────────────────────────┘
```

**Projects 탭 확장:**
- 출처 배지: `[📁 로컬]` `[🔷 Confluence]` `[🐙 GitHub]`
- 외부 소스에 Sync Now + 다음 sync 예정 시각

**외부 문서 뷰어:**
- `[📡 Confluence · 읽기 전용]` 배지
- 편집 버튼 비활성화

---

## 주요 리스크 & 완화

| 리스크 | 완화 방법 |
|--------|----------|
| 플러그인 패닉/비정상 종료 | 에러 감지 + SIGTERM→SIGKILL 타임아웃 |
| 좀비 프로세스 | 앱 종료 시 모든 플러그인 프로세스 정리 |
| 중복 실행 | `is_syncing` 플래그 DB Lock |
| 호스트 버전 불일치 | handshake 단계에서 검증, 불일치 시 실행 거부 |
| Rate Limit | `RateLimited + retry_after_secs` → 스케줄 자동 연기 |
| Token 만료 | `AuthExpired` → UI 재인증 팝업 |
| 고스트 문서 | `deleted_ids`로 즉시 DB 삭제 |
| Missed Sync | `missed_sync_policy`로 동작 선택 |
| 플러그인 내부 오류 파악 | stderr 캡처 → Plugin Log UI |
| 바이너리 신뢰성 | 설치 단계 코드서명 검증 |
| 플랫폼별 배포 | CI에서 크로스컴파일 자동화 |

---

## 구현 순서

1. **Phase 1**: V7 DB 마이그레이션 + `plugin_manager.rs` + `plugin-sdk` 기반
2. **Phase 2**: `plugin-local-vault` (디폴트) → E2E 검증
3. **Phase 3**: `plugin-confluence` + `plugin-github`
4. **Phase 4**: Desktop UI (Plugins 탭, Projects 탭 확장)
5. **Phase 5**: 레지스트리 & CI 자동 빌드

---

## 관련 문서

- [[binary-management-improvement]] — 바이너리 관리 개선 아이디어
- [[architecture/search-system]] — 현재 검색 시스템 구조
