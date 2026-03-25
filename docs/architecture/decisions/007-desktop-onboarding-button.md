---
title: "데스크톱 앱 Claude CLI 온보딩 버튼 추가"
aliases:
  - desktop onboarding button
  - onboarding adr
  - 온보딩 버튼 ADR
  - 클로드 CLI 온보딩 설계
tags:
  - decision
  - desktop
  - onboarding
  - architecture
created: 2026-03-25
updated: 2026-03-25
---

<!-- docsmith: auto-generated 2026-03-25 -->

# 데스크톱 앱 Claude CLI 온보딩 버튼 추가

데스크톱 앱 설정 페이지에 "Claude CLI 온보딩" 카드를 추가하고, `onboard.rs` 로직을 리팩토링하여 `.mcp.json`, `~/.claude/settings.json`, `CLAUDE.md` 3개 파일을 단계별로 생성/병합하는 온보딩 흐름을 도입하기로 한 결정을 기록합니다.

## 상태

채택 (Accepted)

## 배경

Obsidian Nexus를 처음 사용하는 개발자가 Claude Code와 연동하려면 다음 세 가지 설정을 수동으로 해야 했습니다.

1. `.mcp.json` — 프로젝트에 Nexus MCP 서버 등록
2. `~/.claude/settings.json` — Nexus 도구 12종에 대한 권한 허용
3. 프로젝트 `CLAUDE.md` — Claude에게 Nexus 도구 우선 사용 힌트 제공

각 파일의 경로, 형식, 병합 규칙을 숙지하지 않으면 초기 설정 과정에서 오류가 자주 발생했습니다. 또한 기존 `onboard.rs`는 librarian agent/skill/frontmatter 3개 템플릿을 생성하는 방식이었으나, 이 기능은 마켓플레이스를 통해 별도 제공하기로 결정되면서 제거 대상이 되었습니다.

## 검토한 대안

### 방법 A: CLI `obs-nexus onboard` 명령만 유지

기존처럼 CLI에서만 온보딩 명령을 제공합니다.

**문제**: 데스크톱 앱 사용자는 터미널을 열어야 하는 부담이 있습니다. 설정 완료 여부를 앱 내에서 확인할 수 없습니다.

### 방법 B: 앱 최초 실행 시 자동 온보딩 위저드

앱을 처음 열면 온보딩 플로우를 강제합니다.

**문제**: 이미 설정이 완료된 사용자에게 불필요한 UX를 강요합니다. 테스트와 재실행이 불편합니다.

### 방법 C: 설정 페이지 온보딩 카드 — 채택

설정 페이지에 "Claude CLI 온보딩" 카드를 추가하고, 프로젝트 경로를 입력받아 온보딩을 명시적으로 실행합니다. 단계별 결과(created/skipped/error)를 UI에 표시합니다.

**장점**:
- 이미 설정된 환경에서는 자동으로 skip 처리되어 멱등성 보장
- 단계별 결과 가시화로 문제 파악 용이
- 기존 handleTest/handleUpdate UI 패턴 재사용으로 일관된 UX

## 결정

방법 C를 채택합니다. `onboard.rs`를 `Vec<OnboardStep>` 기반으로 리팩토링하고, Tauri 커맨드 `run_onboard`를 추가하며, `SettingsView.tsx`에 온보딩 카드를 구현합니다.

## 구현

### onboard.rs 변경

`crates/core/src/onboard.rs`에 `OnboardStep` / `StepStatus` 타입을 도입합니다.

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct OnboardStep {
    pub name: String,        // ".mcp.json" | "settings.json" | "CLAUDE.md"
    pub status: StepStatus,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus { Created, Skipped, Error }
```

`onboard()` 함수는 3단계를 순차 실행하며, 각 단계 실패 시 `StepStatus::Error`로 기록하고 다음 단계를 계속 진행합니다.

| 단계 | 파일 | 기존 파일이 있을 때 |
|------|------|---------------------|
| 1 | `.mcp.json` | nexus 서버 엔트리 병합 (기존 동작 유지) |
| 2 | `~/.claude/settings.json` | `permissions.allow` 배열에 nexus 도구 12종 병합 |
| 3 | `CLAUDE.md` | `## Obsidian Nexus - 문서 탐색 도구 우선순위` 헤딩 없으면 EOF에 append |

`force=true` 플래그 시 각 단계는 기존 내용을 덮어씁니다.

### 제거 대상 (librarian 관련)

마켓플레이스 제공 예정으로 아래 3개 템플릿 및 관련 단계를 `onboard.rs`에서 제거합니다.

- `crates/core/src/templates/librarian_agent.md`
- `crates/core/src/templates/librarian_skill.md`
- `crates/core/src/templates/frontmatter_guide.md`

### 신규 템플릿

`crates/core/src/templates/claude_md_section.md` — CLAUDE.md에 append할 Nexus 검색 힌트 섹션. 한/영 키워드 변환 팁("데이터독" ↔ "datadog")과 도구별 사용 기준 표를 포함합니다.

### Tauri 커맨드

`apps/desktop/src-tauri/src/main.rs`에 `run_onboard` 커맨드를 등록합니다.

```rust
#[tauri::command]
async fn run_onboard(project_path: String)
    -> Result<Vec<nexus_core::onboard::OnboardStep>, String>
```

### SettingsView UI

`apps/desktop/src/components/views/SettingsView.tsx`에 카드를 추가합니다.

- 프로젝트 경로 텍스트 입력 + "온보딩 시작" 버튼
- 결과: `CheckCircle` (Created, green) / `AlertCircle` (Skipped, yellow) / `XCircle` (Error, red)
- 완료 후 "Claude Code 세션을 재시작하면 적용됩니다" 안내
- 경로가 비어있거나 실행 중이면 버튼 disabled

## 결과

- 터미널 없이 데스크톱 앱에서 Claude CLI 연동 설정 완료 가능
- 기존 설정 파일 내용 보존 (병합/append 방식)
- 온보딩 멱등성 보장 (재실행 시 이미 설정된 항목은 skipped)
- CLI `obs-nexus onboard`와 동일한 core 로직 공유

## 한계 및 후속 과제

- `~/.claude/settings.json` 권한 설정은 Claude Code의 스키마 변경에 취약 — 주기적 검토 필요
- `force=true` UI는 이번 구현에 포함되지 않음 — 추후 고급 옵션으로 노출 가능
- librarian agent/skill 온보딩은 마켓플레이스 플러그인으로 분리 예정

## 관련 문서

- [[guides/getting-started]]
- [[integrations/mcp-tools]]
- [[architecture/architecture]]
- [[006-llm-query-rewriting]]
