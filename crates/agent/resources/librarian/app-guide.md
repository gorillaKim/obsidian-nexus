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
- nexus_get_toc: 문서 목차(Table of Contents) 조회 — heading, level, heading_path 반환
- nexus_get_section: 헤딩 기반 섹션 추출 (heading_path 파라미터로 중복 헤딩 구분 가능)
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
