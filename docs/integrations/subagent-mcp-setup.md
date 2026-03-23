---
title: 서브에이전트 MCP 설정 가이드
tags:
  - guide
  - mcp
  - agent
aliases:
  - subagent MCP
  - 서브에이전트 MCP
  - mcpServers 설정
created: 2026-03-19
---

# 서브에이전트 MCP 설정 가이드

Claude Code 서브에이전트(`.claude/agents/*.md`)에서 MCP 서버를 네이티브 도구로 사용하는 방법.

## 핵심 요약

서브에이전트가 MCP 도구를 네이티브로 호출하려면 **두 가지 조건**이 모두 필요하다:

1. **`.mcp.json`에 MCP 서버 등록** → 세션 시작 시 서버가 활성화됨
2. **에이전트 프론트매터에 `mcpServers` 이름 참조** → 서브에이전트가 해당 도구를 사용 가능

## 자동 설정 (권장)

`nexus_onboard` MCP 도구를 사용하면 아래 수동 설정을 한 번에 자동으로 수행한다:

```json
nexus_onboard({ "project_path": "/path/to/my-project" })
```

`.mcp.json`, `.claude/agents/librarian.md`, `.claude/skills/librarian/SKILL.md`를 생성하고 세션 재시작만 하면 된다.

## 수동 설정

### Step 1: `.mcp.json` 등록

프로젝트 루트에 `.mcp.json` 파일을 생성한다:

```json
{
  "mcpServers": {
    "nexus": {
      "type": "stdio",
      "command": "/path/to/nexus-mcp-server",
      "args": []
    }
  }
}
```

### Step 2: 에이전트 프론트매터

`.claude/agents/my-agent.md`:

```yaml
---
name: my-agent
description: 설명
mcpServers:
  - nexus
---
```

이름 참조(`- nexus`)는 `.mcp.json` 또는 `settings.json`에 등록된 서버명과 일치해야 한다.

### Step 3: 세션 재시작

MCP 서버는 **세션 시작 시 로드**된다. `.mcp.json` 변경 후 반드시 세션을 재시작해야 한다.

## 도구 호출 형식

서브에이전트 내에서 MCP 도구는 `mcp__{서버명}__{도구명}` 패턴으로 호출된다:

```
mcp__nexus__nexus_search
mcp__nexus__nexus_list_projects
mcp__nexus__nexus_get_document
```

## 주의사항

### 인라인 정의 vs 이름 참조

| 방식 | 프론트매터 예시 | 동작 여부 |
|------|----------------|-----------|
| 이름 참조 | `- nexus` | `.mcp.json`에 등록 + 세션 활성화 시 동작 |
| 인라인 정의 | `- nexus: { type: stdio, ... }` | 공식 문서에는 지원으로 기재되나, 실제 테스트에서 동작하지 않음 (2026-03-19 기준) |

**결론**: 이름 참조 방식을 사용하고, `.mcp.json`에 서버를 등록하라.

### 플러그인 서브에이전트 제한

공식 문서에 따르면 **플러그인 서브에이전트에서는 `mcpServers` 필드가 무시**된다.
`.claude/agents/`에 직접 작성한 커스텀 서브에이전트에서만 동작한다.

### Bash 도구 제거

`mcpServers`로 네이티브 MCP 도구를 사용하면 `tools`에서 `Bash`를 제거할 수 있다.
JSON-RPC bash 우회가 불필요해지므로 보안과 토큰 효율이 개선된다.

## 관련 문서

- [[03-MCP-도구-레퍼런스]] — nexus_onboard 도구 포함
- [[05-설정-가이드]]
- [Claude Code 서브에이전트 공식 문서](https://code.claude.com/docs/en/sub-agents)
