use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

use pumpfun::{
    common::types::{Cluster, PriorityFee},
    PumpFun,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};

use crate::config::{DEFAULT_RPC_URL, FEE_BASIS_POINTS};

#[derive(Args, Debug)]
pub struct GetPriceArgs {
    /// Token mint address (base58)
    #[arg(long)]
    pub mint: String,

    /// Trade direction: "buy" or "sell"
    #[arg(long)]
    pub direction: String,

    /// Amount: SOL lamports for buy direction, token units for sell direction
    #[arg(long)]
    pub amount: u64,

    /// Fee in basis points for sell price calculation (default: 100 = 1%)
    #[arg(long, default_value_t = FEE_BASIS_POINTS)]
    pub fee_bps: u64,

    /// Solana RPC URL (overrides HELIUS_RPC_URL env var and default)
    #[arg(long)]
    pub rpc_url: Option<String>,
}

#[derive(Serialize, Debug)]
struct GetPriceOutput {
    ok: bool,
    mint: String,
    direction: String,
    amount_in: u64,
    amount_out: u64,
    amount_out_ui: f64,
    price_sol_per_token: f64,
    /// SOL (not lamports). Divide the raw on-chain lamport value by 1e9.
    market_cap_sol: f64,
    bonding_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    graduated_warning: Option<String>,
}

pub async fn execute(args: &GetPriceArgs) -> Result<()> {
    let direction = args.direction.to_lowercase();
    if direction != "buy" && direction != "sell" {
        anyhow::bail!("direction must be 'buy' or 'sell', got '{}'", args.direction);
    }

    let rpc_url = args
        .rpc_url
        .clone()
        .or_else(|| std::env::var("HELIUS_RPC_URL").ok())
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string());

    let mint: Pubkey = args
        .mint
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid mint address '{}': {}", args.mint, e))?;

    let placeholder_keypair = Arc::new(Keypair::new());
    let commitment = CommitmentConfig::confirmed();
    let priority_fee = PriorityFee::default();
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

    let (amount_out, amount_out_ui) = if direction == "buy" && curve.complete {
        // Graduated token — bonding curve is closed, get_buy_price would error.
        // Return ok:true with amount_out=0 and a graduated_warning instead.
        (0u64, 0.0f64)
    } else if direction == "buy" {
        let tokens = curve
            .get_buy_price(args.amount)
            .map_err(|e| anyhow::anyhow!("get_buy_price failed: {e}"))?;
        // pump.fun tokens have 6 decimals
        let ui = tokens as f64 / 1_000_000.0;
        (tokens, ui)
    } else if curve.complete {
        // Graduated token — bonding curve is closed, get_sell_price returns "Curve is complete".
        // Return ok:true with amount_out=0 and a graduated_warning instead of erroring out.
        (0u64, 0.0f64)
    } else {
        let lamports = curve
            .get_sell_price(args.amount, args.fee_bps)
            .map_err(|e| anyhow::anyhow!("get_sell_price failed: {e}"))?;
        // SOL lamports → UI SOL
        let ui = lamports as f64 / 1_000_000_000.0;
        (lamports, ui)
    };

    let graduated_warning = if curve.complete {
        Some("Token has graduated from bonding curve. Use onchainos dex swap execute --chain 501 to trade on PumpSwap/Raydium.".to_string())
    } else {
        None
    };

    let output = GetPriceOutput {
        ok: true,
        mint: args.mint.clone(),
        direction,
        amount_in: args.amount,
        amount_out,
        amount_out_ui,
        price_sol_per_token,
        market_cap_sol: curve.get_market_cap_sol() as f64 / 1_000_000_000.0,
        bonding_complete: curve.complete,
        graduated_warning,
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
