use clap::Args;
use crate::config::{nfpm_address, pad_address, pad_u256, rpc_url, uint128_max_padded};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::nfpm_positions;

#[derive(Args)]
pub struct CollectFeesArgs {
    /// Token ID of the LP position NFT
    #[arg(long)]
    pub token_id: u128,
    /// Chain ID (default: 8453 = Base)
    #[arg(long, default_value = "8453")]
    pub chain: u64,
    /// Dry run — build calldata without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: CollectFeesArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let nfpm = nfpm_address(args.chain);

    // --- 1. Fetch position to show fee info ---
    let pos = nfpm_positions(nfpm, args.token_id, &rpc).await?;
    println!(
        "Position #{}: token0={} token1={} tokensOwed0={} tokensOwed1={}",
        args.token_id, pos.token0, pos.token1, pos.tokens_owed0, pos.tokens_owed1
    );

    if pos.tokens_owed0 == 0 && pos.tokens_owed1 == 0 {
        println!("No fees owed for position #{}. Nothing to collect.", args.token_id);
        println!("{{\"ok\":true,\"collected\":false,\"reason\":\"no fees owed\"}}");
        return Ok(());
    }

    println!(
        "Collecting fees: tokensOwed0={} tokensOwed1={} from position #{}",
        pos.tokens_owed0, pos.tokens_owed1, args.token_id
    );
    println!("Please confirm the fee collection above before proceeding. (Proceeding automatically in non-interactive mode)");

    // --- 2. Resolve recipient ---
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(args.chain)?
    };

    // --- 3. Build collect calldata ---
    // collect((uint256 tokenId, address recipient, uint128 amount0Max, uint128 amount1Max))
    // Selector: 0xfc6f7865
    let max = uint128_max_padded();
    let calldata = format!(
        "0xfc6f7865{}{}{}{}",
        pad_u256(args.token_id),
        pad_address(&recipient),
        max,
        max,
    );

    let result =
        wallet_contract_call(args.chain, nfpm, &calldata, None, None, true, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"tokenId\":{},\"recipient\":\"{}\"}}",
        tx_hash, args.token_id, recipient
    );

    Ok(())
}
