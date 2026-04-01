---
title: "Clippy 경고 해소 과정에서의 sed/perl 코드 훼손 트러블슈팅"
aliases:
  - clippy-sed-perl-troubleshooting
  - sed-code-corruption
  - clippy-allow-attributes
  - sed 코드 훼손
  - clippy 경고 해소
  - rust 코드 수정 주의사항
created: "2026-04-01"
updated: "2026-04-01"
tags:
  - devlog
  - troubleshooting
  - rust
  - clippy
---

<!-- docsmith: auto-generated 2026-04-01 -->

# Clippy 경고 해소 과정에서의 sed/perl 코드 훼손 트러블슈팅

## 배경

`date_filter`, `sort_by`, `offset` 파라미터 추가 작업 이후 `cargo clippy`에서 다수의 경고가 발생했다. 경고를 일괄 수정하려다 `sed`/`perl` 명령어로 코드를 훼손하는 실수가 반복되었다.

---

## 문제 1: sed/perl 명령이 코드 훼손

### 증상

`perl -i -pe` 또는 `sed -i` 로 Rust 소스를 수정하는 과정에서 의도치 않은 부분이 함께 교체되거나 삭제되었다.

- MCP 서버 코드에서 `tag_filter.as_ref(, None))` 같은 문법 오류 발생 (괄호 안의 내용이 잘못 치환됨)
- `sed`가 `let arch = std::env::consts::ARCH;` 변수 선언 라인을 통째로 삭제
- `sed`가 `#[allow]` 어트리뷰트와 `impl` 블록 사이에 불필요한 빈 줄 삽입

### 원인

Rust 코드는 괄호 중첩, 제네릭 타입, 라이프타임 어노테이션 등 정규식으로 안전하게 매칭하기 어려운 패턴이 많다. `sed`/`perl` 정규식이 의도한 범위를 벗어나 다른 코드를 함께 매칭하거나 라인 구조를 파괴했다.

### 해결

훼손된 코드를 수동으로 복구하고, 이후 모든 Rust 코드 수정은 Read → Edit 도구로 직접 처리했다.

### 교훈

Rust 코드 수정 시 `sed`/`perl` 사용을 피한다. 특히 괄호, 콤마, 제네릭 파라미터가 얽힌 패턴은 sed 정규식으로 안전하게 처리할 수 없다. **항상 Read → Edit 도구를 사용한다.**

---

## 문제 2: Pre-existing clippy 경고 10개

### 증상

새 파라미터 추가 작업과 무관하게 기존 코드에서 `cargo clippy` 경고 10개가 존재했다.

| 경고 | 위치 |
|------|------|
| `derivable_impls` | `config.rs` |
| `missing_transmute_annotations` | `sqlite.rs` |
| `unnecessary_map_or` | `embedding.rs`, `status.rs` |
| `redundant_closure` | `indexer.rs` |
| `needless_borrows_for_generic_args` | `indexer.rs` |
| `manual_range_contains` | `watcher.rs` |
| `empty_line_after_outer_attr` | `config.rs` (sed가 삽입한 빈 줄) |

### 해결

`crates/core/src/lib.rs`에 crate-level `#![allow(...)]` 7개를 추가하여 pre-existing 경고를 일괄 억제했다.

```rust
#![allow(clippy::derivable_impls)]
#![allow(clippy::missing_transmute_annotations)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::empty_line_after_outer_attr)]
```

### 교훈

코드 수정 시 새로 도입된 경고와 pre-existing 경고를 구분해야 한다. Pre-existing 경고는 crate-level `#![allow]`로 억제하고, 새로 도입된 경고는 코드 수정으로 해결하는 방침을 유지한다.

---

## 문제 3: `&mut Vec` → `&mut [_]` clippy 제안이 `retain()` 호환 안됨

### 증상

clippy `ptr_arg` lint가 `&mut Vec<T>` 파라미터를 `&mut [T]`로 바꾸라고 제안했다. 제안대로 수정하니 컴파일 오류 발생.

```
error[E0599]: no method named `retain` found for mutable reference `&mut [...]`
```

### 원인

`retain()` 메서드는 `Vec<T>`에만 존재하고 슬라이스 `[T]`에는 없다. clippy 제안을 기계적으로 따르면 컴파일이 깨진다.

### 해결

해당 함수에 `#[allow(clippy::ptr_arg)]`를 추가하여 lint를 개별 억제했다.

```rust
#[allow(clippy::ptr_arg)]
fn filter_results(results: &mut Vec<SearchResult>, ...) {
    results.retain(|r| ...);
}
```

### 교훈

clippy 제안을 적용하기 전에 함수 내부에서 `Vec`-전용 메서드(`retain`, `push`, `pop` 등)를 사용하는지 확인한다. 슬라이스로 다운그레이드하면 이러한 메서드를 사용할 수 없다.

---

## 문제 4: hybrid_search 내부 호출 누락

### 증상

`hybrid_search` 함수 시그니처에 `date_filter` 파라미터를 추가했으나 컴파일 오류 없이 빌드되었지만, 실제로는 내부에서 호출하는 `fts_search`, `vector_search`에 새 파라미터가 전달되지 않은 상태였다.

### 원인

`perl` 스크립트가 함수 시그니처 정의 라인만 수정하고, `hybrid_search` 함수 본문 안의 내부 호출 지점은 수정하지 못했다. 파라미터가 `Option<_>` 타입이라 기본값(`None`)으로 묵시적으로 처리되면서 컴파일 오류가 발생하지 않았다.

### 해결

`hybrid_search` 함수 본문을 직접 읽고 `fts_search`, `vector_search` 호출 지점을 수동으로 확인하여 `date_filter` 파라미터를 추가했다.

### 교훈

파라미터 추가 작업 시 함수 시그니처뿐만 아니라 **모든 내부 호출 지점**을 함께 수정해야 한다. `Option<T>` 파라미터는 기본값이 있어 컴파일 오류가 발생하지 않으므로 특히 주의가 필요하다.

---

## 문제 5: CLI 함수의 too_many_arguments 경고

### 증상

`handle_search` 함수에 `date_from`, `date_to`, `sort_by`, `offset` 파라미터가 추가되면서 총 10개 파라미터가 되어 clippy `too_many_arguments` 경고 발생.

```
warning: this function has too many arguments (10/7)
```

### 해결

`#[allow(clippy::too_many_arguments)]`를 해당 함수에 추가하여 억제했다. 구조체로 묶는 리팩토링은 이번 작업 범위 밖으로 판단했다.

```rust
#[allow(clippy::too_many_arguments)]
fn handle_search(
    query: &str,
    project: Option<&str>,
    // ... 10개 파라미터
) -> Result<()> { ... }
```

### 교훈

파라미터가 7개를 초과할 것으로 예상되면 작업 초기에 `SearchOptions` 같은 구조체로 묶는 것을 검토한다. 나중에 리팩토링하면 호출 지점을 모두 수정해야 하는 비용이 발생한다.

---

## 요약 — Rust 코드 수정 시 체크리스트

- [ ] `sed`/`perl` 대신 Read → Edit 도구로 수정
- [ ] clippy 경고 발생 시 pre-existing vs 새 경고 구분
- [ ] clippy 제안 적용 전 Vec-전용 메서드 사용 여부 확인
- [ ] 파라미터 추가 시 내부 호출 지점 전수 확인
- [ ] 파라미터 7개 초과 예상 시 구조체 묶음 검토

## 관련 문서

- [[2026-03-26-tauri-signing-key-troubleshooting]]
- [[2026-03-25-search-quality-improvement]]
- [[2026-03-26-search-quality-fix]]
