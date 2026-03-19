# Obsidian Nexus — 작업 현황

## 완료된 기능

### Core Engine
- [x] 마크다운 파싱 + 청킹 (pulldown-cmark)
- [x] FTS5 전문 검색 (unicode61 토크나이저, 한국어 지원)
- [x] sqlite-vec 벡터 KNN 검색 (nomic-embed-text 768차원)
- [x] 하이브리드 검색 (RRF 합산, 동적 가중치)
- [x] 임베딩 정규화 (L2 ≈ 코사인)
- [x] 증분 인덱싱 (content hash 비교)
- [x] Ghost document 정리 (삭제된 파일 감지)
- [x] 위키링크 `[[...]]` 추출 + 백링크/포워드링크
- [x] 인라인 `#태그` 추출 (코드 블록 제외)
- [x] Aliases (frontmatter aliases 필드)
- [x] 메타데이터 리랭킹 (backlink, view_count, title boost)
- [x] 짧은 쿼리 프리픽스 매칭 + 언더스코어 토큰 분리
- [x] 태그 기반 검색 필터링
- [x] 섹션 단위 문서 읽기 (heading 기반)
- [x] 문서 조회 카운팅 (인기도 추적)
- [x] 경로 탈출 방어 (path traversal guard)

### MCP 서버 (11개 도구)
- [x] nexus_search (하이브리드/키워드/벡터, enrich, tags, popularity)
- [x] nexus_list_projects
- [x] nexus_get_document (자동 view_count 기록)
- [x] nexus_get_metadata
- [x] nexus_list_documents (태그 필터)
- [x] nexus_index_project
- [x] nexus_get_section (heading 기반)
- [x] nexus_get_backlinks
- [x] nexus_get_links
- [x] nexus_resolve_alias
- [x] nexus_sync_config (on-config.json 동기화)

### 데스크톱 앱 (Tauri v2)
- [x] 대시보드 (프로젝트 통계)
- [x] 검색 UI (모드 토글, 태그 필터, 설정 패널)
- [x] 프로젝트 관리 (추가/인덱싱/동기화/삭제)
- [x] 문서 뷰어 (마크다운 렌더링)
- [x] Obsidian URI 딥링크 (vault://open)
- [x] 볼트 자동 감지 (.obsidian 폴더 탐색)
- [x] on-config.json 프로젝트 설정
- [x] Collapsible 검색 결과 + 폴더 트리
- [x] 파일 트리 사이드바

### CLI
- [x] nexus project add/list/remove
- [x] nexus index (증분/전체)
- [x] nexus search (keyword/vector/hybrid)
- [x] nexus watch (파일 감시)
- [x] nexus doc get/meta

### 문서
- [x] docs/ Obsidian 볼트 (7개 문서, 위키링크, 태그)

---

## 향후 작업

### 검색 고도화
- [ ] 복합 쿼리 (태그+날짜+키워드 조합 API)
- [ ] 검색 결과 정렬 옵션 (관련도순/최신순/인기순)
- [ ] 그래프 기반 검색 부스트 (백링크 허브 탐색)

### 쓰기 기능
- [ ] nexus_create_document (문서 생성)
- [ ] nexus_append_to_document (내용 추가)
- [ ] nexus_update_section (섹션 편집)

### 품질 개선
- [ ] App.tsx 컴포넌트 분리 (SearchTab, ProjectsTab 등)
- [ ] 프론트엔드 에러 UI (toast 알림)
- [ ] Windows/Linux 지원 (Obsidian 경로, open 명령)
- [ ] CLI 태그 필터 검색

### 성능
- [ ] 임베딩 배치 API 또는 async 전환
- [ ] 청크 단위 해시로 변경분만 재임베딩

### 인프라
- [ ] GitHub Actions CI/CD
- [ ] Tauri 서명 키 + 자동 업데이트
- [ ] 크로스 플랫폼 빌드
