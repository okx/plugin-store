use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{nfpm, pad_address, pad_u256, rpc_url, unix_now};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::nfpm_positions;

#[derive(Args)]
pub struct RemoveLiquidityArgs {
    /// NFT position token ID
    #[arg(long)]
    pub token_id: u128,
    /// Liquidity amount to remove (use positions command to get current liquidity)
    #[arg(long)]
    pub liquidity: u128,
    /// Minimum amount0 to receive (0 = no minimum)
    #[arg(long, default_value = "0")]
    pub amount0_min: u128,
    /// Minimum amount1 to receive (0 = no minimum)
    #[arg(long, default_value = "0")]
    pub amount1_min: u128,
    /// Transaction deadline in minutes from now
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Chain ID (default: 42161 Arbitrum)
    #[arg(long, default_value = "42161")]
    pub chain: u64,
    /// Dry run — build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: RemoveLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let nfpm_addr = nfpm(args.chain)?;
    let deadline = unix_now() + args.deadline_minutes * 60;

    // Fetch current position info (skip in dry-run)
    let liquidity_to_remove = if args.dry_run {
        args.liquidity
    } else {
        let pos = nfpm_positions(nfpm_addr, args.token_id, &rpc).await?;
        let current_liquidity: u128 = pos["liquidity"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        if current_liquidity == 0 {
            anyhow::bail!("Position {} has zero liquidity", args.token_id);
        }
        args.liquidity.min(current_liquidity)
    };

    // Resolve recipient
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(args.chain)?
    };

    eprintln!(
        "Remove liquidity: tokenId={} liquidity={}",
        args.token_id, liquidity_to_remove
    );
    eprintln!("Ask user to confirm before proceeding with remove-liquidity.");

    // Step 1: decreaseLiquidity
    // DecreaseLiquidityParams: (uint256 tokenId, uint128 liquidity, uint256 amount0Min, uint256 amount1Min, uint256 deadline)
    // Selector: 0x0c49ccbe (verified)
    let decrease_calldata = format!(
        "0x0c49ccbe{}{}{}{}{}",
        pad_u256(args.token_id),
        pad_u256(liquidity_to_remove),
        pad_u256(args.amount0_min),
        pad_u256(args.amount1_min),
        pad_u256(deadline as u128)
    );

    let decrease_result =
        wallet_contract_call(args.chain, nfpm_addr, &decrease_calldata, true, args.dry_run).await?;
    let decrease_tx = extract_tx_hash(&decrease_result);

    if !args.dry_run {
        if !decrease_result["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("decreaseLiquidity failed: {}", decrease_result);
        }
        sleep(Duration::from_secs(5)).await;
    }

    // Step 2: collect
    // CollectParams: (uint256 tokenId, address recipient, uint128 amount0Max, uint128 amount1Max)
    // Selector: 0xfc6f7865 (verified)
    // Use u128::MAX for both amounts to collect all available
    let amount_max = u128::MAX;
    let collect_calldata = format!(
        "0xfc6f7865{}{}{}{}",
        pad_u256(args.token_id),
        pad_address(&recipient),
        pad_u256(amount_max),
        pad_u256(amount_max)
    );

    let collect_result =
        wallet_contract_call(args.chain, nfpm_addr, &collect_calldata, true, args.dry_run).await?;
    let collect_tx = extract_tx_hash(&collect_result);

    let output = serde_json::json!({
        "ok": true,
        "dry_run": args.dry_run,
        "data": {
            "token_id": args.token_id,
            "liquidity_removed": liquidity_to_remove.to_string(),
            "decrease_liquidity_tx": decrease_tx,
            "collect_tx": collect_tx,
            "recipient": recipient,
            "chain_id": args.chain
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
