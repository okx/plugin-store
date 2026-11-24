use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{build_approve_calldata, nfpm, rpc_url, token_symbol, unix_now, CHAIN_ID};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_amount, get_allowance, get_decimals, nfpm_positions, parse_human_amount};

#[derive(Args)]
pub struct AddLiquidityArgs {
    /// NFT token ID of the existing position
    #[arg(long)]
    pub token_id: u128,
    /// Additional amount of token0 to add (human-readable)
    #[arg(long)]
    pub amount0: String,
    /// Additional amount of token1 to add (human-readable)
    #[arg(long)]
    pub amount1: String,
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

/// NFPM.increaseLiquidity(IncreaseLiquidityParams) — selector 0x219f5d17
/// Params: tokenId(uint256), amount0Desired(uint256), amount1Desired(uint256), amount0Min(uint256), amount1Min(uint256), deadline(uint256)
fn build_increase_liquidity(
    token_id: u128,
    amount0_desired: u128, amount1_desired: u128,
    amount0_min: u128, amount1_min: u128,
    deadline: u64,
) -> String {
    format!(
        "0x219f5d17{}{}{}{}{}{}",
        format!("{:0>64x}", token_id),
        format!("{:0>64x}", amount0_desired),
        format!("{:0>64x}", amount1_desired),
        format!("{:0>64x}", amount0_min),
        format!("{:0>64x}", amount1_min),
        format!("{:0>64x}", deadline),
    )
}

pub async fn run(args: AddLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let nfpm_addr = nfpm();

    let pos = nfpm_positions(nfpm_addr, args.token_id, rpc).await?;
    let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
    let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);
    let sym0 = token_symbol(&pos.token0).to_string();
    let sym1 = token_symbol(&pos.token1).to_string();

    let amount0_desired = parse_human_amount(&args.amount0, dec0)?;
    let amount1_desired = parse_human_amount(&args.amount1, dec1)?;
    // For LP positions, the NFPM adjusts actual token ratios based on current pool price.
    // Fixed-percentage minimums cause PSC failures when ratios differ from desired.
    let amount0_min: u128 = 0;
    let amount1_min: u128 = 0;

    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_increase_liquidity(
        args.token_id, amount0_desired, amount1_desired, amount0_min, amount1_min, deadline,
    );

    let preview = serde_json::json!({
        "preview": true,
        "action": "add-liquidity",
        "token_id": args.token_id,
        "token0": sym0,
        "token1": sym1,
        "tick_lower": pos.tick_lower,
        "tick_upper": pos.tick_upper,
        "amount0_desired": format_amount(amount0_desired, dec0),
        "amount1_desired": format_amount(amount1_desired, dec1),
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to increase liquidity in position {}.", args.token_id);
        return Ok(());
    }

    if !args.dry_run {
        let allow0 = get_allowance(&pos.token0, &recipient, nfpm_addr, rpc).await?;
        if allow0 < amount0_desired {
            eprintln!("[aerodrome-slipstream-plugin] Approving {} for NFPM...", sym0);
            let approve0 = build_approve_calldata(nfpm_addr, amount0_desired);
            let r = wallet_contract_call(CHAIN_ID, &pos.token0, &approve0, true, false, Some(&recipient)).await?;
            eprintln!("[aerodrome-slipstream-plugin] Approve {} tx: {}", sym0, extract_tx_hash(&r));
            sleep(Duration::from_secs(5)).await;
        }
        let allow1 = get_allowance(&pos.token1, &recipient, nfpm_addr, rpc).await?;
        if allow1 < amount1_desired {
            eprintln!("[aerodrome-slipstream-plugin] Approving {} for NFPM...", sym1);
            let approve1 = build_approve_calldata(nfpm_addr, amount1_desired);
            let r = wallet_contract_call(CHAIN_ID, &pos.token1, &approve1, true, false, Some(&recipient)).await?;
            eprintln!("[aerodrome-slipstream-plugin] Approve {} tx: {}", sym1, extract_tx_hash(&r));
            sleep(Duration::from_secs(5)).await;
        }
    }

    let result = wallet_contract_call(CHAIN_ID, nfpm_addr, &calldata, true, args.dry_run, Some(&recipient)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "add-liquidity",
        "token_id": args.token_id,
        "amount0_desired": format_amount(amount0_desired, dec0),
        "amount1_desired": format_amount(amount1_desired, dec1),
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
