use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct ClaimWithdrawalArgs {
    /// Comma-separated list of request IDs to claim (e.g. 12345,67890)
    #[arg(long, value_delimiter = ',')]
    pub ids: Vec<u128>,

    /// Wallet address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: ClaimWithdrawalArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    if args.ids.is_empty() {
        anyhow::bail!("No request IDs provided. Use --ids <ID1,ID2,...>");
    }

    // Resolve wallet address — must not be zero
    let wallet = args
        .from
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    println!("=== Lido Claim Withdrawal ===");
    println!("From:        {}", wallet);
    println!("Request IDs: {:?}", args.ids);

    // Step 1: getLastCheckpointIndex() -> uint256
    println!("\nStep 1/3: Getting last checkpoint index...");
    let checkpoint_calldata = format!("0x{}", config::SEL_GET_LAST_CHECKPOINT_INDEX);
    let checkpoint_result = onchainos::eth_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &checkpoint_calldata,
    ).await?;

    let last_checkpoint = match rpc::extract_return_data(&checkpoint_result) {
        Ok(hex) => rpc::decode_uint256(&hex).unwrap_or(1) as u64,
        Err(e) => {
            anyhow::bail!("Failed to get last checkpoint index: {}", e);
        }
    };
    println!("Last checkpoint index: {}", last_checkpoint);

    // Step 2a: getWithdrawalStatus — filter out PENDING / CLAIMED before calling findCheckpointHints.
    // findCheckpointHints reverts with an opaque error on any non-finalized request ID.
    println!("Step 2/3: Checking request status...");
    let status_calldata = rpc::calldata_get_withdrawal_status(&args.ids);
    let status_result = onchainos::eth_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &status_calldata,
    ).await?;

    if let Ok(hex) = rpc::extract_return_data(&status_result) {
        let hex = hex.trim_start_matches("0x");
        let data = if hex.len() > 128 { &hex[128..] } else { hex };
        let entry_size = 6 * 64;
        let mut pending_ids: Vec<u128> = vec![];
        let mut already_claimed: Vec<u128> = vec![];
        for (i, &id) in args.ids.iter().enumerate() {
            let start = i * entry_size;
            if start + entry_size > data.len() {
                break;
            }
            let entry = &data[start..start + entry_size];
            let is_finalized = u128::from_str_radix(&entry[4 * 64..5 * 64], 16).unwrap_or(0) != 0;
            let is_claimed   = u128::from_str_radix(&entry[5 * 64..6 * 64], 16).unwrap_or(0) != 0;
            if is_claimed {
                already_claimed.push(id);
            } else if !is_finalized {
                pending_ids.push(id);
            }
        }
        if !already_claimed.is_empty() {
            eprintln!("Warning: already claimed — skipping: {:?}", already_claimed);
        }
        if !pending_ids.is_empty() {
            anyhow::bail!(
                "The following requests are not yet finalized (still PENDING): {:?}\n\
                 Run `lido get-withdrawals` to check status. Withdrawal finalization typically takes 1–5 days.",
                pending_ids
            );
        }
    }

    // Step 2b: findCheckpointHints(uint256[] requestIds, uint256 firstIndex, uint256 lastIndex)
    println!("  Finding checkpoint hints...");
    let hints_calldata =
        rpc::calldata_find_checkpoint_hints(&args.ids, 1, last_checkpoint);
    let hints_result = onchainos::eth_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &hints_calldata,
    ).await?;

    let hints = match rpc::extract_return_data(&hints_result) {
        Ok(hex) => rpc::decode_uint256_array(&hex).unwrap_or_default(),
        Err(e) => {
            anyhow::bail!("Failed to get checkpoint hints: {}", e);
        }
    };

    if hints.len() != args.ids.len() {
        anyhow::bail!(
            "Hint count ({}) does not match ID count ({}). Some requests may not be finalized.",
            hints.len(),
            args.ids.len()
        );
    }
    println!("Hints: {:?}", hints);

    // Step 3: claimWithdrawals(uint256[] requestIds, uint256[] hints)
    let claim_calldata = rpc::calldata_claim_withdrawals(&args.ids, &hints);

    println!("\nStep 3/3: Claiming withdrawals");
    println!("  Contract:  {}", config::WITHDRAWAL_QUEUE_ADDRESS);
    println!("  Calldata:  {}", claim_calldata);

    if args.dry_run {
        println!("\n[dry-run] Transaction NOT submitted.");
        return Ok(());
    }

    // ── Preview mode: show TX details without broadcasting ──────────────────
    if !args.confirm && !args.dry_run {
        println!("=== Transaction Preview (NOT broadcast) ===");
        println!("Add --confirm to execute this transaction.");
        return Ok(());
    }
    let claim_result = onchainos::wallet_contract_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &claim_calldata,
        Some(&wallet),
        None,
        args.confirm,
        args.dry_run,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&claim_result);
    println!("\nClaim transaction submitted: {}", tx_hash);
    println!("ETH will be sent to your wallet. The unstETH NFT(s) are burned.");

    Ok(())
}
