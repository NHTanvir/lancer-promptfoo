//! Cloud auth — login / logout / whoami / teams. Tokens are stored in
//! `<config_dir>/auth.json`, mirroring the upstream `~/.promptfoo/auth.json`
//! token format so existing credentials interoperate.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_host: Option<String>,
}

pub fn load_token() -> Option<AuthToken> {
    let path = crate::core::env::auth_path();
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_token(token: &AuthToken) -> Result<()> {
    let path = crate::core::env::auth_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(token)?;
    std::fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))
}

/// Store an API key (non-interactive login). The interactive PKCE browser flow
/// is documented but a `--api-key` path is provided for CI.
pub fn login(token: String, email: Option<String>) -> Result<()> {
    save_token(&AuthToken {
        token,
        email,
        team: None,
        api_host: Some(crate::core::env::sharing_endpoint()),
    })
}

pub fn logout() -> Result<()> {
    let path = crate::core::env::auth_path();
    if path.exists() {
        std::fs::remove_file(&path).context("failed to remove auth token")?;
    }
    Ok(())
}

pub fn whoami() -> Option<AuthToken> {
    load_token()
}

/// Switch the active team (updates the stored token).
pub fn switch_team(team: &str) -> Result<()> {
    let mut tok = load_token().context("not logged in")?;
    tok.team = Some(team.to_string());
    save_token(&tok)
}
