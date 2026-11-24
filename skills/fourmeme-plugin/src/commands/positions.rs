/// `fourmeme-plugin positions` — show wallet's Four.meme holdings.
///
/// Two modes:
///   - **auto** (default, requires `login`): hits `/private/user/token/owner/list`
///     for full holdings auto-discovery, enriched with on-chain `balanceOf` + sell
///     quote. Use this for Agent workflows.
///   - **explicit**: pass `--tokens 0x...,0x...` to query a fixed address list
///     (no login required, but no auto-discovery).

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain, TOKEN_DECIMALS};

#[derive(Args)]
pub struct PositionsArgs {
    /// Optional comma-separated list. Omit for auto mode (queries four.meme via login token).
    #[arg(long)]
    pub tokens: Option<String>,

    /// Hard cap on number of positions returned in auto mode (default 50, max 300).
    #[arg(long, default_value_t = 50)]
    pub limit: u32,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Override auth token (defaults to ~/.fourmeme-plugin/auth.json from `login`)
    #[arg(long)]
    pub auth_token: Option<String>,
}

pub async fn run(args: PositionsArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("positions"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: PositionsArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    // Mode selection
    let candidate_tokens: Vec<String> = match args.tokens.as_deref() {
        Some(csv) if !csv.trim().is_empty() => {
            csv.split(',').map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty()).collect()
        }
        _ => {
            // Auto mode — needs login
            let auth = crate::auth::resolve_token(args.auth_token.as_deref(), &wallet)?;
            let user_info = crate::api::fetch_user_info(&auth).await?;
            let user_id = user_info["userId"].as_i64()
                .ok_or_else(|| anyhow::anyhow!("user/info missing userId: {}", user_info))?;
            let cap = args.limit.clamp(1, 300);
            let holdings = crate::api::fetch_user_holdings(&auth, user_id, cap).await?;
            holdings.iter()
                .filter_map(|h| h["tokenAddress"].as_str().map(|s| s.to_lowercase()))
                .collect()
        }
    };

    let mut rows: Vec<serde_json::Value> = Vec::new();
    let mut total_bnb_value: f64 = 0.0;
    let mut partial_tokens: Vec<serde_json::Value> = Vec::new();

    for token in &candidate_tokens {
        // EVM-012: track per-token RPC failures separately from "user has 0
        // balance". Silent unwrap_or(0) used to hide tokens whose balance
        // read failed — looks identical to "no longer holding" but is a
        // transient RPC issue. Surface them in `partial_tokens` so callers
        // can retry.
        let bal = match super::erc20_balance(args.chain, token, &wallet).await {
            Ok(v) => v,
            Err(e) => {
                partial_tokens.push(serde_json::json!({
                    "token": token,
                    "error": format!("{:#}", e),
                }));
                continue;
            }
        };
        if bal == 0 {
            // In auto mode, four.meme may show "ever held" — skip empty positions
            // to keep the output clean.
            continue;
        }
        let sym = super::erc20_symbol(args.chain, token).await;
        let info = super::fetch_token_info(args.chain, token).await.ok();
        let mut row = serde_json::json!({
            "token": token,
            "symbol": sym,
            "balance":     super::fmt_decimal(bal, TOKEN_DECIMALS),
            "balance_raw": bal.to_string(),
        });
        if let Some(info) = info {
            row["graduated"] = serde_json::Value::Bool(info.liquidity_added);
            row["is_bnb_quoted"] = serde_json::Value::Bool(info.is_bnb_quoted());
            row["progress_pct"] = serde_json::Value::String(
                format!("{:.2}", info.progress_by_funds_pct())
            );
            if !info.liquidity_added && info.is_bnb_quoted() {
                if let Ok(q) = super::fetch_try_sell(args.chain, token, bal).await {
                    let bnb = q.funds as f64 / 1e18;
                    row["estimated_value_bnb"]     = serde_json::Value::String(format!("{:.8}", bnb));
                    row["estimated_value_bnb_wei"] = serde_json::Value::String(q.funds.to_string());
                    total_bnb_value += bnb;
                }
            }
        }
        rows.push(row);
    }

    let mode = if args.tokens.is_some() { "explicit" } else { "auto" };
    let resp = serde_json::json!({
        "ok": true,
        "data": {
            "mode": mode,
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "wallet": wallet,
            "scanned_tokens": candidate_tokens.len(),
            "active_positions": rows.len(),
            "positions": rows,
            "partial_tokens": partial_tokens,
            "total_estimated_value_bnb": format!("{:.8}", total_bnb_value),
            "tip": match mode {
                "auto" => "Auto mode used four.meme owner/list (login token). Empty positions filtered. \
                          Pass --tokens to query a fixed list without login.",
                _ => "Explicit mode. Pass no --tokens (and login first) to auto-discover all holdings.",
            },
        }
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
