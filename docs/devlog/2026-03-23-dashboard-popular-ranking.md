---
title: "Dashboard 인기 문서 랭킹 기능 구현"
aliases:
  - dashboard-popular-ranking
  - 대시보드 랭킹
  - 인기 문서
created: "2026-03-23"
updated: "2026-03-23"
tags:
  - devlog
  - feature
  - dashboard
  - search
---

<!-- docsmith: auto-generated 2026-03-23 -->

# Dashboard 인기 문서 랭킹 기능 구현

## 배경

Dashboard 탭과 Projects 탭의 역할이 중복되어 Dashboard의 독자적인 가치가 불분명했다. Dashboard를 "현황 요약 + 인기 문서 랭킹" 화면으로 개선하여 프로젝트 활동 현황을 한눈에 파악할 수 있도록 재설계했다.

## 변경 내용

### 주요 변경사항

- `PopularDoc` struct 추가: id, file_path, title, project_id, project_name, view_count, backlink_count, score, last_modified 필드 포함
- `TopProject` struct 추가: id, name, activity 필드 포함
- `get_popular_documents(pool, project_id: Option, limit)` 함수 구현: score = view_count * 0.6 + backlink_count * 0.4, LEFT JOIN 서브쿼리 방식으로 JOIN 곱 방지
- `get_top_projects(pool, limit)` 함수 구현: 프로젝트별 활동량(view + backlink 합) 기준 상위 N개 반환
- `record_view_by_path(pool, project_id, file_path)` 함수 구현: project_id + file_path로 document_id 조회 후 record_view 호출 (fire-and-forget)
- Tauri 커맨드 `get_popular_documents`, `get_top_projects` 등록
- `DashboardView.tsx` 전면 재설계: 3개 통계 카드 + 탭형 랭킹 리스트 구성
- 랭킹 탭 구성: [전체] [Top A] [Top B] [▾ 더보기 드롭다운]
- 1~3위 accent 색상 뱃지, 스켈레톤 로딩 5행 적용
- `App.tsx`에서 `Promise.all` 병렬 로딩 (전체 + top2 프로젝트 + 프로젝트별 랭킹)
- 문서 제목 클릭 시 검색 탭 이동 + 미리보기 패널 오픈 (onViewDocument 콜백)
- ExternalLink 아이콘 버튼으로 Obsidian에서 열기 동작 분리
- `overflow-visible` 처리로 드롭다운 클리핑 문제 해결

### 영향 범위

- `crates/core/src/search.rs`: PopularDoc, TopProject struct 및 관련 함수 추가
- `apps/desktop/src-tauri/src/main.rs`: Tauri 커맨드 등록
- `apps/desktop/src/types/index.ts`: 타입 정의 추가
- `apps/desktop/src/components/views/DashboardView.tsx`: 전면 재설계
- `apps/desktop/src/App.tsx`: 병렬 로딩 및 탭 전환 로직 수정

## 결과

Dashboard가 단순 프로젝트 목록 중복 화면에서 벗어나 인기 문서 랭킹과 프로젝트 활동 현황을 제공하는 독립적인 화면으로 기능한다. 문서 미리보기 연동으로 Dashboard에서 바로 콘텐츠 확인이 가능해졌다.

## 교훈

- SQL JOIN 시 다대다 관계에서 JOIN 곱이 발생할 수 있으므로 LEFT JOIN 서브쿼리 방식으로 집계 후 조인해야 한다.
- Tauri 컴포넌트에서 드롭다운이 부모 컨테이너에 클리핑되는 경우 `overflow-visible` 설정을 컨테이너 전체 체인에서 확인해야 한다.
- `handleTabChange`에서 탭 전환 시 미리보기 상태를 무조건 닫으면 연동 흐름이 끊기므로 예외 케이스를 명시적으로 처리해야 한다.

## 관련 문서

- [[search-system]]
- [[module-map]]
