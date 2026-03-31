---
title: 개발 가이드
tags:
  - development
  - guide
aliases:
  - Development Guide
  - 개발 환경
---

# 개발 가이드

## 빌드

```bash
# 전체 빌드
cargo build

# 릴리즈 빌드
cargo build --release

# 특정 크레이트만
cargo build -p nexus-core
cargo build -p nexus-mcp-server
```

## 테스트

```bash
# 전체 테스트 (unit + integration)
cargo test -p nexus-core

# 특정 테스트
cargo test -p nexus-core --lib indexer::tests::test_extract_wiki_links
```

현재 64개 테스트 (54 unit + 10 integration).

### 테스트 구조

- `crates/core/src/` 각 모듈 내 `#[cfg(test)] mod tests`
- `crates/core/tests/integration_test.rs` — 전체 파이프라인 통합 테스트
- `crates/core/src/test_helpers.rs` — 인메모리 DB 풀 + 테스트 볼트 생성

### 테스트 DB

테스트는 인메모리 SQLite를 사용. `test_pool()`이 모든 마이그레이션(V1~V5) + sqlite-vec 확장을 자동 적용.

## 프론트엔드 테스트 (vitest)

```bash
# 전체 프론트엔드 테스트
cd apps/desktop && npx vitest run

# 특정 컴포넌트 테스트
npx vitest run src/components/FrontmatterCard.test.tsx
```

- 테스트 환경: jsdom
- 설정: `apps/desktop/vite.config.ts` (`test.environment`, `test.globals`)
- 셋업: `apps/desktop/src/test-setup.ts` (`@testing-library/jest-dom` 임포트)
- 테스트 파일은 `tsconfig.json`의 `exclude`에 포함되어 프로덕션 빌드에서 제외됨

## MCP 서버 테스트

```bash
# 빌드
cargo build --release -p nexus-mcp-server

# 수동 테스트 (JSON-RPC via stdin)
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"nexus_search","arguments":{"query":"test"}}}' \
| ./target/release/nexus-mcp-server
```

## CLI 사용

```bash
# 프로젝트 추가
./target/release/nexus project add "My Vault" /path/to/vault

# 인덱싱
./target/release/nexus index "My Vault"

# 검색
./target/release/nexus search "검색어" --mode hybrid

# 파일 감시
./target/release/nexus watch

# 온보딩 (다른 프로젝트에 사서 스킬/에이전트 설치)
./target/release/nexus onboard /path/to/project
./target/release/nexus onboard /path/to/project --force  # 기존 파일 덮어쓰기
```

## 주요 의존성

| 크레이트 | 용도 |
|----------|------|
| rusqlite (bundled) | SQLite 바인딩 |
| sqlite-vec | 벡터 KNN 검색 확장 |
| pulldown-cmark | 마크다운 파싱 |
| reqwest (blocking) | Ollama HTTP API |
| r2d2 | 커넥션 풀 |
| regex | 위키링크/인라인태그 추출 |
| tauri v2 | 데스크톱 앱 프레임워크 |

## 관련 문서

- [[00-프로젝트-개요]]
- [[01-아키텍처]]
- [[04-데이터베이스-스키마]]
