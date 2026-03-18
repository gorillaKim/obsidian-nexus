# Obsidian Nexus — 남은 작업

## Phase 3: 벡터 검색 (다음 우선순위)

### 3-1. ONNX 모델 다운로드 매니저
- [ ] `crates/core/src/embedding.rs` — `EmbeddingProvider` trait 정의
- [ ] `reqwest`로 HuggingFace에서 `multilingual-e5-small` ONNX 모델 다운로드
- [ ] `~/.nexus/models/` 에 저장, SHA-256 체크섬 검증
- [ ] 다운로드 진행률 표시 (CLI: stderr 프로그레스)

### 3-2. ort (ONNX Runtime) 임베딩 통합
- [ ] `OnnxEmbedder` struct — `EmbeddingProvider` trait 구현
- [ ] 텍스트 → 384차원 벡터 변환
- [ ] `tokio::sync::Semaphore`로 동시 추론 수 제한
- [ ] 인덱싱 시에만 ONNX 세션 로드, 유휴 시 해제

### 3-3. LanceDB 저장/검색
- [ ] `crates/core/src/db/lance.rs` 구현
- [ ] 프로젝트별 디렉토리 격리: `~/.nexus/lance/{project_id}/`
- [ ] chunk_id + embedding 저장
- [ ] 벡터 유사도 검색 (cosine similarity)

### 3-4. 하이브리드 검색
- [ ] FTS5 (BM25) + Vector 결과를 Re-ranking
- [ ] `config.toml`의 `hybrid_weight` 적용
- [ ] `nexus search --mode vector|keyword|hybrid` 옵션

---

## Phase 4: MCP 서버

### 4-1. stdin/stdout JSON-RPC 구현
- [ ] `crates/mcp-server/src/main.rs` — JSON-RPC 2.0 프로토콜
- [ ] MCP `initialize`, `tools/list`, `tools/call` 핸들러
- [ ] 6개 tool: `nexus_search`, `nexus_list_projects`, `nexus_get_document`, `nexus_get_metadata`, `nexus_list_documents`, `nexus_index_project`

### 4-2. 에이전트 연동 테스트
- [ ] Claude Code `settings.json`에 MCP 서버 등록
- [ ] 실제 에이전트에서 검색 테스트

---

## Phase 5: Tauri GUI

### 5-1. Tauri v2 프로젝트 셋업
- [ ] `apps/desktop/` — Vite + React + TypeScript
- [ ] Tailwind CSS + Shadcn UI 설치
- [ ] `apps/desktop/src-tauri/` — Tauri Rust 코드 (nexus-core 의존)

### 5-2. Tauri IPC 커맨드
- [ ] `project_list`, `project_add`, `project_remove` IPC
- [ ] `index_project`, `search`, `get_document` IPC
- [ ] File Watcher 토글 IPC

### 5-3. UI 구현
- [ ] 대시보드 (프로젝트별 문서 현황)
- [ ] 검색 UI (결과 미리보기, 하이라이팅)
- [ ] 옵시디언 딥링크 버튼 (`obsidian://open?vault=...&file=...`)
- [ ] File Watcher on/off 토글

---

## Phase 6 (선택): 고급 기능
- [ ] Tantivy 도입 (FTS5 부족 시)
- [ ] OpenAI 임베딩 provider
- [ ] 프로젝트 간 크로스 검색 최적화
- [ ] `nexus watch` 데몬 (PID 파일, graceful shutdown)
- [ ] `nexus backup` 명령어
