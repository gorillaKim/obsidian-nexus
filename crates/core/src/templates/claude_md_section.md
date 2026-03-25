
## Obsidian Nexus - 문서 탐색 도구 우선순위

문서를 검색하거나 지식베이스를 탐색할 때는 **Obsidian Nexus MCP 도구를 먼저 사용**하세요.

### 도구 선택 기준

| 작업 | 사용 도구 |
|------|-----------|
| 문서 검색 | `nexus_search` (hybrid mode 기본) |
| 전체 문서 읽기 | `nexus_get_document` |
| 특정 섹션만 | `nexus_get_section` (토큰 90% 절약) |
| 별칭으로 문서 찾기 | `nexus_resolve_alias` |
| 역방향 링크 탐색 | `nexus_get_backlinks` |
| 정방향 링크 탐색 | `nexus_get_links` |

### 검색 전략

1. `nexus_search(query, mode="hybrid")` → 자연어 검색
2. `nexus_get_section(path, heading)` → 필요한 섹션만 읽기
3. `nexus_get_backlinks` → 관련 문서 추가 탐색

### 폴백 규칙

`Read` / `Grep` 은 **Nexus로 해결 불가능한 경우에만** 사용 (예: 볼트 외부 코드/설정 파일).

### 검색 팁

- 검색 결과가 부족하면 한/영 키워드 변환 시도 (예: "데이터독" ↔ "datadog")
- 관련 키워드로 재시도 (예: "모니터링" → "observability")
