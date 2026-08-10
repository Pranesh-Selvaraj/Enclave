//! Built-in offline embeddings (RAG without any external endpoint).
//!
//! Runs a small embedding model (sentence-transformers/all-MiniLM-L6-v2,
//! 384 dims) in-process via fastembed/ONNX Runtime. The model (~25 MB)
//! downloads from HuggingFace into the app data dir on first use and is then
//! cached there — after that, embeddings work fully offline.
//!
//! The model is process-global: one ONNX session, serialized behind a Mutex
//! (fastembed's embed() takes &mut self). Inference is CPU-bound and must be
//! called from spawn_blocking, never from the main thread.

use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use std::sync::{Mutex, OnceLock};

const MODEL: EmbeddingModel = EmbeddingModel::AllMiniLML6V2;

static ENGINE: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

fn engine(cache_dir: &std::path::Path) -> Result<&'static Mutex<TextEmbedding>, String> {
    if let Some(e) = ENGINE.get() {
        return Ok(e);
    }
    let model = TextEmbedding::try_new(
        TextInitOptions::new(MODEL)
            .with_show_download_progress(false)
            .with_cache_dir(cache_dir.to_path_buf()),
    )
    .map(Mutex::new)
    .map_err(|e| format!("Failed to load embedding model: {e}"))?;
    // ponytail: benign race on first use — a concurrent caller may download
    // the model twice; the loser is dropped. Failure retries on next call.
    Ok(ENGINE.get_or_init(move || model))
}

/// Embed one text. `cache_dir` is where the model lives once downloaded.
/// Blocking (ONNX inference) — call via spawn_blocking.
pub fn embed_text_blocking(cache_dir: &std::path::Path, text: &str) -> Result<Vec<f64>, String> {
    let guard = engine(cache_dir)?;
    let mut model = guard.lock().map_err(|e| e.to_string())?;
    let out = model
        .embed(vec![text.to_string()], None)
        .map_err(|e| format!("Embedding failed: {e}"))?;
    out.into_iter()
        .next()
        .map(|v| v.into_iter().map(|x| x as f64).collect())
        .ok_or_else(|| "Model returned no embedding".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "downloads the ~25 MB model on first run (needs internet once)"]
    fn offline_embedding_roundtrip() {
        let dir = std::env::temp_dir().join("enclave-embed-test");
        let v = embed_text_blocking(&dir, "hello world").unwrap();
        assert_eq!(v.len(), 384, "all-MiniLM-L6-v2 embeds to 384 dims");
        let w = embed_text_blocking(&dir, "hello world").unwrap();
        assert_eq!(v, w, "same input embeds deterministically");
        let z = embed_text_blocking(&dir, "completely different text").unwrap();
        assert_ne!(v, z);
    }
}
