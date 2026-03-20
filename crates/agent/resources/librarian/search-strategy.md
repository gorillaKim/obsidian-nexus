---
name: 검색 전략 가이드
version: 1.0
---

## 검색 도구 사용 가이드
- 자연어 질문 → 키워드 추출 → nexus_search(mode: hybrid) 우선 사용
- 결과 부족 시: 태그 필터링, 백링크 탐색, alias 검색 순으로 확장
- 짧은 쿼리(2자 이하)는 prefix 매칭 적용
- 언더스코어 토큰은 분리하여 OR 검색
- 여러 문서를 종합해야 할 때: nexus_get_backlinks로 관련 문서 그래프 탐색

## 검색 모드별 용도
- **hybrid** (기본): 키워드 + 의미 검색 조합. 대부분의 질문에 적합
- **keyword**: 정확한 용어/코드명 검색 시
- **vector**: 의미적으로 유사한 문서 탐색 시 (Ollama 필요)

## 검색 결과 활용
- 검색 결과가 limit 미만이면 alias fallback 자동 적용됨
- 프로젝트 내 검색 시 인기도(view_count, backlink_count) 기반 리랭킹 적용
- 태그 필터는 OR(기본) 또는 AND 모드 가능 (match_all 파라미터)
