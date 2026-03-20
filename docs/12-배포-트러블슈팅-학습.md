---
title: 배포 트러블슈팅 & 학습
tags: [guide, deployment, homebrew, ci-cd, troubleshooting]
aliases: [배포 학습, 트러블슈팅, homebrew 설정, brew tap, ci 최적화, 설치 스크립트]
created: 2026-03-20
updated: 2026-03-20
---

# 배포 트러블슈팅 & 학습

실제 배포 과정에서 발생한 문제와 해결 방법, 그리고 학습한 내용을 기록한다.

---

## Homebrew Tap 설정

### brew tap 이란

Homebrew의 서드파티 저장소 시스템. `homebrew-` 접두사가 붙은 GitHub 레포가 tap 저장소다.

```bash
brew tap gorillakim/nexus
# 내부적으로: git clone https://github.com/gorillakim/homebrew-nexus
```

한 번 등록하면 일반 패키지처럼 사용 가능:
```bash
brew install obsidian-nexus           # Formula (CLI)
brew install --cask obsidian-nexus   # Cask (Desktop 앱 .dmg)
```

### Formula vs Cask 차이

| | `brew install` (Formula) | `brew install --cask` (Cask) |
|--|--|--|
| 대상 | CLI 도구, 라이브러리 | macOS GUI 앱 (.app, .dmg) |
| 설치 위치 | `/opt/homebrew/bin/` | `/Applications/` |
| 예시 | `git`, `obsidian-nexus` | `slack`, `obsidian-nexus` |

---

## 트러블슈팅

### 1. formula 이름 충돌 — Sonatype Nexus Repository Manager

**증상:** `brew install nexus` 또는 `nexus --version` 실행 시 Java 로그가 폭발적으로 출력됨

**원인:** Homebrew 공식 저장소에 `nexus` (Sonatype Nexus Repository Manager v3.x)가 이미 존재. 우리 formula 이름이 동일해서 충돌.

**해결:** formula 이름을 `nexus` → `obsidian-nexus`로 변경
```ruby
# 변경 전
class Nexus < Formula

# 변경 후
class ObsidianNexus < Formula
```
파일명도 `Formula/nexus.rb` → `Formula/obsidian-nexus.rb`

**교훈:** Homebrew formula 이름은 공식 저장소(`homebrew-core`)와 충돌하지 않도록 프로젝트 고유 이름을 사용해야 한다.

---

### 2. "앱이 손상되었습니다" — macOS Gatekeeper

**증상:** DMG로 설치한 앱 실행 시 "Obsidian Nexus.app이 손상되어 열 수 없습니다" 오류

**원인:** macOS Gatekeeper가 Apple 미서명(notarization 없음) 앱에 quarantine 속성을 부여

**해결 방법 1 (수동):**
```bash
xattr -cr /Applications/Obsidian\ Nexus.app
```

**해결 방법 2 (Homebrew Cask에서 자동 처리):**
```ruby
cask "obsidian-nexus" do
  quarantine false   # 이 한 줄로 quarantine 속성 자동 제거
  ...
end
```

`brew install --cask obsidian-nexus` 로 설치하면 quarantine 문제가 자동 해결된다.

**교훈:** 공증(notarization) 없이 배포할 때는 Cask에 `quarantine false`를 반드시 추가해야 한다.

---

### 3. update-homebrew-tap.sh — `bad substitution` 오류

**증상:** CI homebrew job에서 `bad substitution` 에러 발생
```
./scripts/update-homebrew-tap.sh: line 47: s/(on_arm do.*?sha256 ")...
```

**원인:** `perl` one-liner 내부의 bash 문자열 보간(`${"$1"}` 문법)이 macOS `bash`에서 지원되지 않음

**잘못된 방식:**
```bash
perl -i -0pe "s/(on_arm do.*?sha256 \")[^\"]+(\")/${\"\$1\"}${SHA_AARCH64}\$2/s" "$FORMULA"
```

**올바른 방식:** 파일 전체를 heredoc으로 재생성
```bash
cat > "${TAP_DIR}/Formula/obsidian-nexus.rb" <<FORMULA
class ObsidianNexus < Formula
  ...
  sha256 "${SHA_AARCH64}"
  ...
FORMULA
```

**교훈:** CI 스크립트에서 복잡한 정규식 치환보다 heredoc 재생성이 더 안정적이다.

---

## CI 최적화

### 병렬 실행

**변경 전:** `cli` job이 `desktop` job 완료를 기다림 (`needs: desktop`)
**변경 후:** 의존성이 없으므로 병렬 실행

```yaml
# 변경 전
cli:
  needs: desktop   # desktop 끝날 때까지 대기 (18분 낭비)

# 변경 후
cli:
  # needs 없음 — desktop과 동시 실행
```

### Rust 캐시 개선

**변경 전:** `actions/cache@v4`로 `target/` 전체 디렉토리 캐싱
- 캐시 크기 크고 업로드/다운로드 오래 걸림
- Cargo.lock 변경 시 캐시 전체 무효화

**변경 후:** `Swatinem/rust-cache@v2`
- crate별 증분 캐시
- 변경된 crate만 재빌드
- 첫 실행 후 캐시 히트 시 빌드 시간 대폭 단축

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    key: desktop  # job별 분리
```

### 단일 cargo build 호출

**변경 전:** aarch64, x86_64 순차 빌드 (공통 의존성 2회 컴파일)
```bash
cargo build --release --target aarch64-apple-darwin ...
cargo build --release --target x86_64-apple-darwin ...
```

**변경 후:** 한 번에 두 타겟 빌드 (공통 의존성 1회 컴파일)
```bash
cargo build --release \
  --target aarch64-apple-darwin \
  --target x86_64-apple-darwin \
  ...
```

---

## Homebrew Tap 자동 업데이트 구조

```
./scripts/bump-version.sh
  → 버전 bump + 태그 푸시
  → GitHub Actions 트리거
      ├── desktop job (DMG 빌드)
      ├── cli job     (CLI 바이너리 빌드) ← desktop과 병렬
      └── homebrew job (needs: desktop, cli)
            → scripts/update-homebrew-tap.sh
            → SHA256 계산
            → gorillakim/homebrew-nexus 레포 업데이트
            → 사용자: brew upgrade obsidian-nexus
```

**필요한 GitHub Secret:**
- `TAP_REPO_TOKEN`: Fine-grained PAT, `gorillakim/homebrew-nexus` `Contents: Read and write`

---

## 관련 파일

| 파일 | 역할 |
|------|------|
| `homebrew/Formula/obsidian-nexus.rb` | CLI formula 템플릿 |
| `homebrew/Casks/obsidian-nexus.rb` | Desktop 앱 cask 템플릿 |
| `scripts/update-homebrew-tap.sh` | tap 자동 업데이트 스크립트 |
| `.github/workflows/release.yml` | CI 릴리즈 워크플로우 |
| `install.sh` | curl 원클릭 설치 스크립트 |
