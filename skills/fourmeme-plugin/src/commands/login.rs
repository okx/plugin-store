/// `fourmeme-plugin login` — SIWE-style login to four.meme via onchainos wallet.
///
/// Flow:
///   1. resolve wallet address from `onchainos wallet addresses --chain 56`
///   2. POST /meme-api/v1/private/user/nonce/generate { address, LOGIN, BSC }
///      → response.data = 6-digit nonce string
///   3. `onchainos wallet sign-message --type personal --message
///      "You are sign in Meme {nonce}" --from {wallet} --chain 56`
///      → 65-byte ECDSA signature
///   4. POST /meme-api/v1/private/user/login/dex with verifyInfo+signature
///      → response.data = `meme-web-access` token (cookie value)
///   5. save token to `~/.fourmeme-plugin/auth.json` keyed by wallet
///
/// Subsequent `create-token` and image-upload calls auto-load the token from
/// disk; no need to pass `--auth-token` once logged in.

use anyhow::{Context, Result};
use clap::Args;
use serde_json::{json, Value};

use crate::config::is_supported_chain;

const FOURMEME_API: &str = "https://four.meme";

#[derive(Args)]
pub struct LoginArgs {
    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Override the wallet address (default: query onchainos)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: LoginArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("login"), None));
            Ok(())
        }
    }
}

/// Run the SIWE flow for `wallet` and persist the token. Reusable by other
/// commands (e.g. quickstart auto-login).
pub async fn do_login(chain_id: u64, wallet: &str) -> Result<String> {
    eprintln!("[fourmeme] login as wallet {}...", wallet);
    let nonce = fetch_nonce(wallet).await
        .context("nonce/generate step failed")?;
    eprintln!("[fourmeme] nonce: {}", nonce);
    let message = format!("You are sign in Meme {}", nonce);
    eprintln!("[fourmeme] requesting wallet signature for: {:?}", message);
    let signature = personal_sign(wallet, &message, chain_id).await
        .context("personal_sign via onchainos failed")?;
    let token = submit_login(wallet, &signature).await
        .context("login/dex step failed")?;
    crate::auth::save_token(wallet, &token)
        .context("saving token to ~/.fourmeme-plugin/auth.json")?;
    Ok(token)
}

async fn run_inner(args: LoginArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }

    let wallet = match args.wallet {
        Some(w) => w,
        None => crate::onchainos::get_wallet_address(args.chain).await?,
    };
    let token = do_login(args.chain, &wallet).await?;

    let preview: String = token.chars().take(8).collect::<String>() + "..."
        + &token.chars().rev().take(6).collect::<String>().chars().rev().collect::<String>();
    let resp = json!({
        "ok": true,
        "data": {
            "action": "login",
            "wallet": wallet,
            "chain": "bsc",
            "chain_id": args.chain,
            "auth_token_preview": preview,
            "stored_at": "~/.fourmeme-plugin/auth.json",
            "tip": "Token saved (mode 0600). create-token will auto-use it. \
                   Re-run `login` if you see a FOURMEME_AUTH_REQUIRED error (~30-day TTL).",
        }
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}

async fn fetch_nonce(wallet: &str) -> Result<String> {
    let url = format!("{}/meme-api/v1/private/user/nonce/generate", FOURMEME_API);
    let body = json!({
        "accountAddress": wallet,
        "verifyType": "LOGIN",
        "networkCode": "BSC",
    });
    let resp = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/plain, */*")
        .header("origin", FOURMEME_API)
        .header("referer", format!("{}/en", FOURMEME_API))
        .json(&body)
        .send()
        .await
        .context("POST nonce/generate failed")?;
    let v: Value = resp.json().await.context("parsing nonce response")?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        anyhow::bail!("nonce/generate API error: {}", v);
    }
    v["data"].as_str().map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("nonce response missing data: {}", v))
}

async fn personal_sign(wallet: &str, message: &str, chain_id: u64) -> Result<String> {
    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet", "sign-message",
            "--type", "personal",
            "--message", message,
            "--chain", &chain_id.to_string(),
            "--from", wallet,
            "--force",
        ])
        .output()
        .await
        .context("spawning onchainos wallet sign-message")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos sign-message failed ({}): {}",
            output.status, stderr.trim()
        );
    }
    let v: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("parsing sign-message output: {}", stdout.trim()))?;
    v["data"]["signature"].as_str()
        .or_else(|| v["signature"].as_str())
        .or_else(|| v["data"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no signature in onchainos output: {}", stdout.trim()))
}

async fn submit_login(wallet: &str, signature: &str) -> Result<String> {
    let url = format!("{}/meme-api/v1/private/user/login/dex", FOURMEME_API);
    let body = json!({
        "region":     "WEB",
        "langType":   "EN",
        "loginIp":    "",
        "inviteCode": "",
        "verifyInfo": {
            "address":     wallet,
            "networkCode": "BSC",
            "signature":   signature,
            "verifyType":  "LOGIN",
        },
        "walletName": "WalletConnect",
    });
    let resp = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/plain, */*")
        .header("origin", FOURMEME_API)
        .header("referer", format!("{}/en", FOURMEME_API))
        .json(&body)
        .send()
        .await
        .context("POST login/dex failed")?;
    let v: Value = resp.json().await.context("parsing login/dex response")?;
    if v["code"].as_i64().unwrap_or(-1) != 0 {
        anyhow::bail!("login/dex API error: {}", v);
    }
    v["data"].as_str().map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("login/dex response missing data: {}", v))
}
