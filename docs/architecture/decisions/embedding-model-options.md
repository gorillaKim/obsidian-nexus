---
title: 임베딩 모델 선택 옵션
tags: [embedding, ollama, nomic, bge-m3, idea]
created: 2026-03-26
status: idea
---

# 임베딩 모델 선택 옵션

## 배경

현재 `nomic-embed-text` (768D) 고정 사용 중.
사용자 환경과 검색 패턴에 따라 모델을 선택할 수 있게 하는 것을 목표로 함.

## 모델 비교

| 항목 | nomic-embed-text | bge-m3 |
|------|-----------------|--------|
| 파라미터 | 137M | 567M |
| 모델 크기 | ~274MB | ~1.1GB |
| 차원 | 768D | 1024D |
| 다국어 (한↔영) | 준수 | 우수 |
| RAM 사용 | ~500MB | ~1.5~2GB |
| 문서 1개 인덱싱 (CPU) | ~50ms | ~150~200ms |
| 권장 환경 | RAM 8GB+, CPU only 포함 | RAM 16GB+, GPU 권장 |

## 사용자 선택 시나리오

- **한국어 문서만 사용** → `nomic-embed-text` 충분
- **한↔영 혼용 문서, 크로스랭귀지 검색 필요** → `bge-m3` 권장

## 구현 시 고려사항

현재 코드는 이미 추상화 준비됨:
- `config.embedding.model` — 모델명 변경 가능
- `config.embedding.dimensions` — 차원 변경 가능 (`db/sqlite.rs:91`에서 `vec0` 생성 시 사용)

### 모델 변경 시 필수 작업

1. `config.toml`에서 `model`과 `dimensions` 변경
2. `vec_chunks` 테이블 DROP & CREATE (차원이 달라지면 기존 벡터 무효)
3. 전체 재인덱싱 실행 (`nexus_index_project`)

### 향후 구현 아이디어

- 설치 시 또는 첫 실행 시 모델 선택 wizard
- `nexus_index_project` 실행 전 모델 변경 감지 → 자동 재인덱싱 유도
- `nexus_status`에 현재 모델 및 차원 표시 (이미 일부 구현됨)

## 결정 유보

이번 개발 사이클에서는 구현하지 않음.
Task 1(임베딩 텍스트 강화), Task 2(FTS5 aliases 통합) 완료 후 재검토.
