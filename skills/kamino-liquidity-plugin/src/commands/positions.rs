use clap::Args;
use serde_json::Value;

use crate::api;
use crate::onchainos;

#[derive(Args, Debug)]
pub struct PositionsArgs {
    /// Chain ID (must be 501 for Solana)
    #[arg(long, default_value = "501")]
    pub chain: u64,

    /// Wallet address (base58). If omitted, resolved from onchainos.
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    if args.chain != 501 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "kamino-liquidity only supports Solana (chain 501)",
                "error_code": "UNSUPPORTED_CHAIN",
                "suggestion": "Use --chain 501 or omit --chain (defaults to 501)."
            }))?
        );
        return Ok(());
    }

    let wallet = match args.wallet {
        Some(w) => w,
        None => match onchainos::resolve_wallet_solana() {
            Ok(w) => w,
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": false,
                        "error": format!("{:#}", e),
                        "error_code": "WALLET_NOT_FOUND",
                        "suggestion": "Run `onchainos wallet balance --chain 501` to verify login, or pass --wallet <address>."
                    }))?
                );
                return Ok(());
            }
        },
    };

    if wallet.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "Cannot resolve wallet address.",
                "error_code": "WALLET_NOT_FOUND",
                "suggestion": "Pass --wallet <address> or run `onchainos wallet balance --chain 501` to verify login."
            }))?
        );
        return Ok(());
    }

    let data = match api::get_user_positions(&wallet).await {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": format!("{:#}", e),
                    "error_code": "KAMINO_API_ERROR",
                    "suggestion": "Failed to fetch positions from Kamino API. Check your connection and retry."
                }))?
            );
            return Ok(());
        }
    };

    let positions = match data.as_array() {
        Some(arr) => arr,
        None => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": format!("Unexpected positions response: {}", data),
                    "error_code": "KAMINO_API_ERROR",
                    "suggestion": "Kamino API returned an unexpected format. Retry or check api.kamino.finance status."
                }))?
            );
            return Ok(());
        }
    };

    let mut results: Vec<serde_json::Map<String, Value>> = Vec::new();
    for pos in positions {
        let mut entry = serde_json::Map::new();
        // Actual API response fields (verified 2026-04-05):
        //   vaultAddress, stakedShares, unstakedShares, totalShares
        let vault = pos["vaultAddress"]
            .as_str()
            .unwrap_or(pos["vault"].as_str().unwrap_or(pos["kvault"].as_str().unwrap_or("")))
            .to_string();
        let staked_shares = pos["stakedShares"]
            .as_str()
            .unwrap_or("0")
            .to_string();
        let unstaked_shares = pos["unstakedShares"]
            .as_str()
            .unwrap_or("0")
            .to_string();
        let total_shares = pos["totalShares"]
            .as_str()
            .or_else(|| pos["sharesAmount"].as_str())
            .unwrap_or("0")
            .to_string();

        entry.insert("vault".into(), Value::String(vault));
        entry.insert("staked_shares".into(), Value::String(staked_shares));
        entry.insert("unstaked_shares".into(), Value::String(unstaked_shares));
        entry.insert("total_shares".into(), Value::String(total_shares));
        results.push(entry);
    }

    let output = serde_json::json!({
        "ok": true,
        "wallet": wallet,
        "chain": args.chain,
        "positions": results
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
