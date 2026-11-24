use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{chain_config, pad_address, pad_u256, token_symbol, unix_now};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_amount, get_decimals, nfpm_positions};

#[derive(Args)]
pub struct RemoveLiquidityArgs {
    /// Token ID of the position NFT
    #[arg(long)]
    pub token_id: u128,
    /// Liquidity units to remove (use "max" to remove all)
    #[arg(long, default_value = "max")]
    pub liquidity: String,
    /// Deadline in minutes from now (default: 20)
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos (dry-run)
    #[arg(long)]
    pub dry_run: bool,
}

/// NFPM.decreaseLiquidity(DecreaseLiquidityParams) — selector 0x0c49ccbe
/// DecreaseLiquidityParams: (tokenId, liquidity, amount0Min, amount1Min, deadline)
fn build_decrease_liquidity(token_id: u128, liquidity: u128, deadline: u64) -> String {
    format!(
        "0x0c49ccbe{}{}{}{}{}",
        pad_u256(token_id),
        pad_u256(liquidity),
        pad_u256(0u128), // amount0Min = 0 (LP removal has limited sandwich risk)
        pad_u256(0u128), // amount1Min = 0
        format!("{:0>64x}", deadline),
    )
}

/// NFPM.collect(CollectParams) — selector 0xfc6f7865
/// CollectParams: (tokenId, recipient, amount0Max, amount1Max)
fn build_collect(token_id: u128, recipient: &str) -> String {
    format!(
        "0xfc6f7865{}{}{}{}",
        pad_u256(token_id),
        pad_address(recipient),
        pad_u256(u128::MAX), // amount0Max: collect all owed tokens
        pad_u256(u128::MAX), // amount1Max: collect all owed tokens
    )
}

pub async fn run(args: RemoveLiquidityArgs, chain_id: u64) -> anyhow::Result<()> {
    let cfg = chain_config(chain_id)?;
    let rpc_owned = crate::config::rpc_url(chain_id)?;
    let rpc: &str = &rpc_owned;
    let nfpm = cfg.nfpm;

    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };

    // Fetch current position info
    let pos = nfpm_positions(nfpm, args.token_id, rpc).await.map_err(|e| {
        anyhow::anyhow!(
            "Could not fetch position {}: {}. Verify the token ID is correct for chain {}.",
            args.token_id, e, cfg.name
        )
    })?;

    if pos.liquidity == 0 {
        anyhow::bail!(
            "Position {} has zero liquidity. It may already be fully removed. \
             Use `collect-fees --token-id {}` to claim remaining fees, \
             then `burn-position --token-id {}` to delete the NFT.",
            args.token_id, args.token_id, args.token_id
        );
    }

    let liquidity = if args.liquidity.eq_ignore_ascii_case("max") {
        pos.liquidity
    } else {
        args.liquidity.parse::<u128>().map_err(|_| {
            anyhow::anyhow!(
                "Invalid liquidity value '{}'. Use a positive integer or 'max'.",
                args.liquidity
            )
        })?
    };

    if liquidity == 0 {
        anyhow::bail!("Liquidity to remove must be greater than 0.");
    }
    if liquidity > pos.liquidity {
        anyhow::bail!(
            "Requested liquidity {} exceeds position liquidity {}.",
            liquidity, pos.liquidity
        );
    }

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

    let pct = liquidity * 100 / pos.liquidity;
    let deadline = unix_now() + args.deadline_minutes * 60;

    let preview = serde_json::json!({
        "preview": true,
        "action": "remove-liquidity",
        "token_id":              args.token_id,
        "token0":                sym0,
        "token1":                sym1,
        "fee_bps":               pos.fee,
        "fee_pct":               format!("{:.4}%", pos.fee as f64 / 10_000.0),
        "liquidity_to_remove":   liquidity.to_string(),
        "position_liquidity":    pos.liquidity.to_string(),
        "removing_pct":          format!("{}%", pct),
        "uncollected_fees_token0": format_amount(pos.tokens_owed0, dec0),
        "uncollected_fees_token1": format_amount(pos.tokens_owed1, dec1),
        "wallet":                wallet,
        "chain":                 cfg.name,
        "note":                  "Two transactions: decreaseLiquidity then collect",
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to remove liquidity from this position.");
        return Ok(());
    }

    // Tx 1: decreaseLiquidity
    let decrease_data = build_decrease_liquidity(args.token_id, liquidity, deadline);
    let dec_result = wallet_contract_call(chain_id, nfpm, &decrease_data, false, args.dry_run, Some(&wallet)).await?;
    let dec_hash = extract_tx_hash(&dec_result);
    if !args.dry_run {
        eprintln!("[sushiswap-v3] decreaseLiquidity tx: {}", dec_hash);
        sleep(Duration::from_secs(5)).await;
    }

    // Tx 2: collect (sweeps tokensOwed to wallet)
    let collect_data = build_collect(args.token_id, &wallet);
    let col_result = wallet_contract_call(chain_id, nfpm, &collect_data, false, args.dry_run, Some(&wallet)).await?;
    let col_hash = extract_tx_hash(&col_result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "remove-liquidity",
        "token_id":               args.token_id,
        "token0":                 sym0,
        "token1":                 sym1,
        "fee_bps":                pos.fee,
        "liquidity_removed":      liquidity.to_string(),
        "decrease_liquidity_tx":  dec_hash,
        "collect_tx":             col_hash,
        "explorer_decrease":      format!("{}/{}", cfg.explorer, dec_hash),
        "explorer_collect":       format!("{}/{}", cfg.explorer, col_hash),
        "chain":                  cfg.name,
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
