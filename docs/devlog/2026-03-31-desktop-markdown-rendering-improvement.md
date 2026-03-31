---
title: "데스크톱 문서 뷰어 마크다운 렌더링 개선 (TDD)"
aliases:
  - markdown-rendering-devlog
  - 마크다운 렌더링 개선
  - desktop-markdown-improvement
tags:
  - devlog
  - feature
  - desktop
  - frontend
  - tdd
created: "2026-03-31"
updated: "2026-03-31"
---

<!-- docsmith: auto-generated 2026-03-31 -->

# 데스크톱 문서 뷰어 마크다운 렌더링 개선 (TDD)

## 배경

검색 페이지에서 문서를 열람할 때 세 가지 시각적 문제가 있었다:

1. **Frontmatter raw 표시** — `---` YAML 블록이 그대로 텍스트로 렌더링됨 (tags, created 등)
2. **불릿 미표시** — Tailwind CSS preflight가 `list-style-type: none`으로 리셋하여 `•` 기호가 보이지 않음
3. **코드 블록 하이라이팅 없음** — 언어 구분 없이 단색 코드블록으로 표시

## 변경 내용

### 주요 변경사항

#### `@tailwindcss/typography` 플러그인 도입

- 기존에 수동으로 작성하던 `.prose` CSS 규칙 전체 제거
- `@plugin "@tailwindcss/typography"` 한 줄로 대체 — 불릿, 번호, 헤딩, 테이블, 코드블록 등 자동 처리
- 프로젝트 색상 변수(`var(--text-primary)` 등)로 typography 테마 오버라이드

#### Frontmatter 카드 UI (`FrontmatterCard` 컴포넌트)

- 브라우저 호환 인라인 YAML 파서 구현 (`parseFrontmatter()`)
  - `key: value`, `key: [a, b, c]` 인라인 배열, `key:\n  - item` 블록 리스트 모두 처리
  - `Buffer`, `fs` 등 Node.js 전용 API 미사용
- `FrontmatterCard` 컴포넌트로 key-value를 카드 UI로 표시
  - `tags` 필드: 클릭 가능한 `#tag` 배지로 렌더링
  - 날짜 필드: `2026년 3월 31일` 형식으로 자동 포맷팅

#### 태그 클릭 → 검색 연동

- 태그 배지 클릭 시 `setTagFilter(tag)` + `handleSearch(tag)` 즉시 실행
- `handleSearch(tagOverride?)` 파라미터 추가로 React 상태 업데이트 딜레이 문제 해결
- 쿼리가 없을 때 태그 클릭 시 태그명을 쿼리로 사용하여 검색 가능

#### 코드 블록 구문 강조

- `react-syntax-highlighter` (Prism + `oneDark` 테마) 도입
- `react-markdown`의 `components.code` 오버라이드로 언어별 구문 강조

### TDD 흐름

1. `FrontmatterCard` 테스트 6개 선작성 (Red)
   - 빈 메타데이터 → null 렌더링
   - title만 → null 렌더링
   - tags 배열 → 클릭 가능한 배지
   - 태그 클릭 → onTagClick 호출
   - 날짜 필드 → 한국어 포맷팅
   - 일반 문자열 → 그대로 표시
2. `FrontmatterCard` 컴포넌트 구현으로 6/6 통과 (Green)
3. TypeScript 오류 0개, 빌드 성공 확인

### 트러블슈팅: gray-matter Buffer 에러

#### 현상
`gray-matter` 라이브러리 사용 시 Tauri WebView 환경에서 `ReferenceError: Can't find variable: Buffer` 발생, SearchView 전체가 렌더링 안 됨.

#### 원인
`gray-matter`는 내부적으로 Node.js `Buffer` API와 `eval()`을 사용하며 브라우저 환경에서 동작하지 않음.

#### 해결
외부 의존성 없이 브라우저 호환 인라인 파서(`parseFrontmatter()`)를 직접 구현. Obsidian frontmatter의 실제 사용 패턴(단순 key-value, 인라인 배열, 블록 리스트)을 커버하는 수준으로 충분함.

### 트러블슈팅: 태그 클릭 시 검색 미실행

#### 현상
태그 배지 클릭 후 검색 결과가 변하지 않음.

#### 원인
`setTagFilter(tag)` 후 바로 `handleSearch()` 호출 시 React 상태 업데이트가 아직 반영되지 않아 이전 `tagFilter` 값으로 검색됨.

#### 해결
`handleSearch(tagOverride?: string)` 파라미터 추가. 태그 클릭 시 `handleSearch(tag)`로 직접 값을 전달하여 상태 딜레이 우회.

## 결과

- `cargo test` 전체 통과
- `vitest run` 6/6 통과
- `pnpm build` 성공 (299ms)
- Tauri DMG 빌드 성공 (`Obsidian Nexus_0.5.9_aarch64.dmg`)
- Frontmatter가 카드 UI로 이쁘게 표시됨
- 태그 클릭 → 즉시 검색 실행
- 불릿/번호 리스트 정상 표시
- 코드 블록 구문 강조 적용

## 영향 범위

- `apps/desktop/src/components/FrontmatterCard.tsx` — 신규 컴포넌트
- `apps/desktop/src/components/FrontmatterCard.test.tsx` — 신규 테스트
- `apps/desktop/src/components/views/SearchView.tsx` — frontmatter 파싱, 코드 하이라이팅 통합
- `apps/desktop/src/hooks/useSearch.ts` — `handleSearch(tagOverride?)` 파라미터 추가
- `apps/desktop/src/index.css` — Typography 플러그인 도입, 수동 prose 규칙 제거
- `apps/desktop/vite.config.ts` — vitest 설정 추가
- `apps/desktop/tsconfig.json` — 테스트 파일 빌드 제외

## 교훈

- Node.js 전용 라이브러리(`gray-matter`, `js-yaml`)는 Tauri WebView 환경에서 동작하지 않는다. 브라우저 호환성을 먼저 확인하거나 인라인 구현을 택한다.
- React `setState` 직후 같은 렌더 사이클에서 상태를 읽으면 이전 값이 반환된다. 함수 파라미터로 값을 직접 전달하거나 `useEffect`로 처리해야 한다.
- Tailwind CSS preflight는 `list-style-type`을 리셋한다. 마크다운 렌더러에는 `@tailwindcss/typography` 플러그인을 사용하는 것이 수동 CSS보다 훨씬 안전하다.

## 관련 문서

- [[development]]
- [[frontend]]
