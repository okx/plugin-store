/// Persistent storage for four.meme auth tokens (per wallet).
///
/// Tokens come from the `login` command (SIWE-style flow) and are reused by
/// `create-token` + the image-upload step. Stored at:
///   ~/.fourmeme-plugin/auth.json   (mode 0600)
///
/// Schema:
///   { "0xwalletaddr": "<meme-web-access token>", ... }
///
/// Tokens are bound to a specific wallet (the cookie's HMAC payload encodes
/// `{timestamp_ms}_{wallet_addr}_{nonce}`), so we key the store by lowercase
/// wallet address. ~30 day TTL on four.meme's side; we don't try to track
/// expiry locally — calls just return their auth-error and the user re-logs.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

fn auth_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".fourmeme-plugin"))
}

fn auth_path() -> Result<PathBuf> {
    Ok(auth_dir()?.join("auth.json"))
}

fn load_all() -> Result<Value> {
    let path = auth_path()?;
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&raw)
        .with_context(|| format!("parsing {}", path.display()))
}

fn save_all(v: &Value) -> Result<()> {
    let dir = auth_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("mkdir {}", dir.display()))?;
    let path = auth_path()?;
    let body = serde_json::to_string_pretty(v)?;
    std::fs::write(&path, body)
        .with_context(|| format!("writing {}", path.display()))?;
    // 0600 — keep the cookie out of other processes' reach
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

pub fn save_token(wallet: &str, token: &str) -> Result<()> {
    let mut v = load_all().unwrap_or_else(|_| json!({}));
    v[wallet.to_lowercase()] = Value::String(token.to_string());
    save_all(&v)
}

pub fn load_token(wallet: &str) -> Result<Option<String>> {
    let v = load_all()?;
    Ok(v.get(wallet.to_lowercase())
        .and_then(|x| x.as_str())
        .map(|s| s.to_string()))
}

/// Resolve auth: explicit `--auth-token` flag wins; otherwise look up the
/// stored token for `wallet` from `~/.fourmeme-plugin/auth.json`.
pub fn resolve_token(explicit: Option<&str>, wallet: &str) -> Result<String> {
    if let Some(t) = explicit {
        let t = t.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    match load_token(wallet)? {
        Some(t) => Ok(t),
        None => anyhow::bail!(
            "No four.meme auth token for wallet {}. Run `fourmeme-plugin login` first, \
             or pass --auth-token <value> to override.",
            wallet
        ),
    }
}
