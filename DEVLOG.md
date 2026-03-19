# Obsidian Nexus — 개발 로그

개발 중 발생한 문제점 및 해결 내용을 기록합니다.

---

## 검색 개선 (2026-03-19)

### 18. Alias Fallback 검색
- **문제:** "데이터독"으로 검색 시 결과 0건. FTS5는 chunk 본문만 검색하므로 한글 alias로는 영문 본문 매칭 불가
- **원인:** `document_aliases` 테이블에 alias가 저장되어 있지만 `fts_search()`에서 참조하지 않음
- **해결:** `fts_search()`에 alias fallback 추가 — FTS5 결과가 limit 미만일 때 `document_aliases` LIKE 검색으로 보충
- **교훈:** FTS5 토크나이저(unicode61)는 크로스 언어 매칭에 한계. alias는 이를 보완하는 핵심 메커니즘

### 19. 태그 필터 AND/OR 모드
- **기능:** `filter_by_tags()`에 `match_all` 파라미터 추가
- **OR(기본):** 태그 중 하나라도 매칭 → 결과 포함
- **AND:** 모든 태그가 매칭되는 결과만 포함
- **적용:** MCP(`tag_match_all` 파라미터), Desktop(`tag_match_all` IPC 파라미터)

---

## Phase 1: 스캐폴딩 + 파일 시스템 (2026-03-18)

### 1. Rust 툴체인 미설치
- **문제:** 개발 환경에 Rust가 설치되어 있지 않음
- **해결:** `rustup`으로 stable 1.94.0 설치
- **교훈:** 프로젝트 README에 필수 도구 목록 명시 필요

### 2. Cargo workspace + pnpm 이중 모노레포 구성
- **결정:** `Cargo.toml` workspace + `pnpm-workspace.yaml` 동시 사용
- **구조:** `crates/` (Rust), `apps/` (Tauri+React), `packages/` (공유 TS)
- **workspace.dependencies** 활용으로 버전 중앙 관리

### 3. unused import 경고
- **문제:** `std::path::Path` import가 사용되지 않는 곳에 포함
- **해결:** 불필요한 import 제거
- **교훈:** `cargo clippy` 를 CI에 포함하여 조기 발견

---

## Phase 2: DB + 인덱싱 (2026-03-18)

### 4. Rust 라이프타임 에러 (rusqlite Statement)
- **문제:** `stmt.query_map(...)?.collect()?`를 if-else 블록에서 사용 시 `stmt does not live long enough` 에러 발생
- **원인:** `query_map`이 반환하는 iterator가 `stmt`를 빌려오는데, if-else의 각 블록에서 `stmt`가 블록 끝에서 drop됨
- **해결:** 각 if/else 분기 내에서 `let docs = stmt.query_map(...)?.collect()?; Ok(docs)` 패턴으로 변경하여 결과를 즉시 수집
- **교훈:** rusqlite의 `MappedRows`는 lazy iterator이므로, `.collect()` 전에 `Statement`가 살아있어야 함

### 5. FTS5 검색 결과 빈 배열 (프로젝트 필터)
- **문제:** `nexus search "rust" --project test-vault` 가 빈 결과 반환
- **원인 1:** 프로젝트 필터 없는 SQL에서 `LIMIT ?3`으로 되어있었으나, 파라미터는 2개만 전달 (`?1`=query, `?2`=limit)
- **원인 2:** CLI에서 프로젝트 이름("test-vault")을 project_id로 직접 넘김, 하지만 DB의 `d.project_id`는 UUID
- **해결 1:** 프로젝트 필터 없는 SQL의 LIMIT을 `?2`로 수정
- **해결 2:** CLI search에서 `get_project(name)` → `proj.id` 변환 추가
- **교훈:** FTS5 external content table에서 파라미터 바인딩 인덱스 주의

### 6. FTS5 external content 테이블 + 트리거
- **결정:** `content=chunks, content_rowid=rowid` 방식 사용
- **주의:** TEXT PRIMARY KEY 테이블도 내부 rowid를 갖지만, 명시적 INTEGER PRIMARY KEY가 아니므로 트리거에서 `new.rowid` 사용
- **동작 확인:** INSERT 트리거가 자동으로 FTS5 인덱스를 동기화함

---

## 구현 완료 현황

| 기능 | 상태 | 비고 |
|------|------|------|
| Cargo workspace 모노레포 | ✅ | pnpm workspace 포함 |
| crates/core (config, error, db) | ✅ | thiserror, rusqlite+WAL |
| SQLite 마이그레이션 (V1) | ✅ | FTS5 + 트리거 포함 |
| 마크다운 파서 + 청킹 | ✅ | pulldown-cmark, 헤딩 기반 분할 |
| 프론트매터 추출 | ✅ | serde_yaml, 태그 추출 |
| 증분 인덱싱 (UoW) | ✅ | content_hash 비교, indexing_status checkpoint |
| CLI: project (add/list/remove/info/update) | ✅ | clap derive |
| CLI: index (증분/전체/전프로젝트) | ✅ | |
| CLI: search (FTS5, 프로젝트 필터) | ✅ | snippet + heading_path |
| CLI: doc (get/meta/list) | ✅ | 태그 필터 지원 |
| crates/mcp-server (스텁) | ✅ | 구조만 |
| 유닛 테스트 | ✅ | 9개 통과 |
| E2E 테스트 | ✅ | 프로젝트 추가→인덱싱→검색→문서목록 |

---

## 코드 리뷰 및 수정 (2026-03-18)

Opus 코드 리뷰어를 통해 14개 이슈 발견, 전량 수정 완료.

### CRITICAL 수정
| # | 이슈 | 수정 내용 |
|---|------|-----------|
| 1 | index_engine.rs에 SQLite 트랜잭션 미사용 | `conn.transaction()` + `tx.commit()`으로 UoW 원자성 보장 |
| 2 | 멀티바이트 문자 슬라이싱 패닉 (한국어 등) | `String::len()` (바이트) → `.chars().collect()` (문자 단위) 슬라이싱으로 변경 |

### MAJOR 수정
| # | 이슈 | 수정 내용 |
|---|------|-----------|
| 3 | PRAGMA가 풀의 첫 커넥션에만 적용 | `SqliteConnectionManager::with_init()`으로 모든 커넥션에 적용 |
| 4 | FTS5 검색 쿼리 미 sanitize | 쿼리를 `""`로 감싸 phrase 검색으로 처리, 특수 연산자 주입 방지 |
| 5 | projects.name UNIQUE 제약 누락 | migration SQL에 `UNIQUE` 추가 |
| 6 | spawn_blocking 패닉 전파 | `.expect()` → `NexusError::Indexing`으로 변환 |

### MINOR 수정
| # | 이슈 | 수정 내용 |
|---|------|-----------|
| 7 | 중복 hash 함수 | `indexer::compute_hash`를 pub으로 변경, `index_engine`의 중복 함수 삭제 |

### 2차 코드 리뷰 (2026-03-18)
1차 수정 후 재리뷰 진행. **1차 CRITICAL/MAJOR 7건 모두 수정 확인됨.** 신규 CRITICAL 0건.

| 심각도 | 이슈 | 수정 |
|--------|------|------|
| HIGH | 마이그레이션 트랜잭션 누락 | `conn.transaction()` + `tx.commit()`으로 감싸기 |
| MEDIUM | 빈 쿼리 시 FTS5 에러 | 빈 쿼리 가드 추가 (`query.trim().is_empty()`) |
| MEDIUM | last_indexed_at 무조건 갱신 | 에러 0건일 때만 타임스탬프 갱신 |
| MEDIUM | chunk_size 바이트/문자 혼동 | `text.chars().count()` 문자 단위로 변경 |
| MEDIUM | config expect() 패닉 | 향후 개선 (현재 CLI 전용이라 허용) |
| MEDIUM | 에러 매핑 광범위 | 향후 개선 (error code 기반 분기) |

**판정: 조건부 GO → 수정 후 GO 승격**

### 미수정 (SUGGESTION, 향후 개선)
- `indexing_status` 문자열 → Rust enum 전환
- `print_output` text 포맷 구현
- `Config::data_dir()` expect → Result 반환
- trait 기반 DB 추상화 (DocumentStore, SearchEngine)
- 테스트 커버리지 확대 (index_engine, search, project CRUD)

---

---

## Phase 4: MCP 서버 (2026-03-18)

### 7. MCP 서버 구현
- **구현:** `crates/mcp-server/src/main.rs` — stdin/stdout JSON-RPC 2.0
- **6개 tool:** `nexus_search`, `nexus_list_projects`, `nexus_get_document`, `nexus_get_metadata`, `nexus_list_documents`, `nexus_index_project`
- **프로토콜:** `initialize` → `tools/list` → `tools/call` 핸들러
- **E2E 검증:** initialize 응답, 6 tools 등록, search 2 hits 확인

---

## Phase 5: Tauri GUI (2026-03-18)

### 8. Tauri v2 아이콘 문제
- **문제 1:** `frontendDist: "../dist"` 경로가 존재하지 않음
- **해결:** placeholder `dist/index.html` 생성
- **문제 2:** `icon.png is not RGBA` — RGB PNG를 생성했으나 Tauri는 RGBA 필요
- **해결:** Python으로 RGBA PNG 생성 (color type 6)
- **교훈:** Tauri v2는 모든 아이콘이 RGBA PNG여야 함. `cargo tauri icon` 명령어로 자동 생성 권장

### 9. Tauri IPC 커맨드
- **구현:** `apps/desktop/src-tauri/src/main.rs`
- **4개 IPC:** `list_projects`, `search_documents`, `index_project`, `get_document`
- **AppState:** `DbPool`을 Tauri State로 관리

### 10. React UI
- **구현:** `apps/desktop/src/App.tsx`
- **기능:** 검색 탭 (프로젝트 필터, FTS5 검색, 결과 snippet), 프로젝트 탭 (목록, 인덱싱 버튼)
- **스타일:** Tailwind CSS, 다크 테마 (Tokyo Night 컬러)
- **옵시디언 딥링크:** 검색 결과 클릭 시 `obsidian://open` URI 호출

---

## 전체 구현 현황

| Phase | 기능 | 상태 |
|-------|------|------|
| 1 | Cargo workspace 모노레포 | ✅ |
| 1 | crates/core (config, error, db, indexer) | ✅ |
| 1 | crates/cli (project, index, search, doc) | ✅ |
| 2 | SQLite + FTS5 + 마이그레이션 | ✅ |
| 2 | 마크다운 파서 + 청킹 (UoW) | ✅ |
| 2 | 증분 인덱싱 + 트랜잭션 | ✅ |
| 4 | MCP 서버 (6 tools, JSON-RPC) | ✅ |
| 5 | Tauri v2 + Vite + React | ✅ |
| 5 | Tauri IPC + 검색 UI | ✅ |
| — | 코드 리뷰 2회 + 수정 완료 | ✅ |

---

## Phase 3: 벡터 검색 (2026-03-18)

### 11. Ollama 임베딩 통합
- **결정:** ONNX Runtime 대신 Ollama 사용 — 구현 복잡도 1/5로 감소
- **모델:** `nomic-embed-text` (768차원, Ollama `/api/embeddings` 엔드포인트)
- **저장:** LanceDB 대신 SQLite BLOB (`chunk_embeddings` 테이블) + 코사인 유사도
- **이유:** LanceDB Rust SDK 성숙도 우려 해소, SQLite 단일 DB로 관리 단순화

### 12. 임베딩 모델명 불일치
- **문제:** config 기본값이 `multilingual-e5-small`인데 Ollama에는 `nomic-embed-text` 설치됨
- **해결:** 기본 모델을 `nomic-embed-text`, dimensions를 768로 변경

### 13. 하이브리드 검색 (Reciprocal Rank Fusion)
- **구현:** FTS5 키워드 결과 + 벡터 유사도 결과를 RRF로 합산
- **config.toml의 `hybrid_weight`** 값으로 벡터 비중 조절 (0.0=키워드만, 1.0=벡터만)
- **E2E 검증:** "memory safety" 벡터 검색 → score 0.756으로 정확한 문서 1위

### 14. `nexus setup` 자동 설치 명령어
- **구현:** 4단계 자동 셋업 (디렉토리 생성, Ollama 설치, 모델 풀, DB 초기화)
- **brew install ollama + ollama pull nomic-embed-text** 자동 실행

---

## 전체 구현 현황

| Phase | 기능 | 상태 |
|-------|------|------|
| 1 | Cargo workspace 모노레포 | ✅ |
| 1 | crates/core (config, error, db, indexer) | ✅ |
| 1 | crates/cli (project, index, search, doc, setup) | ✅ |
| 2 | SQLite + FTS5 + 마이그레이션 (V1+V2) | ✅ |
| 2 | 마크다운 파서 + 청킹 (UoW + 트랜잭션) | ✅ |
| 2 | 증분 인덱싱 + content_hash | ✅ |
| 3 | Ollama 임베딩 (nomic-embed-text, 768d) | ✅ |
| 3 | 벡터 검색 (SQLite BLOB + cosine similarity) | ✅ |
| 3 | 하이브리드 검색 (FTS5 + Vector, RRF) | ✅ |
| 4 | MCP 서버 (6 tools, JSON-RPC) | ✅ |
| 5 | Tauri v2 + Vite + React + Tailwind | ✅ |
| 5 | Tauri IPC + 검색 UI + 옵시디언 딥링크 | ✅ |
| — | nexus setup (자동 설치) | ✅ |
| — | 코드 리뷰 2회 + 수정 완료 | ✅ |
| — | 유닛 테스트 11개 통과 | ✅ |

---

## Phase 6: File Watcher + GUI 보강 + Auto-Update (2026-03-18)

### 15. File Watcher 구현
- **구현:** `crates/core/src/watcher.rs` + `crates/cli/src/commands/watch.rs`
- **기능:** `nexus watch [project_id]` — 단일 또는 전체 프로젝트 감시
- **디바운스:** 500ms (config.toml `watcher.debounce_ms`), 중복 이벤트 병합
- **이벤트 처리:** Create/Modify → 증분 인덱싱, Remove → 문서 인덱스 삭제
- **종료:** Ctrl+C (ctrlc 크레이트) 또는 mpsc 채널 stop signal
- **필터:** `.md`/`.markdown`만 처리, exclude_patterns 적용
- **의존성 추가:** `ctrlc = "3"` (CLI), `notify` (core, 기존)

### 16. Tauri GUI 보강
- **Dashboard 탭 추가:** 프로젝트/문서/청크 수 통계 카드, 프로젝트별 상세 현황
- **project_info IPC 추가:** `apps/desktop/src-tauri/src/main.rs` — 5번째 IPC 커맨드
- **Projects 탭 보강:** 인덱싱 상태 표시 (docs/chunks/pending), 인덱싱 버튼 로딩 상태
- **Search 탭 보강:** score 표시, 빈 상태 안내 메시지

### 17. 자동 업데이트 (tauri-plugin-updater)
- **구현:** `tauri-plugin-updater = "2"` 의존성 추가
- **설정:** `tauri.conf.json`에 `plugins.updater` 설정 (GitHub Releases 엔드포인트)
- **권한:** `capabilities/default.json`에 `updater:default` 추가
- **향후:** GitHub Releases에 서명 키 설정 + latest.json 배포 파이프라인 필요

---

## 전체 구현 현황 (최종)

| Phase | 기능 | 상태 |
|-------|------|------|
| 1 | Cargo workspace 모노레포 | ✅ |
| 1 | crates/core (config, error, db, indexer, watcher) | ✅ |
| 1 | crates/cli (project, index, search, doc, setup, watch) | ✅ |
| 2 | SQLite + FTS5 + 마이그레이션 (V1+V2) | ✅ |
| 2 | 마크다운 파서 + 청킹 (UoW + 트랜잭션) | ✅ |
| 2 | 증분 인덱싱 + content_hash | ✅ |
| 3 | Ollama 임베딩 (nomic-embed-text, 768d) | ✅ |
| 3 | 벡터 검색 (SQLite BLOB + cosine similarity) | ✅ |
| 3 | 하이브리드 검색 (FTS5 + Vector, RRF) | ✅ |
| 4 | MCP 서버 (6 tools, JSON-RPC) | ✅ |
| 5 | Tauri v2 + Vite + React + Tailwind | ✅ |
| 5 | Tauri IPC (5 commands) + Dashboard/Search/Projects UI | ✅ |
| 6 | File Watcher (notify + debounce) | ✅ |
| 6 | 자동 업데이트 (tauri-plugin-updater) | ✅ |
| — | nexus setup (자동 설치) | ✅ |
| — | 코드 리뷰 2회 + 수정 완료 | ✅ |
| — | 48 테스트 통과 | ✅ |

### 미구현 (향후)
- GitHub Releases 배포 파이프라인 (CI/CD)
- 서명 키 생성 + updater pubkey 설정
- OpenAI 임베딩 provider (trait 추상화로 추가 가능)
- SUGGESTION 이슈 5건 (enum, trait 추상화 등)
