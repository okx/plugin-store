use clap::Args;
use crate::config::{factory, resolve_token_validated, rpc_url, token_symbol, CHAIN_ID};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{amm_get_pool, format_amount, get_decimals};

#[derive(Args)]
pub struct ClaimFeesArgs {
    /// First token of the pool (symbol or address)
    #[arg(long)]
    pub token_a: String,
    /// Second token of the pool (symbol or address)
    #[arg(long)]
    pub token_b: String,
    /// Claim from stable pool (default: volatile)
    #[arg(long)]
    pub stable: bool,
    /// Broadcast on-chain
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata only, do not call onchainos
    #[arg(long)]
    pub dry_run: bool,
}

/// Pool.claimFees() → (uint256 claimed0, uint256 claimed1)
/// Selector: 0xd294f093 (verified on-chain)
const CLAIM_FEES_SELECTOR: &str = "0xd294f093";

pub async fn run(args: ClaimFeesArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let token_a = resolve_token_validated(&args.token_a)?;
    let token_b = resolve_token_validated(&args.token_b)?;

    let sym_a = resolve_symbol(&token_a, &args.token_a);
    let sym_b = resolve_symbol(&token_b, &args.token_b);

    let zero = "0x0000000000000000000000000000000000000000";
    let pool_type = if args.stable { "stable" } else { "volatile" };
    let pool = amm_get_pool(fac, &token_a, &token_b, args.stable, rpc).await?;

    if pool == zero {
        anyhow::bail!(
            "No Aerodrome AMM {} pool found for {}/{}.",
            pool_type, sym_a, sym_b
        );
    }

    let dec_a = get_decimals(&token_a, rpc).await.unwrap_or(18);
    let dec_b = get_decimals(&token_b, rpc).await.unwrap_or(18);

    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let preview = serde_json::json!({
        "preview": true,
        "action": "claim_fees",
        "pool": pool,
        "pool_type": pool_type,
        "token_a": sym_a,
        "token_b": sym_b,
        "wallet": wallet,
        "chain": "Base (8453)",
        "note": "claimFees() sends accrued trading fees to your wallet. Amount is determined on-chain."
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast.");
        return Ok(());
    }

    let result  = wallet_contract_call(CHAIN_ID, &pool, CLAIM_FEES_SELECTOR, false, args.dry_run, Some(&wallet)).await?;
    let tx_hash = extract_tx_hash(&result);

    // Decode claimed amounts from return data if available
    let claimed_0 = result["data"]["returnData"]
        .as_str()
        .and_then(|s| {
            let clean = s.trim_start_matches("0x");
            if clean.len() >= 64 {
                u128::from_str_radix(&clean[0..64], 16).ok()
            } else { None }
        })
        .unwrap_or(0);

    let claimed_1 = result["data"]["returnData"]
        .as_str()
        .and_then(|s| {
            let clean = s.trim_start_matches("0x");
            if clean.len() >= 128 {
                u128::from_str_radix(&clean[64..128], 16).ok()
            } else { None }
        })
        .unwrap_or(0);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "claim_fees",
        "pool": pool,
        "pool_type": pool_type,
        "claimed_token_a": format_amount(claimed_0, dec_a),
        "claimed_token_b": format_amount(claimed_1, dec_b),
        "token_a": sym_a,
        "token_b": sym_b,
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
    });
    if claimed_0 == 0 && claimed_1 == 0 {
        out["note"] = serde_json::json!(
            "No fees accrued yet — fees accumulate as trading volume passes through your LP position."
        );
    }
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn resolve_symbol(addr: &str, fallback: &str) -> String {
    let s = token_symbol(addr);
    if s != "UNKNOWN" { s.to_string() } else { fallback.to_string() }
}
