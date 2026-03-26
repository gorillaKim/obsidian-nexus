---
title: "Tauri 서명키 관리 및 빌드 트러블슈팅"
aliases:
  - tauri-signing-key-troubleshooting
  - signing-key-troubleshooting
  - tauri 서명키
  - signing key
  - 빌드 트러블슈팅
created: "2026-03-26"
updated: "2026-03-26"
tags:
  - devlog
  - troubleshooting
  - tauri
  - build
  - mcp
---

<!-- docsmith: auto-generated 2026-03-26 -->

# Tauri 서명키 관리 및 빌드 트러블슈팅

## 문제 1: TAURI_SIGNING_PRIVATE_KEY_PASSWORD 분실

### 증상

```
failed to decode secret key: incorrect updater private key password: Device not configured
```

`pnpm tauri:build` 실행 시 위 에러가 발생. 기존 키(`~/.tauri/obsidian-nexus.key`)는 존재하나 암호화에 사용한 패스워드를 알 수 없는 상태.

### 해결: 키 재생성 (패스워드 없음)

`cargo tauri signer generate`는 TTY(인터랙티브 터미널)를 필요로 하므로 Claude Code 환경에서 직접 실행이 불가하다. `expect` 명령어로 빈 패스워드 입력을 자동화한다.

```bash
expect -c '
  spawn cargo tauri signer generate -w ~/.tauri/obsidian-nexus.key --force
  expect "Password:"; send "\r"
  expect "Password (one more time):"; send "\r"
  expect eof
'
```

**주의**: `-w` 경로를 상대경로로 지정하면 현재 작업 디렉토리 기준으로 파일이 생성된다. 반드시 절대경로를 사용할 것.

이번 세션에서 상대경로(`~/.tauri/obsidian-nexus.key`)가 쉘 확장 없이 그대로 처리되어 `/Users/madup/gorillaProject/obsidian-nexus/~/.tauri/` 하위에 생성되었다. 이 경우 파일을 수동으로 올바른 위치(`~/.tauri/`)로 이동해야 한다.

---

## 문제 2: .env에서 키 읽기 시 base64 패딩 깨짐

### 증상

```
failed to decode base64 secret key: Invalid padding
```

### 원인

`.env` 파일에 `TAURI_SIGNING_PRIVATE_KEY=<값>` 형식으로 저장 후 `cut -d= -f2`로 값을 추출하면 멀티라인 base64 값의 패딩이 깨진다.

### 해결

`.env` 파싱 대신 키 파일을 직접 읽는다.

```bash
TAURI_SIGNING_PRIVATE_KEY=$(cat ~/.tauri/obsidian-nexus.key) \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
pnpm --dir apps/desktop tauri:build
```

> 멀티라인 base64 값은 파이프나 문자열 파싱을 거치면 줄바꿈이나 패딩이 손상될 수 있다. 항상 파일 직접 읽기를 사용할 것.

---

## 문제 3: pubkey 업데이트 누락

### 증상

자동 업데이트 서명 검증 실패.

### 원인

키를 재생성하면 공개키도 새로 발급된다. `apps/desktop/src-tauri/tauri.conf.json`의 `plugins.updater.pubkey` 값이 구 키를 바라보고 있으면 업데이트 검증이 실패한다.

### 해결

키 재생성 직후 공개키를 확인하여 `tauri.conf.json`에 반영한다.

```bash
cat ~/.tauri/obsidian-nexus.key.pub
```

`tauri.conf.json`:
```json
{
  "plugins": {
    "updater": {
      "pubkey": "<새로 생성된 공개키 값>"
    }
  }
}
```

---

## 문제 4: MCP 바이너리 구버전으로 인한 도구 인식 실패

### 증상

데스크톱 앱 에이전트(및 MCP 클라이언트)에서 `nexus_get_toc` 도구를 인식하지 못함.

### 원인

릴리즈 타르볼에 포함된 `nexus-mcp-server` 바이너리가 해당 기능이 추가되기 전 구버전이었다. 코드 변경 후 타르볼 재패키징을 누락한 것이 원인.

### 해결

```bash
cargo build --release -p nexus-mcp-server
```

빌드 후 타르볼을 재패키징하여 GitHub Releases에 업로드한다.

빌드 검증:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | ./target/release/nexus-mcp-server
```

출력에서 `nexus_get_toc`가 포함되어 있는지 확인한다.

---

## 서명키 재생성 체크리스트

키를 새로 생성할 때마다 아래 순서를 따른다.

- [ ] `expect`로 키 생성 (절대경로 사용)
- [ ] 생성 위치 확인 — `ls ~/.tauri/obsidian-nexus.key`
- [ ] `tauri.conf.json`의 `pubkey` 업데이트
- [ ] 빌드 테스트 — `pnpm --dir apps/desktop tauri:build`
- [ ] 릴리즈 업로드 전 MCP 바이너리 재빌드 및 도구 목록 검증

---

## 교훈

- `.env`에서 멀티라인 base64 값은 파싱하지 말고 파일 직접 읽기 사용
- `cargo tauri signer generate`의 `-w` 옵션은 반드시 절대경로로 지정
- 키 재생성 시 `pubkey` 업데이트는 빌드 전 필수 단계
- 릴리즈 타르볼 패키징 전 `tools/list` 응답으로 MCP 도구 목록 검증 필수

## 관련 문서

- [[2026-03-25-mcp-update-asset-name-fix]]
- [[2026-03-25-onboarding-button]]
