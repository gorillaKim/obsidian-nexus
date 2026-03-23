---
title: "CLI 감지 오탐 및 CPU 폭증 버그 수정"
aliases:
  - cli-detection-cpu-runaway-fix
  - cli-detection-troubleshooting
  - CLI 감지 오탐 수정
  - CPU 폭증 버그
tags:
  - devlog
  - troubleshooting
  - bugfix
  - bug
created: "2026-03-23"
updated: "2026-03-23"
---

<!-- docsmith: auto-generated 2026-03-23 -->

# CLI 감지 오탐 및 CPU 폭증 버그 수정

## 배경

데스크탑 앱 설정 페이지에서 Gemini CLI와 Claude CLI가 "미설치" 또는 "로그인 필요"로 잘못 표시되는 문제가 발생했다. 동시에 앱 실행 후 CPU 사용률이 81%까지 치솟는 심각한 성능 문제도 확인됐다. 세 문제 모두 `crates/agent/src/cli_detector.rs`의 CLI 감지 로직에 집중되어 있었다.

## 변경 내용

### 문제 1: Gemini CLI "미설치" 오탐

**증상**: nvm으로 설치한 Gemini CLI가 터미널에서는 정상 동작하는데, 데스크탑 앱에서 "미설치"로 표시.

**원인**: Tauri GUI 앱은 쉘 PATH를 상속하지 않는다. `which gemini`는 `/opt/homebrew/bin/gemini`(broken shim)를 반환하고, 실제 바이너리는 `~/.nvm/versions/node/v20.20.0/bin/gemini`에 있다. nvm 바이너리의 shebang(`#!/usr/bin/env node`)이 실행 시 `node`를 PATH에서 찾는데, `~/.nvm/versions/node/v20.20.0/bin/`이 PATH에 없어서 `bad interpreter` 오류로 실패한다.

**진단 데이터**:
- `which` 결과: `/opt/homebrew/bin/gemini` (broken shim)
- 직접 spawn: `No such file or directory`
- shell 실행: exit 127, `bad interpreter: /opt/homebrew/opt/node/bin/node: no such file or directory`
- 실제 설치 위치: `/Users/madup/.nvm/versions/node/v20.20.0/bin/gemini`

**해결책**: `get_cli_version` 실행 시 바이너리의 부모 디렉토리를 PATH 앞에 추가한다.

```rust
let enriched_path = format!("{}:{}", parent.display(), current_path);
Command::new(path).args(args).env("PATH", enriched_path)
```

shebang의 `#!/usr/bin/env node`가 같은 디렉토리에 있는 `node`를 찾게 되어 정상 실행된다.

---

### 문제 2: Claude "로그인 필요" 오탐

**증상**: Claude CLI가 설치되어 있고 인증도 된 상태인데 "로그인 필요"로 표시.

**원인**: `check_claude_auth()`가 `~/.claude/.credentials.json` 파일 존재 여부로 인증 체크를 했는데, 최신 Claude Code는 자격증명을 시스템 keychain에 저장하므로 해당 파일이 존재하지 않는다.

**해결책**: 파일이 없으면 인증된 것으로 가정(true 반환). keychain 기반 인증은 파일로 확인 불가능하므로 파일 부재를 "미인증"으로 판단하지 않는다.

---

### 문제 3: node 프로세스 CPU 폭증 (가장 심각)

**증상**: 데스크탑 앱 실행 후 수십~수백 개의 `node --no-warnings=DEP0040 .../gemini --version` 프로세스가 좀비로 남아 CPU 81% 점유.

**원인 체인**:

1. `try_which`에서 `which -a gemini` 결과의 각 후보를 `spawn()` 후 즉시 `kill()`하는 spawn 체크 로직 존재
2. `gemini`는 shell script이므로 실행 시 자식 `node` 프로세스를 생성함
3. shell script에 `kill()`을 보내면 shell은 죽지만 자식 `node`는 살아남음 (고아 프로세스)
4. `system_status`가 UI 새로고침마다 호출되어 매번 고아 node 프로세스가 누적 생성됨
5. 쉘 명령에 타임아웃이 없어 `.zshrc` 로딩 중 블로킹 발생 → "상태 확인 중" 무한 대기

**해결책**:

1. **spawn 체크 완전 제거**: `try_which`에서 spawn test 로직 삭제. bad interpreter 처리는 `get_cli_version`의 PATH 보강으로 처리.

2. **`command_output_timeout` 도입**: 모든 쉘 명령 호출에 타임아웃 적용.

```rust
fn command_output_timeout(mut cmd: Command, timeout: Duration) -> Option<Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // reap zombie
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}
```

3. **타임아웃 기준**: `which -a` → 5초, version check → 5초, diagnose → 3초

### 주요 변경사항

- `try_which`의 spawn test 로직 제거
- `get_cli_version`에서 바이너리 부모 디렉토리 PATH 보강 추가
- `command_output_timeout` 함수 신설 및 모든 쉘 명령에 적용
- `check_claude_auth()`에서 파일 부재를 인증됨으로 처리

### 영향 범위

- `crates/agent/src/cli_detector.rs` — 모든 수정 집중
- `apps/desktop/src-tauri/src/main.rs` — `system_status`, `diagnose_cli` 커맨드
- `apps/desktop/src/components/views/SettingsView.tsx` — 진단 UI (변경 없음, 동작 정상화)

## 결과

- Gemini CLI 설치 상태가 정상적으로 "설치됨"으로 표시됨
- Claude 인증 상태 오탐 해소
- 고아 node 프로세스 누적 문제 해결, CPU 사용률 정상화
- UI 새로고침 시 "상태 확인 중" 무한 대기 해소

## 2026-03-23 추가 수정: test_cli 테스트 버튼 실패

### 문제

설정 페이지 테스트 버튼(🧪)을 눌렀을 때 Gemini가 "실행 오류: No such file or directory (os error 2)" 반환.
감지(detect)는 성공(v0.33.0 표시)했지만, 실행 테스트는 여전히 실패하는 상황.

### 원인

`test_cli` 핸들러(`apps/desktop/src-tauri/src/main.rs`)가 경로 해석에
`find_sidecar()` → `$SHELL -l -c "which gemini"` 순서를 사용했다.
`which gemini`가 `/opt/homebrew/bin/gemini`(broken shebang)을 반환하고,
이를 직접 실행하면 커널이 shebang의 interpreter(`/opt/homebrew/opt/node/bin/node`)를 찾지 못해
ENOENT(os error 2)를 반환한다.

`is_executable_script` 필터가 `cli_detector.rs`에만 적용되어 있고
`test_cli`의 경로 해석 코드에는 적용되지 않았던 것이 root cause.

### 해결

`test_cli`에서 `which` 대신 `cli_detector::find_cli_path_pub()`을 사용하도록 변경.
`find_cli_path_pub`은 내부적으로 `is_executable_script` 필터를 통과한 경로만 반환하므로
broken shebang 경로가 자동으로 걸러진다.

```rust
// 변경 전: which 명령어로 경로 해석 (is_executable_script 미적용)
let binary_path = find_sidecar(&cli)
    .or_else(|| { /* which gemini → /opt/homebrew/bin/gemini */ })
    .unwrap_or(cli.clone());

// 변경 후: cli_detector의 필터링된 경로 사용
let binary_path = find_sidecar(&cli)
    .or_else(|| {
        cli_detector::find_cli_path_pub(&cli)
            .map(|p| p.to_string_lossy().to_string())
    })
    .unwrap_or(cli.clone());
```

### 교훈

**감지(detect)와 테스트(test) 코드가 서로 다른 경로 해석 로직을 사용하면 불일치가 생긴다.**
경로 해석은 반드시 `find_cli_path_pub` 한 곳에서 일관되게 처리해야 한다.

## 교훈

**shell script를 `kill()`해도 자식 프로세스는 살아남는다.**
프로세스 그룹 kill(`killpg`)을 사용하거나, CLI 감지에서 spawn test 자체를 하지 않아야 한다. GUI 앱에서 CLI 감지는 "path 존재 확인 + 실제 실행(with timeout)" 방식이 안전하다.

**GUI 앱은 쉘 PATH를 상속하지 않는다.**
nvm, pyenv 등 쉘 init script에서 PATH를 조작하는 버전 관리자는 GUI 앱에서 감지 실패할 수 있다. 바이너리 실행 시 해당 디렉토리를 PATH에 직접 추가하는 방어 로직이 필요하다.

**`$SHELL -l -c` 호출은 반드시 타임아웃이 있어야 한다.**
`.zshrc` 로딩, nvm 초기화 등으로 수 초가 걸릴 수 있다. 타임아웃 없이 UI 갱신 루프에서 호출하면 UI 블로킹과 프로세스 누적이 동시에 발생한다.

**인증 상태 확인은 파일 의존도를 낮춰야 한다.**
CLI 도구의 자격증명 저장 방식은 버전마다 바뀔 수 있다(파일 → keychain). 파일 부재를 "미인증"으로 단정하면 오탐이 발생한다.

## 관련 문서

- [[2026-03-23-mcp-path-mismatch-fix]]
- [[2026-03-19]]
