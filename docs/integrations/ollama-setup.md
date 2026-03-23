---
title: Ollama 설치 및 임베딩 설정
aliases:
  - ollama-setup
  - 올라마
  - ollama설정
  - 임베딩설정
tags:
  - integration
  - ollama
  - embedding
  - setup
  - vector-search
created: 2026-03-23
updated: 2026-03-23
---

<!-- docsmith: auto-generated 2026-03-23 -->

# Ollama 설치 및 임베딩 설정

## Ollama란?

[Ollama](https://ollama.com)는 로컬 머신에서 LLM(대형 언어 모델)과 임베딩 모델을 실행할 수 있는 오픈소스 도구입니다.

obsidian-nexus는 벡터 검색에 임베딩이 필요하며, 기본적으로 Ollama를 통해 `nomic-embed-text` 모델을 사용합니다. 인덱싱 시 각 문서 청크를 768차원 벡터로 변환하여 SQLite(sqlite-vec)에 저장하고, 검색 시 쿼리를 동일 모델로 임베딩하여 KNN 유사도 검색을 수행합니다.

Ollama가 없어도 키워드(FTS5) 검색은 정상 동작합니다. 벡터 검색과 하이브리드 검색만 비활성화됩니다.

---

## 설치

### macOS

```bash
brew install ollama
```

### Linux

```bash
curl -fsSL https://ollama.com/install.sh | sh
```

### Windows

[https://ollama.com/download](https://ollama.com/download)에서 설치 파일을 내려받아 실행합니다.

---

## 서비스 시작

```bash
ollama serve
```

기본적으로 `http://localhost:11434`에서 HTTP API가 활성화됩니다.

macOS에서 백그라운드 서비스로 등록하려면:

```bash
brew services start ollama
```

---

## nomic-embed-text 모델 다운로드

```bash
ollama pull nomic-embed-text
```

모델 크기는 약 274 MB입니다. 다운로드 후 로컬에 캐시되므로 이후에는 네트워크 없이 사용할 수 있습니다.

모델 목록 확인:

```bash
ollama list
```

출력 예시:

```
NAME                    ID              SIZE    MODIFIED
nomic-embed-text:latest 0a109f422b47    274 MB  2 minutes ago
```

---

## 연결 확인

Ollama가 정상 실행 중인지 확인합니다.

```bash
curl http://localhost:11434/api/tags
```

성공 응답 예시:

```json
{
  "models": [
    {
      "name": "nomic-embed-text:latest",
      "modified_at": "...",
      "size": 274302450
    }
  ]
}
```

임베딩 생성 테스트:

```bash
curl http://localhost:11434/api/embeddings \
  -d '{"model": "nomic-embed-text", "prompt": "hello world"}'
```

`embedding` 배열(768개 float)이 반환되면 정상입니다.

---

## nexus 설정 파일

설정 파일 위치: `~/.nexus/config.toml`

파일이 없으면 모든 항목이 기본값으로 동작합니다.

```toml
[embedding]
provider    = "ollama"                    # 임베딩 공급자 (기본값: "ollama")
model       = "nomic-embed-text"          # 임베딩 모델명 (기본값: "nomic-embed-text")
dimensions  = 768                         # 벡터 차원 수 (기본값: 768)
ollama_url  = "http://localhost:11434"    # Ollama API 주소 (기본값)
```

설정 변경 후에는 이미 인덱싱된 문서를 재인덱싱해야 새 임베딩이 적용됩니다.

### 포트 변경 예시

11434 포트를 다른 프로세스가 사용 중이라면:

```bash
# 다른 포트로 Ollama 실행
OLLAMA_HOST=0.0.0.0:11435 ollama serve
```

```toml
# config.toml에 반영
[embedding]
ollama_url = "http://localhost:11435"
```

---

## 트러블슈팅

### Ollama 서버가 실행되지 않은 경우

증상: 인덱싱 또는 벡터 검색 시 다음 오류 발생

```
Ollama is not running. Start with: ollama serve
```

해결:

```bash
ollama serve
# 또는 macOS 백그라운드 서비스
brew services start ollama
```

### 모델이 설치되지 않은 경우

증상:

```
Model 'nomic-embed-text' not found. Install with: ollama pull nomic-embed-text
```

해결:

```bash
ollama pull nomic-embed-text
```

### 포트 충돌

증상: `ollama serve` 실행 시 `address already in use` 오류

확인:

```bash
lsof -i :11434
```

해결 방법 1 — 기존 프로세스 종료:

```bash
kill $(lsof -t -i :11434)
ollama serve
```

해결 방법 2 — 다른 포트 사용 (위 포트 변경 예시 참고)

### 임베딩 응답이 느린 경우

`nomic-embed-text`는 CPU만으로도 동작하지만 GPU가 있으면 훨씬 빠릅니다. 대용량 볼트 초기 인덱싱 시 시간이 걸릴 수 있습니다. 임베딩 요청 타임아웃은 30초로 설정되어 있습니다 (`embedding.rs`의 `http_client()`).

---

## Ollama 없이 사용하기

Ollama가 설치되지 않은 환경에서도 키워드 검색(FTS5)은 정상 동작합니다.

- 벡터 검색(`--mode vector`) — 비활성화
- 하이브리드 검색(`--mode hybrid`) — 키워드 검색으로 자동 폴백
- 키워드 검색(`--mode keyword`) — 정상 동작

Ollama 연결에 실패하면 nexus는 graceful fallback으로 키워드 검색 결과만 반환합니다. 벡터 검색 없이 운영하려면 별도 설정 변경이 필요하지 않습니다.

---

## 관련 문서

- [[05-설정-가이드]]
- [[02-검색-시스템]]
- [[00-프로젝트-개요]]
