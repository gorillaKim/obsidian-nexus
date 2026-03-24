---
title: "문서 아카이빙 전략"
aliases:
  - archiving-strategy
  - 아카이빙전략
  - 문서아카이빙
  - document-archiving
tags:
  - guide
  - archiving
  - document-management
  - best-practice
created: "2026-03-24"
updated: "2026-03-24"
---

# 문서 아카이빙 전략

> 오래되거나 더 이상 유효하지 않지만 히스토리 보존이 필요한 문서를 관리하는 전략.
> **인플레이스(in-place) 방식**을 기본으로 채택하여 위키링크를 유지하면서 문서의 신선도를 명시한다.

---

## 핵심 원칙

Nexus의 위키링크(`[[파일명]]`)는 경로가 아닌 **파일명 기반**으로 resolve된다.
파일을 다른 폴더로 이동하면 기존 링크가 모두 깨지므로, 아카이빙은 파일을 이동하지 않고 **frontmatter 상태 플래그**로 처리한다.

```
파일 이동 ❌  →  위키링크 전부 깨짐
frontmatter 플래그 ✅  →  위키링크 유지 + 상태 명시
```

---

## Frontmatter 스펙

아카이빙할 문서에 아래 필드를 추가한다.

```yaml
---
status: archived
archived_at: "2026-03-24"
archived_reason: "v2 설계로 전면 대체됨"
superseded_by: "[[guides/new-document]]"
tags:
  - archived
  - (기존 태그 유지)
---
```

### 필드 설명

| 필드 | 필수 | 설명 |
|------|------|------|
| `status: archived` | ✅ | 아카이브 상태 표시. 검색/대시보드 필터링 기준 |
| `archived_at` | ✅ | 아카이빙 날짜 (`YYYY-MM-DD`) |
| `archived_reason` | ✅ | 아카이빙 사유. 미래의 독자를 위한 컨텍스트 |
| `superseded_by` | 선택 | 이 문서를 대체하는 후계 문서 위키링크 |
| `tags: [archived]` | ✅ | 검색 태그 필터링 및 대시보드 감지용 |

---

## 아카이빙 워크플로우

### Step 1. 백링크 확인

아카이빙 전에 반드시 백링크를 확인한다. 백링크가 있으면 참조 문서에도 안내가 필요하다.

```
nexus_get_backlinks(project, path) 호출
→ 백링크 있음: Step 2 진행 + 참조 문서에 안내 배너 추가
→ 백링크 없음: Step 2 진행 (참조 문서 수정 불필요)
```

### Step 2. 아카이빙 대상 문서에 frontmatter 추가

```yaml
status: archived
archived_at: "YYYY-MM-DD"
archived_reason: "사유 기술"
superseded_by: "[[후계문서]]"  # 있을 경우
```

기존 태그 목록에 `archived`를 추가한다.

### Step 3. 문서 상단에 안내 배너 추가

```markdown
> [!warning] 아카이브된 문서
> 이 문서는 **YYYY-MM-DD** 기준으로 아카이브되었습니다.
> 사유: {archived_reason}
> 최신 내용은 [[후계문서]]를 참조하세요.
```

### Step 4. 후계 문서에 역참조 추가 (해당 시)

후계 문서 상단 또는 관련 문서 섹션에 아래를 추가한다.

```markdown
> [!info] 이전 버전
> 이 문서는 [[구문서|구 버전]]을 대체합니다.
```

---

## 아카이빙 후보 판별 기준

다음 조건 중 하나 이상 해당하면 아카이빙을 검토한다.

| 기준 | 판별 방법 |
|------|----------|
| **장기 미수정** | `last_modified` 기준 6개월 이상 갱신 없음 |
| **조회수 0** | Dashboard "관심 필요 문서" 섹션에서 확인 |
| **백링크 0** | `nexus_get_backlinks` 결과 없음 |
| **후계 문서 존재** | `superseded_by` 대상 문서가 이미 작성됨 |
| **설계 변경으로 무효화** | 구현이 변경되어 내용이 더 이상 사실이 아님 |

> [!tip] Dashboard 활용
> **관심 필요 문서** 섹션(v0.4.0+)에서 열람 없음 + 백링크 없음 + 장기 미수정 문서를 자동으로 감지한다. 이 목록이 아카이빙 후보의 1차 소스가 된다.

---

## 검색 시스템 통합

### 아카이브 문서만 검색

```
nexus_search(query: "...", tags: ["archived"])
```

### 아카이브 문서 제외하고 검색

현재 `nexus_search`는 태그 제외 필터를 직접 지원하지 않는다.
운영 팁: 검색 결과에 `status: archived` 문서가 보이면 `tags: [archived]` 필터로 확인 후 무시한다.

### 대시보드 감지

`status: archived` 문서에 다른 문서가 백링크를 걸고 있으면 → **깨진 지식 그래프 신호**.
Dashboard "관심 필요 문서" 섹션에서 이런 문서를 우선 확인하고 참조 문서를 업데이트한다.

---

## 예시

### Before (아카이빙 전)

```markdown
---
title: "구 인증 설계"
tags:
  - architecture
  - auth
---

# 구 인증 설계
...
```

### After (아카이빙 후)

```markdown
---
title: "구 인증 설계"
status: archived
archived_at: "2026-03-24"
archived_reason: "JWT → Session 방식으로 전환, 설계 전면 변경"
superseded_by: "[[architecture/auth-v2]]"
tags:
  - archived
  - architecture
  - auth
---

> [!warning] 아카이브된 문서
> 이 문서는 **2026-03-24** 기준으로 아카이브되었습니다.
> 사유: JWT → Session 방식으로 전환, 설계 전면 변경
> 최신 내용은 [[architecture/auth-v2]]를 참조하세요.

# 구 인증 설계
...
```

---

## 관련 문서

- [[architecture/chunk-scalability|청크 규모 확장과 검색 품질 분석]]
- [[guides/configuration|설정 가이드]] — `exclude_patterns`로 특정 폴더 인덱싱 제외 가능
- [[integrations/mcp-tools|MCP 도구 레퍼런스]] — `nexus_get_backlinks` 활용
