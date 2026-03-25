---
title: "MCP 업데이트 asset 이름 불일치 수정"
aliases:
  - mcp-update-fix
  - nexus-cli-tarball
  - MCP 업데이트 수정
  - asset 이름 불일치
created: "2026-03-25"
updated: "2026-03-25"
tags:
  - devlog
  - troubleshooting
  - release
  - mcp
---

<!-- docsmith: auto-generated 2026-03-25 -->

# MCP 업데이트 asset 이름 불일치 수정

## 증상

설정 > Nexus 바이너리 > nexus-mcp-server 업데이트 버튼 클릭 시:

> "릴리즈에서 nexus-mcp-server-aarch64-apple-darwin 바이너리를 찾을 수 없습니다"

## 원인

GitHub Releases의 실제 asset 이름: `nexus-cli-darwin-aarch64.tar.gz` (tarball)
코드에서 찾던 이름: `nexus-mcp-server-aarch64-apple-darwin` (존재하지 않음)

tarball 내부에 `obs-nexus`와 `nexus-mcp-server` 두 바이너리가 함께 포함됨.

## 해결

1. asset 이름을 `nexus-cli-darwin-{arch}.tar.gz` 패턴으로 수정
2. tarball 다운로드 후 `tar -xzf`로 특정 바이너리 추출
3. 공통 헬퍼 `download_nexus_binary(name, dest)` 추출 → mcp/obs-nexus 양쪽에서 재사용
4. arch 매핑: `aarch64-apple-darwin` → `aarch64`, `x86_64-apple-darwin` → `x86_64`

## 교훈

릴리즈 asset 이름은 CI 워크플로우에서 결정되므로, 업데이트 기능 구현 전 실제 릴리즈 asset 목록을 확인할 것.

```bash
curl https://api.github.com/repos/.../releases/latest | jq '.assets[].name'
```

## 관련 문서

- [[2026-03-25-search-quality-improvement]]
- [[module-map]]
