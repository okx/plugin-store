use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{nfpm_address, pad_address, pad_u256, rpc_url, uint128_max_padded, unix_now};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::nfpm_positions;

#[derive(Args)]
pub struct RemoveLiquidityArgs {
    /// Token ID of the LP position NFT
    #[arg(long)]
    pub token_id: u128,
    /// Amount of liquidity to remove. Omit to remove all liquidity.
    #[arg(long)]
    pub liquidity: Option<u128>,
    /// Minimum acceptable amount0 (0 for no minimum)
    #[arg(long, default_value = "0")]
    pub amount0_min: u128,
    /// Minimum acceptable amount1 (0 for no minimum)
    #[arg(long, default_value = "0")]
    pub amount1_min: u128,
    /// Transaction deadline in minutes from now
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Also burn the NFT after removing all liquidity
    #[arg(long)]
    pub burn: bool,
    /// Chain ID (default: 8453 = Base)
    #[arg(long, default_value = "8453")]
    pub chain: u64,
    /// Dry run — build calldata without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: RemoveLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let nfpm = nfpm_address(args.chain);

    // --- 1. Fetch current position data ---
    let pos = nfpm_positions(nfpm, args.token_id, &rpc).await?;
    println!(
        "Position #{}: token0={} token1={} fee={} liquidity={}",
        args.token_id, pos.token0, pos.token1, pos.fee, pos.liquidity
    );

    let liquidity_to_remove = args.liquidity.unwrap_or(pos.liquidity);
    if liquidity_to_remove == 0 && pos.liquidity == 0 {
        println!("Position has no liquidity. Will attempt collect to sweep any remaining fees...");
    }

    println!(
        "Removing liquidity={} from position #{}",
        liquidity_to_remove, args.token_id
    );
    println!("Please confirm the remove-liquidity parameters above before proceeding. (Proceeding automatically in non-interactive mode)");

    // --- 2. Resolve recipient ---
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(args.chain)?
    };

    let deadline = unix_now() + args.deadline_minutes * 60;

    // --- 3. decreaseLiquidity ---
    // decreaseLiquidity((uint256 tokenId, uint128 liquidity, uint256 amount0Min,
    //   uint256 amount1Min, uint256 deadline))
    // Selector: 0x0c49ccbe
    let decrease_calldata = format!(
        "0x0c49ccbe{}{}{}{}{}",
        pad_u256(args.token_id),
        pad_u256(liquidity_to_remove),
        pad_u256(args.amount0_min),
        pad_u256(args.amount1_min),
        pad_u256(deadline as u128),
    );

    let decrease_result =
        wallet_contract_call(args.chain, nfpm, &decrease_calldata, None, None, true, args.dry_run).await?;
    let decrease_tx = extract_tx_hash(&decrease_result).to_string();
    println!("decreaseLiquidity tx: {}", decrease_tx);

    // Wait for decreaseLiquidity nonce to clear before collect
    if !args.dry_run {
        sleep(Duration::from_secs(5)).await;
    }

    // --- 4. collect ---
    // collect((uint256 tokenId, address recipient, uint128 amount0Max, uint128 amount1Max))
    // Selector: 0xfc6f7865
    let max = uint128_max_padded();
    let collect_calldata = format!(
        "0xfc6f7865{}{}{}{}",
        pad_u256(args.token_id),
        pad_address(&recipient),
        max,
        max,
    );

    let collect_result =
        wallet_contract_call(args.chain, nfpm, &collect_calldata, None, None, true, args.dry_run).await?;
    let collect_tx = extract_tx_hash(&collect_result).to_string();
    println!("collect tx: {}", collect_tx);

    // --- 5. Optional burn ---
    let mut burn_tx = "".to_string();
    if args.burn && liquidity_to_remove >= pos.liquidity {
        // burn(uint256 tokenId) — selector 0x42966c68
        let burn_calldata = format!("0x42966c68{}", pad_u256(args.token_id));
        let burn_result =
            wallet_contract_call(args.chain, nfpm, &burn_calldata, None, None, true, args.dry_run).await?;
        burn_tx = extract_tx_hash(&burn_result).to_string();
        println!("burn tx: {}", burn_tx);
    }

    println!(
        "{{\"ok\":true,\"tokenId\":{},\"decreaseTx\":\"{}\",\"collectTx\":\"{}\",\"burnTx\":\"{}\"}}",
        args.token_id, decrease_tx, collect_tx, burn_tx
    );

    Ok(())
}
