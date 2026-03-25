---
title: "Claude CLI 온보딩 버튼 구현 (v0.5.2)"
aliases:
  - onboarding-button
  - claude-cli-onboard
  - 온보딩 버튼
created: "2026-03-25"
updated: "2026-03-25"
tags:
  - devlog
  - feature
  - onboarding
---

<!-- docsmith: auto-generated 2026-03-25 -->

# Claude CLI 온보딩 버튼 구현 (v0.5.2)

## 배경

신규 사용자가 Obsidian Nexus를 프로젝트에 연동하려면 `.claude/settings.json` 도구 권한 등록과 `CLAUDE.md` 가이드 섹션 추가를 수동으로 해야 했다. 반복 작업인 데다 실수가 잦아 온보딩 자동화 필요성이 제기됐다.

계획 문서(`.omc/plans/onboarding-button.md`)를 기반으로 TDD 방식으로 구현했다.

## 변경 내용

### 1. `crates/core/src/onboard.rs` 리팩토링

반환 타입을 `OnboardResult { created, skipped }` 단일 구조체에서 `Vec<OnboardStep>` 으로 변경했다. 각 단계를 독립적으로 추적할 수 있어 UI 표시와 에러 핸들링이 명확해졌다.

신규 타입:

```rust
pub struct OnboardStep {
    pub name: String,
    pub status: StepStatus,
    pub message: String,
}

pub enum StepStatus {
    Created,   // "created"
    Skipped,   // "skipped"
    Error,     // "error"
}
```

`StepStatus`는 `serde rename_all = "lowercase"` 적용으로 JSON 직렬화 시 소문자로 내려간다.

단계별 동작:

- `setup_settings_json()`: 프로젝트 `.claude/settings.json`에 nexus 12개 도구 권한 등록. 기존 파일 병합(idempotent).
- `setup_claude_md()`: 루트 `CLAUDE.md`에 Nexus 검색 가이드 섹션 append. 헤딩 존재 여부로 중복 삽입 방지(idempotent).

기존 librarian agent / skill / frontmatter 3단계는 마켓플레이스로 제공되므로 삭제했다.

### 2. 템플릿 변경

| 파일 | 변경 |
|------|------|
| `crates/core/src/templates/claude_md_section.md` | 신규 생성 |
| `librarian_agent.md` | 삭제 |
| `librarian_skill.md` | 삭제 |
| `frontmatter_guide.md` | 삭제 |

### 3. `crates/cli/src/commands/onboard.rs`

`result.created / skipped` 필드 접근 방식을 `Vec<OnboardStep>` 이터레이션으로 교체했다. Next steps 안내 메시지도 함께 업데이트했다.

### 4. `apps/desktop/src-tauri/src/main.rs`

Tauri 커맨드 추가:

```rust
pub async fn run_onboard(project_path: String) -> Result<Vec<OnboardStep>, String>
```

### 5. `apps/desktop/src/components/views/SettingsView.tsx`

온보딩 Card UI를 추가했다.

- `OnboardStep` interface 정의
- 경로 input + 폴더 선택 버튼 + 단계별 결과 표시
- `@tauri-apps/plugin-dialog`의 `open()`으로 네이티브 폴더 선택 다이얼로그 연동

### 영향 범위

- `crates/core/src/onboard.rs`: 핵심 로직 리팩토링
- `crates/core/src/templates/`: 템플릿 파일 교체
- `crates/cli/src/commands/onboard.rs`: CLI 출력 업데이트
- `apps/desktop/src-tauri/src/main.rs`: Tauri 커맨드 등록
- `apps/desktop/src/components/views/SettingsView.tsx`: 온보딩 UI 추가

## 설계 결정

**settings.json 경로**: 초기 계획은 글로벌 `~/.claude/settings.json`이었으나 프로젝트별 격리를 위해 프로젝트 `.claude/settings.json`으로 변경했다. 글로벌 파일을 건드리면 다른 프로젝트의 설정과 충돌할 위험이 있다.

**CLAUDE.md 위치**: Claude Code는 `.claude/CLAUDE.md`를 읽지 않으므로 루트 `CLAUDE.md`를 유지했다.

**에러 처리 전략**: 각 단계 실패 시 `StepStatus::Error`를 반환하고 다음 단계를 계속 진행하는 resilient 방식을 채택했다. 한 단계 실패가 전체 온보딩을 중단시키지 않는다.

## 테스트

- 신규 단위 테스트 12개 추가 (`crates/core/src/onboard.rs` 내 `#[cfg(test)]`)
- 전체 테스트 101개 통과
- `cargo clippy` 경고 0개
- `cargo tauri build` 성공

## 교훈

- 반환 타입을 단순 구조체 대신 `Vec<Step>` 패턴으로 설계하면 UI 렌더링과 에러 추적 모두 자연스럽게 확장된다.
- idempotent 설계(헤딩 존재 여부 확인, 파일 병합)는 온보딩처럼 반복 실행 가능성이 있는 기능에서 필수다.
- 글로벌 설정 파일 수정은 부작용 범위가 넓으므로 프로젝트 스코프로 좁히는 것이 안전하다.

## 관련 문서

- [[getting-started]]
- [[module-map]]
- [[검색 품질 개선 — LLM Query Rewriting & Alias 토큰화]]
