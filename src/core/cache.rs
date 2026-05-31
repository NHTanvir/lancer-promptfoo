//! Response cache keyed by `(provider, prompt, options-hash)`.
//!
//! Upstream uses SQLite; we use a JSON-file map (no native dep, no build risk).
//! Dedup matters because the judge model runs on every response — a rerun
//! reuses cached completions and saves 60%+ of cost.

use crate::core::types::ProviderResponse;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Cache {
    path: PathBuf,
    entries: Mutex<HashMap<String, ProviderResponse>>,
    enabled: bool,
}

impl Cache {
    /// Load the cache from disk (or start empty).
    pub fn load(path: PathBuf, enabled: bool) -> Self {
        let entries = if enabled {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Cache {
            path,
            entries: Mutex::new(entries),
            enabled,
        }
    }

    pub fn disabled() -> Self {
        Cache {
            path: PathBuf::new(),
            entries: Mutex::new(HashMap::new()),
            enabled: false,
        }
    }

    /// FNV-1a 64-bit hash of the cache key components.
    pub fn key(provider: &str, prompt: &str, options: &str) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in provider.bytes().chain(b"\0".iter().copied())
            .chain(prompt.bytes()).chain(b"\0".iter().copied())
            .chain(options.bytes())
        {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }

    pub fn get(&self, key: &str) -> Option<ProviderResponse> {
        if !self.enabled {
            return None;
        }
        self.entries.lock().unwrap().get(key).cloned().map(|mut r| {
            r.cached = true;
            r
        })
    }

    pub fn put(&self, key: String, value: ProviderResponse) {
        if !self.enabled {
            return;
        }
        self.entries.lock().unwrap().insert(key, value);
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Persist the cache to disk.
    pub fn flush(&self) -> std::io::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let entries = self.entries.lock().unwrap();
        let json = serde_json::to_string(&*entries).unwrap_or_else(|_| "{}".to_string());
        std::fs::write(&self.path, json)
    }

    /// Clear all entries (and the on-disk file).
    pub fn clear(&self) -> std::io::Result<()> {
        self.entries.lock().unwrap().clear();
        if self.enabled && self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}
