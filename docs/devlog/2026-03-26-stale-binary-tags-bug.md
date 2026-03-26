---
title: "트러블슈팅 — Stale MCP 바이너리로 인한 tags 필터 무응답"
aliases:
  - stale-binary-tags-bug
  - 스테일 바이너리 버그
  - mcp-tags-filter-bug
  - tags-empty-result-bug
  - stale-binary-troubleshooting
created: "2026-03-26"
updated: "2026-03-26"
tags:
  - troubleshooting
  - devlog
  - mcp
  - bugfix
  - binary
---

<!-- docsmith: auto-generated 2026-03-26 -->

# 트러블슈팅 — Stale MCP 바이너리로 인한 tags 필터 무응답

## 증상

`nexus_search(tags=["mcp"])` 호출 시 항상 빈 배열 반환.

같은 쿼리에서 `tags` 파라미터를 제거하면 정상 결과가 반환됐다. tags 필터 적용 여부에 따라 결과가 완전히 달라지는 상황이었다.

## 진단 과정

### 1단계 — 파싱 로직 의심

`as_array()` 파싱 실패로 tags 값이 빈 배열로 해석되는 것을 의심했다. 코드를 확인했으나 파싱 로직에는 이상이 없었다.

### 2단계 — DB 직접 확인

`document_tags` 테이블을 직접 조회했다.

```
document_tags: 143개 태그, 365개 entries 존재
```

DB는 정상이었다. 인덱싱 누락이나 태그 저장 실패가 아니었다.

### 3단계 — 바이너리 버전 확인 (실제 원인)

`~/.local/bin/nexus-mcp-server` 바이너리의 빌드 날짜를 확인했다.

```
빌드 날짜: 2026-03-20
```

최근 tags 파라미터 처리 개선이 반영되기 이전 버전이었다. Claude Code가 실행하는 MCP 서버는 소스 코드가 아닌 설치된 바이너리를 사용하므로, 소스 수정이 반영되지 않은 것이었다.

## 원인

`~/.local/bin/nexus-mcp-server`가 2026-03-20 빌드된 스테일(stale) 버전으로, tags 파라미터 처리 개선이 포함되지 않은 버전이 설치된 상태였다.

## 해결

```bash
# 바이너리 재빌드 및 교체
cargo build --release
cp target/release/nexus-mcp-server ~/.local/bin/nexus-mcp-server

# 실행 중인 MCP 서버 재기동
pkill -f nexus-mcp-server
```

Claude Code 재연결 후 tags 필터 정상 동작을 확인했다.

## 검증

`nexus_search(tags=["mcp"])` 재실행 후 정상 결과 반환 확인.

## 교훈

- MCP 도구가 이상 동작할 때 **코드보다 바이너리 버전을 먼저 확인**해야 한다. DB, 파싱 로직보다 배포 상태가 먼저다.
- `~/.local/bin/` 경로에 설치된 바이너리는 소스 코드와 별개로 관리된다. 소스를 수정해도 바이너리를 교체하지 않으면 반영되지 않는다.
- 개발 중 소스를 자주 변경하는 시기에는 MCP 서버 바이너리를 빌드 후 즉시 교체하는 루틴을 만들어두는 것이 좋다.

## 관련 문서

- [[mcp-tools]]
- [[getting-started]]
- [[search-system]]
