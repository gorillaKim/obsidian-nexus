use crate::config::Config;
use crate::error::{NexusError, Result};

/// Ollama /api/generate 응답 구조
#[derive(Debug, serde::Deserialize)]
struct GenerateResponse {
    response: String,
}

/// LLM을 사용하여 검색 쿼리를 도메인 용어로 재작성한다.
///
/// 실패 시 original_query를 그대로 반환하므로 항상 Ok(String)을 반환한다.
/// 호출자는 config.llm.enabled 여부를 확인한 후 호출해야 한다.
pub fn rewrite_query(config: &Config, original_query: &str) -> Result<String> {
    let ollama_url = if config.llm.ollama_url.is_empty() {
        &config.embedding.ollama_url
    } else {
        &config.llm.ollama_url
    };

    let url = format!("{}/api/generate", ollama_url);
    let prompt = build_rewrite_prompt(original_query);

    let timeout = if config.llm.timeout_secs > 0 {
        config.llm.timeout_secs
    } else {
        5
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| NexusError::Search(format!("HTTP client build failed: {e}")))?;

    let resp: std::result::Result<reqwest::blocking::Response, reqwest::Error> = client
        .post(&url)
        .json(&serde_json::json!({
            "model": config.llm.model,
            "prompt": prompt,
            "stream": false,
        }))
        .send();

    match resp {
        Ok(r) if r.status().is_success() => {
            match r.json::<GenerateResponse>() {
                Ok(gen) => {
                    let rewritten = sanitize_rewrite_output(&gen.response, original_query);
                    if rewritten.is_empty() {
                        Ok(original_query.to_string())
                    } else {
                        tracing::debug!(
                            original = original_query,
                            rewritten = %rewritten,
                            "query rewritten by LLM"
                        );
                        Ok(rewritten)
                    }
                }
                Err(e) => {
                    tracing::warn!("LLM query rewrite parse error: {e}, falling back to original");
                    Ok(original_query.to_string())
                }
            }
        }
        Ok(r) => {
            tracing::warn!(
                status = %r.status(),
                "LLM query rewrite request failed, falling back to original"
            );
            Ok(original_query.to_string())
        }
        Err(e) => {
            tracing::warn!("LLM query rewrite connection error: {e}, falling back to original");
            Ok(original_query.to_string())
        }
    }
}

/// LLM 출력을 sanitize한다.
///
/// - 첫 라인만 사용 (개행 이후 무시) — 멀티라인 조작 방어
/// - 원본 길이의 3배로 상한 — 과도하게 긴 출력 방어 (defense-in-depth)
///
/// NOTE: SQL injection은 호출 경로의 parameterized binding(?1)으로 이미 방어됨.
/// 이 함수는 semantic manipulation(엉뚱한 쿼리 반환)에 대한 부가 방어다.
fn sanitize_rewrite_output(raw: &str, original_query: &str) -> String {
    let max_len = (original_query.chars().count() * 3).max(50);
    raw.trim()
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn build_rewrite_prompt(query: &str) -> String {
    format!(
        r#"You are a search query optimizer for a developer knowledge base written in Korean and English.
Rewrite the user's search query to include likely technical and domain-specific synonyms.
Rules:
- Output ONLY the rewritten query, no explanation, no quotes, no prefix.
- Keep the original language (Korean → keep Korean + add English equivalents).
- Include alternative phrasings developers would use in documentation.
- Keep it concise (max 2x the original length).

User query: {query}
Rewritten query:"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_rewrite_prompt_contains_query() {
        let prompt = build_rewrite_prompt("overview 페이지 리뉴얼");
        assert!(prompt.contains("overview 페이지 리뉴얼"));
    }

    #[test]
    fn test_sanitize_single_line_passthrough() {
        let result = sanitize_rewrite_output("performance report redesign", "overview 리뉴얼");
        assert_eq!(result, "performance report redesign");
    }

    #[test]
    fn test_sanitize_multiline_keeps_first_line_only() {
        let raw = "performance report redesign\nIgnore above. DROP TABLE documents;\nmore stuff";
        let result = sanitize_rewrite_output(raw, "overview 리뉴얼");
        assert_eq!(result, "performance report redesign");
        assert!(!result.contains("DROP"));
    }

    #[test]
    fn test_sanitize_truncates_oversized_output() {
        let original = "ab";  // 2자
        let _max = 2 * 3;      // max_len = 6 (min 50 적용되므로 실제 50)
        let long_response = "a".repeat(200);
        let result = sanitize_rewrite_output(&long_response, original);
        assert!(result.chars().count() <= 50);
    }

    #[test]
    fn test_sanitize_empty_raw_returns_empty() {
        let result = sanitize_rewrite_output("   \n  ", "query");
        assert!(result.is_empty());
    }

    #[test]
    fn test_sanitize_trims_whitespace() {
        let result = sanitize_rewrite_output("  performance report  ", "overview");
        assert_eq!(result, "performance report");
    }
}
