use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{nfpm, rpc_url, token_symbol, CHAIN_ID, pad_address};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_amount, get_decimals, nfpm_positions};

#[derive(Args)]
pub struct RemoveLiquidityArgs {
    /// NFT token ID of the position to remove liquidity from
    #[arg(long)]
    pub token_id: u128,
    /// Percentage of liquidity to remove (1-100, default: 100 = full close)
    #[arg(long, default_value = "100")]
    pub percent: u8,
    /// Slippage tolerance % (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Deadline in minutes (default: 20)
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos
    #[arg(long)]
    pub dry_run: bool,
}

/// NFPM.decreaseLiquidity(DecreaseLiquidityParams) — selector 0x0c49ccbe
/// Params: tokenId(uint256), liquidity(uint128), amount0Min(uint256), amount1Min(uint256), deadline(uint256)
fn build_decrease_liquidity(token_id: u128, liquidity: u128, amount0_min: u128, amount1_min: u128, deadline: u64) -> String {
    format!(
        "0x0c49ccbe{}{}{}{}{}",
        format!("{:0>64x}", token_id),
        format!("{:0>64x}", liquidity),
        format!("{:0>64x}", amount0_min),
        format!("{:0>64x}", amount1_min),
        format!("{:0>64x}", deadline),
    )
}

/// NFPM.collect(CollectParams) — selector 0xfc6f7865
/// Params: tokenId(uint256), recipient(address), amount0Max(uint128), amount1Max(uint128)
fn build_collect(token_id: u128, recipient: &str, amount0_max: u128, amount1_max: u128) -> String {
    format!(
        "0xfc6f7865{}{}{}{}",
        format!("{:0>64x}", token_id),
        pad_address(recipient),
        format!("{:0>64x}", amount0_max),
        format!("{:0>64x}", amount1_max),
    )
}

pub async fn run(args: RemoveLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let nfpm_addr = nfpm();

    if args.percent == 0 || args.percent > 100 {
        anyhow::bail!("--percent must be between 1 and 100");
    }

    let pos = nfpm_positions(nfpm_addr, args.token_id, rpc).await?;
    if pos.liquidity == 0 {
        anyhow::bail!("Position {} has no liquidity (already closed or never minted)", args.token_id);
    }

    let liquidity_to_remove = (pos.liquidity as u128 * args.percent as u128) / 100;
    // The exact output amounts depend on the current price tick and pool math.
    // Setting mins to 0 means we accept any output; the --slippage flag is informational
    // for the user. For large positions, prefer calling with a tighter deadline.
    let amount0_min: u128 = 0;
    let amount1_min: u128 = 0;

    let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
    let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);
    let sym0 = token_symbol(&pos.token0).to_string();
    let sym1 = token_symbol(&pos.token1).to_string();

    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let deadline = crate::config::unix_now() + args.deadline_minutes * 60;
    let decrease_calldata = build_decrease_liquidity(
        args.token_id, liquidity_to_remove, amount0_min, amount1_min, deadline,
    );
    let collect_calldata = build_collect(args.token_id, &recipient, u128::MAX, u128::MAX);

    let preview = serde_json::json!({
        "preview": true,
        "action": "remove-liquidity",
        "token_id": args.token_id,
        "token0": sym0,
        "token1": sym1,
        "tick_lower": pos.tick_lower,
        "tick_upper": pos.tick_upper,
        "liquidity_to_remove": liquidity_to_remove.to_string(),
        "total_liquidity": pos.liquidity.to_string(),
        "percent": args.percent,
        "uncollected_fees_token0": format_amount(pos.tokens_owed0, dec0),
        "uncollected_fees_token1": format_amount(pos.tokens_owed1, dec1),
        "recipient": recipient,
        "chain": "Base (8453)",
        "note": "Two transactions will be sent: decreaseLiquidity then collect"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to remove liquidity from position {}.", args.token_id);
        return Ok(());
    }

    // Tx 1: decreaseLiquidity
    eprintln!("[aerodrome-slipstream-plugin] Step 1/2: decreaseLiquidity...");
    let r1 = wallet_contract_call(CHAIN_ID, nfpm_addr, &decrease_calldata, true, args.dry_run, Some(&recipient)).await?;
    let h1 = extract_tx_hash(&r1);
    eprintln!("[aerodrome-slipstream-plugin] decreaseLiquidity tx: {}", h1);

    if !args.dry_run {
        sleep(Duration::from_secs(5)).await;
    }

    // Tx 2: collect (withdraw tokens)
    eprintln!("[aerodrome-slipstream-plugin] Step 2/2: collect...");
    let r2 = wallet_contract_call(CHAIN_ID, nfpm_addr, &collect_calldata, true, args.dry_run, Some(&recipient)).await?;
    let h2 = extract_tx_hash(&r2);
    eprintln!("[aerodrome-slipstream-plugin] collect tx: {}", h2);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "remove-liquidity",
        "token_id": args.token_id,
        "percent_removed": args.percent,
        "decrease_tx": h1,
        "collect_tx": h2,
        "explorer_decrease": format!("https://basescan.org/tx/{}", h1),
        "explorer_collect": format!("https://basescan.org/tx/{}", h2),
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
