/// `fourmeme-plugin quickstart` — show wallet status and the next concrete step.
///
/// Status routing:
///   no_wallet      → run `onchainos wallet add`
///   chain_invalid  → use BSC (chain 56)
///   no_funds       → top up BNB on BSC (need ≥ 0.001 BNB for any meaningful trade)
///   ready_to_trade → call `quote-buy` to preview a trade
///   active         → user has BNB + already holds at least one Four.meme token

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{eth_get_balance_wei, wei_to_bnb};

const MIN_TRADE_BNB_WEI: u128 = 1_000_000_000_000_000; // 0.001 BNB

#[derive(Args)]
pub struct QuickstartArgs {
    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Optional list of Four.meme token addresses to check holdings for (comma-separated)
    #[arg(long)]
    pub tokens: Option<String>,

    /// Skip the auto-login step (just check status, don't sign anything).
    /// Useful when you only want to inspect wallet/balance without provisioning auth.
    #[arg(long, default_value_t = false)]
    pub no_login: bool,
}

pub async fn run(args: QuickstartArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("quickstart"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: QuickstartArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        let resp = serde_json::json!({
            "ok": true,
            "data": {
                "status": "chain_invalid",
                "chain_id": args.chain,
                "message": format!("Chain {} not supported. fourmeme-plugin v0.1 supports BNB Chain only (chain 56).", args.chain),
                "next_step": "fourmeme-plugin quickstart --chain 56",
            }
        });
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    let wallet = match crate::onchainos::get_wallet_address(args.chain).await {
        Ok(w) => w,
        Err(_) => {
            let resp = serde_json::json!({
                "ok": true,
                "data": {
                    "status": "no_wallet",
                    "chain": chain_name(args.chain),
                    "chain_id": args.chain,
                    "message": "No active onchainos wallet found.",
                    "next_step": "Run `onchainos wallet add` to create one, then re-run quickstart.",
                }
            });
            println!("{}", serde_json::to_string_pretty(&resp)?);
            return Ok(());
        }
    };

    // Auto-login: if no token saved for this wallet, do the SIWE flow now so
    // create-token / positions auto / etc. work in the same session. Free
    // (signature only, no on-chain tx). Skip with --no-login.
    let auth_status = match crate::auth::load_token(&wallet) {
        Ok(Some(_)) => "logged_in",
        _ => {
            if args.no_login {
                "not_logged_in"
            } else {
                match crate::commands::login::do_login(args.chain, &wallet).await {
                    Ok(_) => "logged_in_just_now",
                    Err(e) => {
                        eprintln!("[fourmeme] auto-login failed (non-fatal): {:#}", e);
                        "login_failed"
                    }
                }
            }
        }
    };

    let bnb_wei = eth_get_balance_wei(args.chain, &wallet).await?;
    let bnb = wei_to_bnb(bnb_wei);

    if bnb_wei < MIN_TRADE_BNB_WEI {
        let resp = serde_json::json!({
            "ok": true,
            "data": {
                "status": "no_funds",
                "chain": chain_name(args.chain),
                "chain_id": args.chain,
                "wallet": wallet,
                "auth_status": auth_status,
                "bnb_balance": format!("{:.6}", bnb),
                "bnb_balance_wei": bnb_wei.to_string(),
                "message": "Wallet BNB balance is below 0.001 BNB — need more for gas + a meaningful trade.",
                "next_step": "Top up BNB on BSC (bridge from another chain or buy on Binance), then re-run quickstart.",
            }
        });
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    // Check holdings if user passed --tokens. EVM-012: track per-token RPC
    // failures so a transient blip doesn't silently render as "0 holdings"
    // and route the user to `ready_to_trade` instead of `active`.
    let mut held: Vec<serde_json::Value> = Vec::new();
    let mut partial_tokens: Vec<serde_json::Value> = Vec::new();
    if let Some(toks) = args.tokens.as_ref() {
        for t in toks.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let bal = match super::erc20_balance(args.chain, t, &wallet).await {
                Ok(v) => v,
                Err(e) => {
                    partial_tokens.push(serde_json::json!({
                        "token": t,
                        "error": format!("{:#}", e),
                    }));
                    continue;
                }
            };
            if bal > 0 {
                let sym = super::erc20_symbol(args.chain, t).await;
                held.push(serde_json::json!({
                    "token": t,
                    "symbol": sym,
                    "balance": super::fmt_decimal(bal, crate::config::TOKEN_DECIMALS),
                    "balance_raw": bal.to_string(),
                }));
            }
        }
    }

    let status = if held.is_empty() { "ready_to_trade" } else { "active" };
    let next_step = if held.is_empty() {
        "Pick a token (paste the contract address from four.meme) and call \
         `fourmeme-plugin quote-buy --token <address> --funds 0.005`"
    } else {
        "Run `fourmeme-plugin positions --tokens <addresses>` to see your full holdings, \
         or call `fourmeme-plugin sell --token <address> --all` to exit."
    };

    let resp = serde_json::json!({
        "ok": true,
        "data": {
            "status": status,
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "wallet": wallet,
            "auth_status": auth_status,
            "bnb_balance": format!("{:.6}", bnb),
            "bnb_balance_wei": bnb_wei.to_string(),
            "held_tokens": held,
            "partial_tokens": partial_tokens,
            "next_step": next_step,
        }
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
