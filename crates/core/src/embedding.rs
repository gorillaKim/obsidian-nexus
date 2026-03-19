use serde::Deserialize;

use crate::config::Config;
use crate::error::{NexusError, Result};

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

/// Generate embedding vector for text using Ollama
pub fn embed_text(config: &Config, text: &str) -> Result<Vec<f32>> {
    let url = format!("{}/api/embeddings", config.embedding.ollama_url);

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "model": config.embedding.model,
            "prompt": text,
        }))
        .send()
        .map_err(|e| NexusError::Indexing(format!("Ollama request failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(NexusError::Indexing(format!(
            "Ollama returned {}: {}",
            status, body
        )));
    }

    let data: OllamaEmbeddingResponse = resp
        .json()
        .map_err(|e| NexusError::Indexing(format!("Ollama response parse error: {}", e)))?;

    Ok(data.embedding)
}

/// Generate embeddings for multiple texts (batched)
pub fn embed_batch(config: &Config, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    texts.iter().map(|t| embed_text(config, t)).collect()
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Normalize embedding vector to unit length (L2 norm = 1).
/// This makes L2 distance equivalent to cosine distance for sqlite-vec.
pub fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Serialize embedding to bytes (for SQLite BLOB storage)
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Deserialize embedding from bytes
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Check if Ollama is running and the model is available
pub fn check_ollama(config: &Config) -> Result<()> {
    let url = format!("{}/api/tags", config.embedding.ollama_url);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| NexusError::Config(format!("HTTP client error: {}", e)))?;

    let resp = client
        .get(&url)
        .send()
        .map_err(|_| NexusError::Config(
            "Ollama is not running. Start with: ollama serve".to_string()
        ))?;

    if !resp.status().is_success() {
        return Err(NexusError::Config("Ollama returned an error".to_string()));
    }

    // Check if model is available
    let data: serde_json::Value = resp.json().map_err(|e| NexusError::Config(e.to_string()))?;
    let models = data.get("models").and_then(|m| m.as_array());

    if let Some(models) = models {
        let model_name = &config.embedding.model;
        let found = models.iter().any(|m| {
            m.get("name")
                .and_then(|n| n.as_str())
                .map_or(false, |n| n.starts_with(model_name))
        });
        if !found {
            return Err(NexusError::Config(format!(
                "Model '{}' not found. Install with: ollama pull {}",
                model_name, model_name
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 1e-6);
    }

    #[test]
    fn test_embedding_serialization() {
        let original = vec![1.5, -2.3, 0.0, 42.0];
        let bytes = embedding_to_bytes(&original);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(original, restored);
    }
}
