//! Environment + config-directory handling.
//!
//! Honors the upstream `PROMPTFOO_*` environment variables so existing
//! workflows and `~/.promptfoo/` state keep working.

use directories::ProjectDirs;
use std::collections::HashMap;
use std::path::PathBuf;

/// Per-OS config directory. Honors `PROMPTFOO_CONFIG_DIR` override.
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PROMPTFOO_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    ProjectDirs::from("dev", "Lancer", "lancer-promptfoo")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".lancer-promptfoo"))
}

/// Cache file path. Honors `PROMPTFOO_CACHE_PATH`.
pub fn cache_path() -> PathBuf {
    if let Ok(p) = std::env::var("PROMPTFOO_CACHE_PATH") {
        return PathBuf::from(p);
    }
    config_dir().join("cache.json")
}

/// Log directory. Honors `PROMPTFOO_LOG_DIR`.
pub fn log_dir() -> PathBuf {
    if let Ok(p) = std::env::var("PROMPTFOO_LOG_DIR") {
        return PathBuf::from(p);
    }
    config_dir().join("logs")
}

/// The auth token file (mirrors `~/.promptfoo/auth.json`).
pub fn auth_path() -> PathBuf {
    config_dir().join("auth.json")
}

/// True when remote attack generation is disabled
/// (`PROMPTFOO_DISABLE_REDTEAM_REMOTE_GENERATION=1`).
pub fn remote_generation_disabled() -> bool {
    matches!(
        std::env::var("PROMPTFOO_DISABLE_REDTEAM_REMOTE_GENERATION").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Custom self-hosted share endpoint (`PROMPTFOO_SHARING_ENDPOINT`).
pub fn sharing_endpoint() -> String {
    std::env::var("PROMPTFOO_SHARING_ENDPOINT")
        .unwrap_or_else(|_| "https://api.promptfoo.app".to_string())
}

/// Parse a `.env` file and inject any KEY=VALUE pairs into the process env.
/// Existing process-env values win (so explicit env beats the file).
pub fn load_dotenv(path: &std::path::Path) -> std::io::Result<usize> {
    let text = std::fs::read_to_string(path)?;
    let mut loaded = 0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let mut v = v.trim();
            // strip surrounding quotes
            if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2)
                || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2)
            {
                v = &v[1..v.len() - 1];
            }
            if std::env::var(k).is_err() {
                std::env::set_var(k, v);
                loaded += 1;
            }
        }
    }
    Ok(loaded)
}

/// The full set of provider/credential env vars lancer-promptfoo recognizes.
/// Used by `validate` / `--self-test` to report which credentials are present.
pub const RECOGNIZED_ENV_KEYS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GOOGLE_API_KEY",
    "AZURE_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "GROQ_API_KEY",
    "MISTRAL_API_KEY",
    "COHERE_API_KEY",
    "TOGETHER_API_KEY",
    "OPENROUTER_API_KEY",
    "XAI_API_KEY",
    "DEEPSEEK_API_KEY",
    "PERPLEXITY_API_KEY",
    "FIREWORKS_API_KEY",
    "CEREBRAS_API_KEY",
    "REPLICATE_API_TOKEN",
    "HF_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "PROMPTFOO_API_KEY",
];

/// Snapshot which recognized credentials are set (value redacted).
pub fn present_credentials() -> HashMap<&'static str, bool> {
    RECOGNIZED_ENV_KEYS
        .iter()
        .map(|k| (*k, std::env::var(k).is_ok()))
        .collect()
}
