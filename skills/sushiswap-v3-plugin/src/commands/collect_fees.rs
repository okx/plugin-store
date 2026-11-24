use clap::Args;
use crate::config::{chain_config, pad_address, pad_u256, token_symbol};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_amount, get_decimals, nfpm_positions};

#[derive(Args)]
pub struct CollectFeesArgs {
    /// Token ID of the position NFT
    #[arg(long)]
    pub token_id: u128,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos (dry-run)
    #[arg(long)]
    pub dry_run: bool,
}

/// NFPM.collect(CollectParams) — selector 0xfc6f7865
/// CollectParams: (tokenId, recipient, amount0Max, amount1Max)
fn build_collect(token_id: u128, recipient: &str) -> String {
    format!(
        "0xfc6f7865{}{}{}{}",
        pad_u256(token_id),
        pad_address(recipient),
        pad_u256(u128::MAX), // amount0Max: collect all owed token0
        pad_u256(u128::MAX), // amount1Max: collect all owed token1
    )
}

pub async fn run(args: CollectFeesArgs, chain_id: u64) -> anyhow::Result<()> {
    let cfg = chain_config(chain_id)?;
    let rpc_owned = crate::config::rpc_url(chain_id)?;
    let rpc: &str = &rpc_owned;
    let nfpm = cfg.nfpm;

    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };

    // Fetch position to display pending fees and validate
    let pos = nfpm_positions(nfpm, args.token_id, rpc).await.map_err(|e| {
        anyhow::anyhow!(
            "Could not fetch position {}: {}. Verify the token ID is correct for chain {}.",
            args.token_id, e, cfg.name
        )
    })?;

    let sym0 = if token_symbol(&pos.token0, chain_id) != "UNKNOWN" {
        token_symbol(&pos.token0, chain_id).to_string()
    } else {
        pos.token0.clone()
    };
    let sym1 = if token_symbol(&pos.token1, chain_id) != "UNKNOWN" {
        token_symbol(&pos.token1, chain_id).to_string()
    } else {
        pos.token1.clone()
    };

    let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
    let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);

    let preview = serde_json::json!({
        "preview": true,
        "action": "collect-fees",
        "token_id":   args.token_id,
        "token0":     sym0,
        "token1":     sym1,
        "fee_bps":    pos.fee,
        "fee_pct":    format!("{:.4}%", pos.fee as f64 / 10_000.0),
        "uncollected_fees_token0": format_amount(pos.tokens_owed0, dec0),
        "uncollected_fees_token1": format_amount(pos.tokens_owed1, dec1),
        "wallet":     wallet,
        "chain":      cfg.name,
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to collect fees from this position.");
        return Ok(());
    }

    if pos.tokens_owed0 == 0 && pos.tokens_owed1 == 0 && !args.dry_run {
        eprintln!(
            "[sushiswap-v3] Warning: position shows 0 uncollected fees. \
             Proceeding anyway in case of rounding or indexer lag."
        );
    }

    let calldata = build_collect(args.token_id, &wallet);
    let result = wallet_contract_call(chain_id, nfpm, &calldata, false, args.dry_run, Some(&wallet)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "collect-fees",
        "token_id": args.token_id,
        "token0":   sym0,
        "token1":   sym1,
        "fee_bps":  pos.fee,
        "tx_hash":  tx_hash,
        "explorer": format!("{}/{}", cfg.explorer, tx_hash),
        "chain":    cfg.name,
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
