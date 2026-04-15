/// swap: Execute a token swap on Raydium via the transaction API + onchainos broadcast.
///
/// Flow:
///   1. (dry_run guard) - return early before wallet resolution
///   2. Resolve Solana wallet address
///   3. GET /compute/swap-base-in -> get quote
///   4. POST /transaction/swap-base-in -> get base64 serialized tx
///   5. onchainos wallet contract-call --chain 501 --unsigned-tx <base58_tx>
///
/// NOTE: Steps 4 and 5 must happen consecutively - Solana blockhash expires in ~60s.
use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{
    parse_human_amount, DEFAULT_COMPUTE_UNIT_PRICE, DEFAULT_SLIPPAGE_BPS, DEFAULT_TX_VERSION,
    PRICE_IMPACT_BLOCK_PCT, PRICE_IMPACT_WARN_PCT, RAYDIUM_AMM_PROGRAM, SOL_NATIVE_MINT,
    SOLANA_RPC_URL, USDC_SOLANA, TX_API_BASE,
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

    /// Input amount in human-readable units (e.g. "0.1" for 0.1 SOL, "1.5" for 1.5 USDC)
    #[arg(long)]
    pub amount: String,

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

/// Resolve token decimals for well-known mints; fall back to Raydium mint API for others.
/// SOL: 9 decimals, USDC on Solana: 6 decimals.
async fn resolve_decimals(mint: &str, client: &reqwest::Client) -> anyhow::Result<u8> {
    if mint == SOL_NATIVE_MINT {
        return Ok(9);
    }
    if mint == USDC_SOLANA {
        return Ok(6);
    }
    let url = format!("{}/mint/ids", crate::config::DATA_API_BASE);
    let resp: Value = client
        .get(&url)
        .query(&[("mints", mint)])
        .send()
        .await?
        .json()
        .await?;
    if let Some(decimals) = resp["data"][0]["decimals"].as_u64() {
        return Ok(decimals as u8);
    }
    anyhow::bail!(
        "Could not resolve decimals for mint '{}'. Pass amount in raw base units or use a known mint.",
        mint
    )
}

pub async fn execute(args: &SwapArgs, dry_run: bool) -> Result<()> {
    // Validate mint addresses before any API calls
    crate::config::validate_solana_address(&args.input_mint)?;
    crate::config::validate_solana_address(&args.output_mint)?;

    let client = reqwest::Client::new();

    // Resolve input token decimals and parse human-readable amount to raw u64
    let input_decimals = resolve_decimals(&args.input_mint, &client).await?;
    let raw_amount = parse_human_amount(&args.amount, input_decimals)?;

    // dry_run guard - must come before resolve_wallet_solana()
    if dry_run {
        let amount_display = args.amount.parse::<f64>()
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|_| args.amount.clone());
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "inputMint": args.input_mint,
                "outputMint": args.output_mint,
                "amount": args.amount,
                "amountDisplay": amount_display,
                "rawAmount": raw_amount,
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
            anyhow::bail!(
                "Could not resolve wallet address. Pass --from or ensure onchainos is logged in."
            );
        }
        w
    };

    // Pre-flight balance check
    if args.input_mint == SOL_NATIVE_MINT {
        let lamports = onchainos::get_sol_balance(&wallet, SOLANA_RPC_URL)
            .await
            .unwrap_or(0);
        if lamports < raw_amount {
            anyhow::bail!(
                "Insufficient SOL balance: need {:.9} SOL, have {:.9} SOL. \
                 Add funds to your wallet before swapping.",
                raw_amount as f64 / 1e9,
                lamports as f64 / 1e9,
            );
        }
    } else {
        let token_balance = onchainos::get_spl_token_balance(&wallet, &args.input_mint, SOLANA_RPC_URL)
            .await
            .unwrap_or(0);
        if token_balance < raw_amount {
            anyhow::bail!(
                "Insufficient token balance: need {} units, have {} units for mint {}. \
                 Add funds to your wallet before swapping.",
                raw_amount,
                token_balance,
                args.input_mint,
            );
        }
    }

    // Step 1: Get swap quote
    let quote_url = format!("{}/compute/swap-base-in", TX_API_BASE);
    let quote_resp: Value = client
        .get(&quote_url)
        .query(&[
            ("inputMint", args.input_mint.as_str()),
            ("outputMint", args.output_mint.as_str()),
            ("amount", &raw_amount.to_string()),
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

    // Step 2: Resolve input token account (required by Raydium API when input is SPL, not native SOL)
    let input_account: Option<String> = if args.input_mint != SOL_NATIVE_MINT {
        let acct = onchainos::get_token_account(&wallet, &args.input_mint, SOLANA_RPC_URL)
            .await
            .map_err(|e| anyhow::anyhow!(
                "Failed to resolve input token account for mint {}: {}. \
                 Ensure the wallet holds the input token before swapping.",
                args.input_mint, e
            ))?;
        Some(acct)
    } else {
        None
    };

    // Step 3: Build serialized transaction - must submit immediately after (blockhash ~60s)
    let tx_url = format!("{}/transaction/swap-base-in", TX_API_BASE);
    let mut tx_body = serde_json::json!({
        "swapResponse": quote_resp,
        "txVersion": args.tx_version,
        "wallet": wallet,
        "wrapSol": args.wrap_sol,
        "unwrapSol": args.unwrap_sol,
        "computeUnitPriceMicroLamports": args.compute_unit_price,
    });
    if let Some(ref acct) = input_account {
        tx_body["inputAccount"] = serde_json::Value::String(acct.clone());
    }
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

    // Step 4: Broadcast each transaction immediately (blockhash expires ~60s)
    let mut results: Vec<Value> = Vec::new();
    for tx_item in transactions {
        let serialized_tx = tx_item["transaction"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'transaction' field in tx item"))?;

        let broadcast_result =
            onchainos::wallet_contract_call_solana(RAYDIUM_AMM_PROGRAM, serialized_tx, false)
                .await?;
        let tx_hash = onchainos::extract_tx_hash(&broadcast_result)?;
        results.push(serde_json::json!({
            "txHash": tx_hash,
            "broadcastResult": broadcast_result,
        }));
    }

    let amount_display = args.amount.parse::<f64>()
        .map(|v| format!("{:.2}", v))
        .unwrap_or_else(|_| args.amount.clone());
    let output = serde_json::json!({
        "ok": true,
        "inputMint": args.input_mint,
        "outputMint": args.output_mint,
        "amount": args.amount,
        "amountDisplay": amount_display,
        "rawAmount": raw_amount,
        "outputAmount": quote_resp["data"]["outputAmount"],
        "priceImpactPct": price_impact,
        "transactions": results,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
