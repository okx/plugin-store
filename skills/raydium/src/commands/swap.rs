/// swap: Execute a token swap on Raydium via the transaction API + onchainos broadcast.
///
/// Flow:
///   1. (dry_run guard) — return early before wallet resolution
///   2. Resolve Solana wallet address
///   3. GET /compute/swap-base-in → get quote
///   4. POST /transaction/swap-base-in → get base64 serialized tx
///   5. onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>
///
/// NOTE: Steps 4 and 5 must happen consecutively — Solana blockhash expires in ~60s.
use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{
    DEFAULT_COMPUTE_UNIT_PRICE, DEFAULT_SLIPPAGE_BPS, DEFAULT_TX_VERSION, PRICE_IMPACT_BLOCK_PCT,
    PRICE_IMPACT_WARN_PCT, RAYDIUM_AMM_PROGRAM, TX_API_BASE,
};
use crate::onchainos;

#[derive(Args, Debug)]
pub struct SwapArgs {
    /// Input token mint address
    #[arg(long)]
    pub input_mint: String,

    /// Output token mint address
    #[arg(long)]
    pub output_mint: String,

    /// Input amount in base units (with decimals, e.g. 1000000000 for 1 SOL)
    #[arg(long)]
    pub amount: u64,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u32,

    /// Transaction version: V0 or LEGACY (default: V0)
    #[arg(long, default_value = DEFAULT_TX_VERSION)]
    pub tx_version: String,

    /// Wrap native SOL to WSOL if input is SOL (default: true)
    #[arg(long, default_value_t = true)]
    pub wrap_sol: bool,

    /// Unwrap WSOL to native SOL if output is WSOL (default: true)
    #[arg(long, default_value_t = true)]
    pub unwrap_sol: bool,

    /// Priority fee in micro-lamports (default: 1000; "auto" is not supported by the API)
    #[arg(long, default_value = DEFAULT_COMPUTE_UNIT_PRICE)]
    pub compute_unit_price: String,

    /// Wallet public key (base58); if omitted, resolved from onchainos
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn execute(args: &SwapArgs, dry_run: bool) -> Result<()> {
    // Validate mint addresses before any API calls
    crate::config::validate_solana_address(&args.input_mint)?;
    crate::config::validate_solana_address(&args.output_mint)?;

    // dry_run guard — must come before resolve_wallet_solana()
    if dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "inputMint": args.input_mint,
                "outputMint": args.output_mint,
                "amount": args.amount,
                "slippageBps": args.slippage_bps,
                "note": "dry_run: tx not built or broadcast"
            }))?
        );
        return Ok(());
    }

    // Resolve wallet address
    let wallet = if let Some(ref w) = args.from {
        w.clone()
    } else {
        let w = onchainos::resolve_wallet_solana()?;
        if w.is_empty() {
            anyhow::bail!("Could not resolve wallet address. Pass --from or ensure onchainos is logged in.");
        }
        w
    };

    let client = reqwest::Client::new();

    // Step 1: Get swap quote
    let quote_url = format!("{}/compute/swap-base-in", TX_API_BASE);
    let quote_resp: Value = client
        .get(&quote_url)
        .query(&[
            ("inputMint", args.input_mint.as_str()),
            ("outputMint", args.output_mint.as_str()),
            ("amount", &args.amount.to_string()),
            ("slippageBps", &args.slippage_bps.to_string()),
            ("txVersion", args.tx_version.as_str()),
        ])
        .send()
        .await?
        .json()
        .await?;

    if !quote_resp["success"].as_bool().unwrap_or(false) {
        anyhow::bail!(
            "Failed to get swap quote: {}",
            serde_json::to_string(&quote_resp)?
        );
    }

    // Warn on high price impact
    let price_impact = quote_resp["data"]["priceImpactPct"]
        .as_f64()
        .unwrap_or(0.0);
    if price_impact >= PRICE_IMPACT_BLOCK_PCT {
        anyhow::bail!(
            "Price impact {:.2}% exceeds {:.1}% threshold. Swap aborted to protect funds.",
            price_impact,
            PRICE_IMPACT_BLOCK_PCT
        );
    }
    if price_impact >= PRICE_IMPACT_WARN_PCT {
        eprintln!(
            "WARNING: Price impact {:.2}% exceeds {:.1}% warning threshold. Proceeding.",
            price_impact, PRICE_IMPACT_WARN_PCT
        );
    }

    // Step 2: Build serialized transaction — must submit immediately after (blockhash ~60s)
    let tx_url = format!("{}/transaction/swap-base-in", TX_API_BASE);
    let tx_body = serde_json::json!({
        "swapResponse": quote_resp,
        "txVersion": args.tx_version,
        "wallet": wallet,
        "wrapSol": args.wrap_sol,
        "unwrapSol": args.unwrap_sol,
        "computeUnitPriceMicroLamports": args.compute_unit_price,
    });
    let tx_resp: Value = client
        .post(&tx_url)
        .json(&tx_body)
        .send()
        .await?
        .json()
        .await?;

    if !tx_resp["success"].as_bool().unwrap_or(false) {
        anyhow::bail!(
            "Failed to build swap transaction: {}",
            serde_json::to_string(&tx_resp)?
        );
    }

    let transactions = tx_resp["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No transactions in response"))?;

    if transactions.is_empty() {
        anyhow::bail!("No transactions returned from Raydium API");
    }

    // Step 3: Broadcast each transaction immediately (blockhash expires ~60s)
    let mut results: Vec<Value> = Vec::new();
    for tx_item in transactions {
        let serialized_tx = tx_item["transaction"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'transaction' field in tx item"))?;

        let broadcast_result =
            onchainos::wallet_contract_call_solana(RAYDIUM_AMM_PROGRAM, serialized_tx, false).await?;
        let tx_hash = onchainos::extract_tx_hash(&broadcast_result);
        results.push(serde_json::json!({
            "txHash": tx_hash,
            "broadcastResult": broadcast_result,
        }));
    }

    let output = serde_json::json!({
        "ok": true,
        "inputMint": args.input_mint,
        "outputMint": args.output_mint,
        "amount": args.amount,
        "outputAmount": quote_resp["data"]["outputAmount"],
        "priceImpactPct": price_impact,
        "transactions": results,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
