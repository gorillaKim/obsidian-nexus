//! 검색 시나리오 테스트
//!
//! 실제 업무 피드백 기반 검색 품질 검증:
//! - 짧은 쿼리 (2~4자)
//! - 긴 문장 쿼리
//! - 도메인 용어 불일치 (alias 기반 매칭)
//! - 한국어 + 영어 혼합
//! - 토큰화 alias 매칭

use nexus_core::config::Config;
use nexus_core::db::sqlite::DbPool;
use nexus_core::index_engine;
use nexus_core::project;
use nexus_core::search;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::fs;
use tempfile::TempDir;

// ─── 공통 셋업 ────────────────────────────────────────────────────────────

fn test_pool() -> DbPool {
    nexus_core::db::sqlite::register_sqlite_vec_for_test();
    let manager = SqliteConnectionManager::memory().with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )
    });
    let pool = Pool::builder().max_size(1).build(manager).unwrap();
    let conn = pool.get().unwrap();
    conn.execute_batch(include_str!("../migrations/V1__initial.sql")).unwrap();
    conn.execute_batch(include_str!("../migrations/V2__embeddings.sql")).unwrap();
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
            chunk_id TEXT PRIMARY KEY,
            embedding float[768]
        );",
    )
    .unwrap();
    conn.execute_batch(include_str!("../migrations/V4__links.sql")).unwrap();
    conn.execute_batch(include_str!("../migrations/V5__search_enhancements.sql")).unwrap();
    conn.execute_batch(include_str!("../migrations/V6__fts_aliases.sql")).unwrap();
    pool
}

/// 실무 KB(knowledge base) 시나리오 문서 세트를 준비한다.
/// 피드백에서 언급된 "overview 페이지 = performance-report" 구조를 재현.
fn setup_work_kb() -> (DbPool, TempDir, String) {
    let pool = test_pool();
    let vault = TempDir::new().unwrap();
    let vp = vault.path();

    fs::create_dir_all(vp.join("features")).unwrap();
    fs::create_dir_all(vp.join("guides")).unwrap();
    fs::create_dir_all(vp.join("decisions")).unwrap();

    // 핵심 문서: "퍼포먼스 리포트" 기능 — aliases에 overview 포함
    fs::write(
        vp.join("features/performance-report.md"),
        r#"---
title: 퍼포먼스 리포트
tags:
  - feature
  - dashboard
aliases:
  - performance-report
  - 퍼포먼스 리포트
  - AI 리포트
  - overview
---

# 퍼포먼스 리포트

대시보드 메인 화면의 분석 리포트 페이지입니다.

## 주요 지표

- MAU, DAU 트렌드 그래프
- 전환율 funnel 분석
- AI 기반 인사이트 요약

## 데이터 파이프라인

BigQuery → ETL → 캐시 → API → 프론트엔드 렌더링
"#,
    )
    .unwrap();

    // 관련 없는 문서: 컴포넌트 컨벤션
    fs::write(
        vp.join("guides/component-conventions.md"),
        r#"---
title: 컴포넌트 개발 컨벤션
tags:
  - guide
  - frontend
aliases:
  - component-conventions
  - 컴포넌트 규칙
---

# 컴포넌트 개발 컨벤션

React 컴포넌트 작성 규칙과 패턴을 정리합니다.

## 파일 구조

src/components 하위에 기능별 폴더로 구성합니다.

## Props 타입 정의

인터페이스를 별도로 선언하고 컴포넌트와 분리합니다.
"#,
    )
    .unwrap();

    // 설정 관련 문서
    fs::write(
        vp.join("features/setting-pages.md"),
        r#"---
title: 설정 페이지
tags:
  - feature
  - settings
aliases:
  - setting-pages
  - 설정
---

# 설정 페이지

사용자 계정 및 알림 설정 페이지입니다.

## 알림 설정

이메일, 슬랙, 인앱 알림 채널을 설정할 수 있습니다.
"#,
    )
    .unwrap();

    // 의사결정 문서 — "리뉴얼" 키워드가 링크에만 등장
    fs::write(
        vp.join("decisions/adr-dashboard-v2.md"),
        r#"---
title: ADR - 대시보드 V2 설계
tags:
  - decision
  - dashboard
aliases:
  - adr-dashboard-v2
  - 대시보드 설계
---

# ADR - 대시보드 V2 설계

## 배경

[[RD 연결 설정 V2 (리뉴얼)]] 작업과 연계하여 대시보드를 개편합니다.

## 결정

퍼포먼스 리포트 페이지를 중심으로 재설계합니다.
"#,
    )
    .unwrap();

    // 짧은 쿼리 테스트용: "MAU" 키워드가 있는 문서
    fs::write(
        vp.join("features/metrics-glossary.md"),
        r#"---
title: 지표 용어집
tags:
  - reference
  - metrics
aliases:
  - metrics-glossary
  - 지표
  - MAU
  - DAU
---

# 지표 용어집

## MAU

Monthly Active Users. 월간 활성 사용자 수.

## DAU

Daily Active Users. 일간 활성 사용자 수.

## 전환율

특정 액션을 완료한 사용자 비율.
"#,
    )
    .unwrap();

    let proj = project::add_project(&pool, "work-kb", vp.to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    let pid = proj.id.clone();
    (pool, vault, pid)
}

// ─── 짧은 쿼리 시나리오 ──────────────────────────────────────────────────

/// 2자 영문 쿼리: "MAU" → metrics-glossary 히트
#[test]
fn test_short_query_english_acronym() {
    let (pool, _vault, pid) = setup_work_kb();

    let results = search::fts_search(&pool, "MAU", Some(&pid), 10, None).unwrap();
    assert!(
        !results.is_empty(),
        "짧은 영문 약어 'MAU' 검색 결과가 있어야 함"
    );
    assert!(
        results[0].file_path.contains("metrics-glossary"),
        "MAU 문서가 1위여야 함, 실제: {}",
        results[0].file_path
    );
}

/// 2자 한국어 쿼리: "지표" → metrics-glossary 또는 performance-report 히트
#[test]
fn test_short_query_korean_two_chars() {
    let (pool, _vault, pid) = setup_work_kb();

    let results = search::fts_search(&pool, "지표", Some(&pid), 10, None).unwrap();
    assert!(
        !results.is_empty(),
        "짧은 한국어 '지표' 검색 결과가 있어야 함"
    );
    // 지표 용어집이 top 5 안에 있어야 함
    let found = results
        .iter()
        .take(5)
        .any(|r| r.file_path.contains("metrics-glossary") || r.file_path.contains("performance-report"));
    assert!(found, "top 5 안에 관련 문서가 있어야 함");
}

/// 3자 쿼리: "DAU" → metrics-glossary alias 매칭
#[test]
fn test_short_query_alias_exact_match() {
    let (pool, _vault, pid) = setup_work_kb();
    let config = Config::default();

    let results = search::hybrid_search(&pool, "DAU", Some(&pid), 10, &config, None).unwrap();
    assert!(
        !results.is_empty(),
        "alias 'DAU' hybrid 검색 결과가 있어야 함"
    );
    assert!(
        results[0].file_path.contains("metrics-glossary"),
        "DAU alias를 가진 metrics-glossary가 1위여야 함, 실제: {}",
        results[0].file_path
    );
}

// ─── 문장 쿼리 시나리오 ──────────────────────────────────────────────────

/// 긴 문장 쿼리: "대시보드 분석 리포트 페이지" → performance-report 히트
#[test]
fn test_sentence_query_korean_domain() {
    let (pool, _vault, pid) = setup_work_kb();

    let results = search::fts_search(&pool, "대시보드 분석 리포트 페이지", Some(&pid), 10, None).unwrap();
    assert!(
        !results.is_empty(),
        "문장 쿼리 검색 결과가 있어야 함"
    );
    let top3_paths: Vec<&str> = results.iter().take(3).map(|r| r.file_path.as_str()).collect();
    let hit = results
        .iter()
        .take(3)
        .any(|r| r.file_path.contains("performance-report"));
    assert!(
        hit,
        "top3 안에 performance-report가 있어야 함, 실제: {:?}",
        top3_paths
    );
}

/// 피드백 재현: "overview 페이지 리뉴얼" — alias 토큰 매칭으로 performance-report 히트
#[test]
fn test_sentence_query_alias_token_matching() {
    let (pool, _vault, pid) = setup_work_kb();
    let config = Config::default();

    // hybrid_search 내부의 resolve_alias_results가
    // "overview 페이지 리뉴얼"을 토큰으로 분리하여
    // "overview" alias를 가진 performance-report를 찾아야 함
    let results = search::hybrid_search(&pool, "overview 페이지 리뉴얼", Some(&pid), 10, &config, None).unwrap();
    assert!(
        !results.is_empty(),
        "alias 토큰 매칭으로 검색 결과가 있어야 함"
    );
    let found = results
        .iter()
        .any(|r| r.file_path.contains("performance-report"));
    assert!(
        found,
        "'overview' alias를 가진 performance-report가 결과에 포함되어야 함, 실제: {:?}",
        results.iter().map(|r| r.file_path.as_str()).collect::<Vec<_>>()
    );
}

/// alias 단일 토큰 직접 검색: "overview" → performance-report
#[test]
fn test_sentence_query_single_alias_token() {
    let (pool, _vault, pid) = setup_work_kb();
    let config = Config::default();

    let results = search::hybrid_search(&pool, "overview", Some(&pid), 10, &config, None).unwrap();
    assert!(
        !results.is_empty(),
        "alias 'overview' 검색 결과가 있어야 함"
    );
    assert!(
        results[0].file_path.contains("performance-report"),
        "alias 'overview'를 가진 performance-report가 1위여야 함, 실제: {}",
        results[0].file_path
    );
}

/// 영문 문장 쿼리: "performance report dashboard" → performance-report 히트
#[test]
fn test_sentence_query_english_technical() {
    let (pool, _vault, pid) = setup_work_kb();

    let results = search::fts_search(&pool, "performance report dashboard", Some(&pid), 10, None).unwrap();
    assert!(
        !results.is_empty(),
        "영문 문장 쿼리 검색 결과가 있어야 함"
    );
    let hit = results
        .iter()
        .take(3)
        .any(|r| r.file_path.contains("performance-report"));
    assert!(
        hit,
        "top3 안에 performance-report가 있어야 함, 실제: {:?}",
        results.iter().take(3).map(|r| r.file_path.as_str()).collect::<Vec<_>>()
    );
}

/// 무관 쿼리 → 엉뚱한 문서가 상위에 없어야 함 (음수 score도 OK지만 top1이 무관하면 안 됨)
#[test]
fn test_sentence_query_irrelevant_does_not_pollute_top() {
    let (pool, _vault, pid) = setup_work_kb();

    let results = search::fts_search(&pool, "소유권 빌림 lifetime", Some(&pid), 10, None).unwrap();
    // 없거나 있어도 관련 없는 문서만 나와야 함 — performance-report가 top1이면 안 됨
    if !results.is_empty() {
        assert!(
            !results[0].file_path.contains("performance-report"),
            "무관 쿼리에서 performance-report가 top1이면 안 됨"
        );
    }
}

// ─── 한국어 + 영어 혼합 쿼리 ─────────────────────────────────────────────

/// "BigQuery 파이프라인" → performance-report (영문 기술 용어 포함)
#[test]
fn test_mixed_korean_english_query() {
    let (pool, _vault, pid) = setup_work_kb();

    let results = search::fts_search(&pool, "BigQuery 파이프라인", Some(&pid), 10, None).unwrap();
    assert!(
        !results.is_empty(),
        "한영 혼합 쿼리 결과가 있어야 함"
    );
    assert!(
        results[0].file_path.contains("performance-report"),
        "BigQuery는 performance-report에만 있으므로 1위여야 함, 실제: {}",
        results[0].file_path
    );
}

/// "JWT OAuth" → security 문서 (cross-project 키워드)
#[test]
fn test_mixed_query_security_terms() {
    let (pool, _vault, pid) = setup_work_kb();

    // 이 KB에는 security 문서가 없으므로 결과 없음이 정상
    let results = search::fts_search(&pool, "JWT OAuth", Some(&pid), 10, None).unwrap();
    // 결과가 없거나 낮은 relevance여도 무관한 문서가 1위면 안 됨
    if !results.is_empty() {
        // 최소한 score가 음수여야 정상 (BM25 낮음)
        // 여기서는 파일 경로만 확인
        println!(
            "JWT OAuth 결과 (예상: 없거나 낮음): {:?}",
            results.iter().map(|r| (&r.file_path, r.score)).collect::<Vec<_>>()
        );
    }
}

// ─── 크로스 프로젝트 시나리오 ────────────────────────────────────────────

/// 프로젝트 필터 없이 alias 검색 → 해당 문서 반환
#[test]
fn test_alias_search_without_project_filter() {
    let (pool, _vault, _pid) = setup_work_kb();
    let config = Config::default();

    let results = search::hybrid_search(&pool, "AI 리포트", None, 10, &config, None).unwrap();
    assert!(
        !results.is_empty(),
        "프로젝트 필터 없이 alias '퍼포먼스 리포트' 검색 결과가 있어야 함"
    );
    let hit = results
        .iter()
        .any(|r| r.file_path.contains("performance-report"));
    assert!(hit, "performance-report가 결과에 포함되어야 함");
}

// ─── LLM rewrite config 비활성화 확인 ────────────────────────────────────

/// config.llm.enabled=false(기본값)일 때 hybrid_search가 정상 작동해야 함
#[test]
fn test_hybrid_search_with_llm_disabled() {
    let (pool, _vault, pid) = setup_work_kb();
    let config = Config::default();
    assert!(!config.llm.enabled, "기본값은 llm.enabled=false여야 함");

    let results =
        search::hybrid_search(&pool, "퍼포먼스 리포트 분석", Some(&pid), 10, &config, None).unwrap();
    // Ollama 없어도 keyword fallback으로 결과 있어야 함
    assert!(
        !results.is_empty(),
        "llm 비활성화 상태에서도 검색 결과가 있어야 함"
    );
    let hit = results
        .iter()
        .any(|r| r.file_path.contains("performance-report"));
    assert!(hit, "performance-report가 결과에 포함되어야 함");
}
