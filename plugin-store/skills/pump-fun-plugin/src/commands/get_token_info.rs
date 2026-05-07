use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

/// Serialize an f64 as a decimal string (never scientific notation).
/// Formats to 9 decimal places and strips trailing zeros.
fn serialize_f64_decimal<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(
        &format!("{:.9}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string(),
    )
}

use pumpfun::{
    common::types::{Cluster, PriorityFee},
    PumpFun,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};

use crate::config::{DEFAULT_RPC_URL, FEE_BASIS_POINTS, GRADUATION_SOL_THRESHOLD};

#[derive(Args, Debug)]
pub struct GetTokenInfoArgs {
    /// Token mint address (base58)
    #[arg(long)]
    pub mint: String,

    /// Solana RPC URL (overrides HELIUS_RPC_URL env var and default)
    #[arg(long)]
    pub rpc_url: Option<String>,
}

#[derive(Serialize, Debug)]
struct TokenInfoOutput {
    ok: bool,
    mint: String,
    virtual_token_reserves: u64,
    virtual_sol_reserves: u64,
    real_token_reserves: u64,
    /// SOL (not lamports). Divide the raw on-chain lamport value by 1e9.
    #[serde(serialize_with = "serialize_f64_decimal")]
    real_sol_reserves: f64,
    token_total_supply: u64,
    complete: bool,
    creator: String,
    #[serde(serialize_with = "serialize_f64_decimal")]
    price_sol_per_token: f64,
    /// SOL (not lamports). Divide the raw on-chain lamport value by 1e9.
    #[serde(serialize_with = "serialize_f64_decimal")]
    market_cap_sol: f64,
    /// SOL (not lamports). Divide the raw on-chain lamport value by 1e9.
    #[serde(serialize_with = "serialize_f64_decimal")]
    final_market_cap_sol: f64,
    #[serde(serialize_with = "serialize_f64_decimal")]
    graduation_progress_pct: f64,
    status: String,
}

pub async fn execute(args: &GetTokenInfoArgs) -> Result<()> {
    let rpc_url = args
        .rpc_url
        .clone()
        .or_else(|| std::env::var("HELIUS_RPC_URL").ok())
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string());

    let mint: Pubkey = args
        .mint
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid mint address '{}': {}", args.mint, e))?;

    // Use a placeholder keypair — reads don't require signing
    let placeholder_keypair = Arc::new(Keypair::new());

    let commitment = CommitmentConfig::confirmed();
    let priority_fee = PriorityFee::default();

    // Build cluster with custom RPC URL
    let ws_url = derive_ws_url(&rpc_url);
    let cluster = Cluster::new(rpc_url, ws_url, commitment, priority_fee);

    let pumpfun = PumpFun::new(placeholder_keypair, cluster);

    let curve = pumpfun
        .get_bonding_curve_account(&mint)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Borsh") || msg.contains("serialization") || msg.contains("length") {
                anyhow::anyhow!(
                    "Failed to fetch bonding curve: {}. \
                     This token may use an updated pump.fun contract layout not yet supported by the SDK. \
                     Try `onchainos token search --mint {}` for token info instead.",
                    msg, mint
                )
            } else {
                anyhow::anyhow!("Failed to fetch bonding curve: {}", msg)
            }
        })?;

    let price_sol_per_token = if curve.virtual_token_reserves > 0 {
        curve.virtual_sol_reserves as f64 / curve.virtual_token_reserves as f64
    } else {
        0.0
    };

    let graduation_progress_pct = if GRADUATION_SOL_THRESHOLD > 0 {
        (curve.real_sol_reserves as f64 / GRADUATION_SOL_THRESHOLD as f64) * 100.0
    } else {
        0.0
    };

    let status = if curve.complete {
        "Graduated (trading on PumpSwap/Raydium)".to_string()
    } else {
        "Active (bonding curve)".to_string()
    };

    // get_final_market_cap_sol internally calls get_buy_out_price which can panic with
    // divide-by-zero when virtual_token_reserves <= real_token_reserves (nearly exhausted
    // bonding curve). Guard against this by catching the panic.
    let final_market_cap_sol = {
        let vtr = curve.virtual_token_reserves;
        let rtr = curve.real_token_reserves;
        // Replicate the sol_tokens selection from get_buy_out_price:
        // sol_tokens = max(amount, real_sol_reserves) where amount = real_token_reserves
        // Then denominator = virtual_token_reserves - sol_tokens, which panics when 0.
        // Use saturating arithmetic to return 0 in the degenerate case.
        let sol_tokens = if rtr < curve.real_sol_reserves {
            curve.real_sol_reserves as u128
        } else {
            rtr as u128
        };
        let vtr_u128 = vtr as u128;
        if vtr_u128 <= sol_tokens || vtr == 0 {
            // Degenerate case — bonding curve nearly or fully exhausted; return 0
            0u64
        } else {
            curve.get_final_market_cap_sol(FEE_BASIS_POINTS)
        }
    };

    let output = TokenInfoOutput {
        ok: true,
        mint: args.mint.clone(),
        virtual_token_reserves: curve.virtual_token_reserves,
        virtual_sol_reserves: curve.virtual_sol_reserves,
        real_token_reserves: curve.real_token_reserves,
        real_sol_reserves: curve.real_sol_reserves as f64 / 1_000_000_000.0,
        token_total_supply: curve.token_total_supply,
        complete: curve.complete,
        creator: curve.creator.to_string(),
        price_sol_per_token,
        market_cap_sol: curve.get_market_cap_sol() as f64 / 1_000_000_000.0,
        final_market_cap_sol: final_market_cap_sol as f64 / 1_000_000_000.0,
        graduation_progress_pct: graduation_progress_pct.min(100.0),
        status,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Derive a WebSocket URL from an HTTP RPC URL.
fn derive_ws_url(http_url: &str) -> String {
    http_url
        .replace("https://", "wss://")
        .replace("http://", "ws://")
}
