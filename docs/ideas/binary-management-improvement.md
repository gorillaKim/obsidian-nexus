---
title: "개선 계획: MCP 바이너리 3중 관리 구조 단순화"
aliases:
  - binary-management-improvement
  - 바이너리 관리 개선
  - mcp-binary-simplification
tags:
  - ideas
  - improvement
  - deployment
  - mcp
  - dx
created: "2026-03-27"
updated: "2026-03-27"
---

# 개선 계획: MCP 바이너리 3중 관리 구조 단순화

> **배경**: 냉정한 자기 평가에서 "바이너리 3중 관리가 개인 사용자에게도 고통"이라는 진단이 나왔다.
> 관련 사고 기록: [[stale-binary-tags-bug]], [[mcp-binary-path-confusion]], [[2026-03-27-graph-query-tools]]

---

## 문제 정의

### 현재 바이너리가 사는 곳 (3곳)

| 위치 | 갱신 방법 | 용도 |
|------|----------|------|
| `/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server` | 앱 업데이트 버튼 / DMG 재설치 | 데스크톱 앱 사이드카 |
| `~/.local/bin/nexus-mcp-server` | `nexus onboard` 재실행 or 수동 복사 | Claude Code MCP 등록 경로 |
| `target/release/nexus-mcp-server` | `cargo build --release` | 로컬 개발 빌드 |

### 실제로 발생한 사고들

1. **stale-binary-tags-bug (2026-03-26)**
   - 소스 수정 후 `~/.local/bin` 미교체
   - tags 필터가 무응답 → DB와 파싱 로직을 먼저 의심, 삽질 후 바이너리 문제 발견

2. **mcp-binary-path-confusion (2026-03-26)**
   - 앱 업데이트 버튼으로 사이드카만 교체
   - Claude Code가 바라보는 `~/.local/bin`은 구버전 그대로
   - `nexus_get_toc` 추가했는데 "도구가 없다"는 응답

3. **graph-query-tools devlog (2026-03-27)**
   - 그래프 도구 3종 구현 후 데스크톱 앱에서 미노출
   - `cargo build --release`만 하면 앱에 반영 안 된다는 걸 또 다시 실수

### 왜 문제인가

```
개발자(나)조차 같은 실수를 반복하고 있다.
일반 사용자라면 "MCP 도구가 이상해요" 하고 이탈한다.
```

---

## 목표

> **Claude Code가 항상 최신 바이너리를 바라보도록, 사용자 개입 없이.**

- 앱 업데이트 한 번 → 모든 경로 자동 갱신
- `cargo build` 후 별도 복사 작업 불필요
- "어떤 바이너리 보고 있는지" 확인 명령어 없어도 됨

---

## 개선 방안

### 방안 A. 단일 경로 정책 — 앱 번들만 (현재 방향)

`nexus_onboard`가 MCP 경로를 **항상 앱 번들**로 등록하도록 강제.

```json
{
  "mcpServers": {
    "nexus": {
      "command": "/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server"
    }
  }
}
```

- ✅ 앱 업데이트 버튼 한 번으로 Claude Code까지 갱신
- ✅ `~/.local/bin` 관리 불필요 → 경로 소멸
- ❌ 앱이 설치되어 있지 않은 CLI 전용 사용자는 별도 처리 필요
- ❌ macOS 경로 하드코딩 (Windows/Linux 지원 시 재작업)

**현재 `nexus_onboard`가 이미 `current_exe()` 기반 경로를 사용하므로, 앱에서 onboard 실행 시 자동으로 올바른 경로가 등록됨. 문제는 앱 외부(CLI, cargo)에서 onboard를 실행했을 때.**

**단기 조치**: onboard 실행 시 앱 번들 존재 여부를 확인하고, 있으면 앱 번들 경로를 우선 등록.

```rust
// onboard.rs 개선 방향
let mcp_path = if app_bundle_exists() {
    "/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server"
} else {
    current_exe_path()  // CLI 전용 환경 fallback
};
```

---

### 방안 B. 심볼릭 링크 자동 갱신

앱 업데이트 시 `~/.local/bin/nexus-mcp-server`를 앱 번들 바이너리로 symlink 교체.

```bash
ln -sf "/Applications/Obsidian Nexus.app/Contents/MacOS/nexus-mcp-server" \
       ~/.local/bin/nexus-mcp-server
```

- ✅ 기존 `~/.local/bin` 경로를 MCP config에 유지해도 됨 (경로 변경 불필요)
- ✅ 앱 업데이트 시 Tauri의 post-update hook에서 자동 실행 가능
- ❌ symlink 관리도 결국 상태 하나 추가
- ❌ Tauri post-update hook 구현 필요

---

### 방안 C. MCP 서버를 앱 외부에서 완전히 분리 (장기)

CLI 설치 (`brew install obsidian-nexus`)에서 MCP 서버도 함께 설치.
앱과 CLI MCP 서버를 동일 바이너리로 통합, 버전 관리를 Homebrew에 위임.

```
brew upgrade obsidian-nexus → CLI + MCP 서버 동시 갱신
앱 업데이트 버튼 → 데스크톱 UI만 갱신
```

- ✅ 가장 깔끔한 분리
- ✅ Windows/Linux 지원 시 자연스럽게 확장
- ❌ 현재 앱 번들 내 사이드카 구조와 충돌 가능
- ❌ 구현 공수 가장 큼

---

## 권장 순서

```
단기 (지금 바로)
  → 방안 A: onboard 로직 개선
     앱 번들 있으면 앱 번들 경로 우선 등록

중기 (다음 마일스톤)
  → 방안 B: 앱 업데이트 시 symlink 자동 갱신
     post-update hook 구현

장기 (Windows/Linux 지원 시)
  → 방안 C: CLI/MCP 서버 완전 분리
     플랫폼 독립 배포 구조로 전환
```

---

## 부수 개선: 개발 중 빌드 루틴 문서화

현재 `.claude/rules/build.md`에 기록되어 있다고 하나, 실수가 반복됨.
빌드 스크립트로 자동화하는 것이 문서보다 확실하다.

```bash
# scripts/dev-install.sh (신규 제안)
#!/bin/bash
# 개발 중 빌드 + 모든 경로 동시 교체

set -e
cargo build --release --target aarch64-apple-darwin \
  -p nexus-mcp-server -p nexus-cli

# 사이드카 교체
cp target/aarch64-apple-darwin/release/nexus-mcp-server \
   apps/desktop/src-tauri/binaries/nexus-mcp-server-aarch64-apple-darwin

# ~/.local/bin 교체 (CLI 전용 환경용)
cp target/aarch64-apple-darwin/release/nexus-mcp-server \
   ~/.local/bin/nexus-mcp-server

echo "✅ 빌드 완료 및 모든 경로 교체됨"
echo "   → 데스크톱 앱: Tauri 재빌드 필요"
echo "   → Claude Code: MCP 재연결 필요 (세션 재시작)"
```

---

## 성공 기준

- [ ] 앱 업데이트 버튼 한 번 → Claude Code에서 새 도구 즉시 사용 가능
- [ ] `nexus onboard` 실행 시 항상 올바른 (최신) 바이너리 경로 등록
- [ ] 개발 중 "왜 새 도구가 없지?" 트러블슈팅 시간 제로
- [ ] devlog에 "stale 바이너리" 키워드 더 이상 등장 안 함

---

## 관련 문서

- [[stale-binary-tags-bug]] — 태그 필터 무응답 사고 기록
- [[mcp-binary-path-confusion]] — MCP 경로 혼선 사고 기록
- [[2026-03-27-graph-query-tools]] — 그래프 도구 사이드카 미갱신 사고
- [[guides/deployment|배포 및 버전 관리]]
- [[context/deployment-troubleshooting|배포 트러블슈팅 & 학습]]
- [[integrations/subagent-mcp-setup|서브에이전트 MCP 설정 가이드]]
