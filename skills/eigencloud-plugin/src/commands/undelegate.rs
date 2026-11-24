use clap::Args;
use crate::abi::selector;
use crate::chain::CHAIN_ID;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};

const DELEGATION_MANAGER: &str = "0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A";

#[derive(Args)]
pub struct UndelegateArgs {
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Dry-run: build calldata without calling onchainos
    #[arg(long, conflicts_with = "confirm")]
    pub dry_run: bool,
}

pub async fn run(args: UndelegateArgs) -> anyhow::Result<()> {
    let wallet = if args.dry_run {
        resolve_wallet(CHAIN_ID).unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    } else if args.confirm {
        resolve_wallet(CHAIN_ID)?
    } else {
        resolve_wallet(CHAIN_ID).unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    };

    // undelegate(address staker) — queues all shares for withdrawal and removes delegation
    let sel = selector("undelegate(address)");
    let staker = crate::abi::encode_address(&wallet);
    let mut data = sel.to_vec();
    data.extend_from_slice(&staker);
    let calldata = format!("0x{}", hex::encode(data));

    let preview = serde_json::json!({
        "preview":            !args.confirm && !args.dry_run,
        "action":             "undelegate",
        "wallet":             wallet,
        "delegation_manager": DELEGATION_MANAGER,
        "warning":            "Undelegating queues ALL restaked shares for withdrawal. You must wait 7 days then complete each withdrawal separately.",
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast this undelegation transaction.");
        eprintln!("WARNING: This queues ALL your restaked positions for withdrawal (7-day delay).");
        return Ok(());
    }

    eprintln!("[eigencloud] Undelegating — queuing all shares for withdrawal...");
    let result = wallet_contract_call(CHAIN_ID, DELEGATION_MANAGER, &calldata, "0", args.dry_run, Some(&wallet))?;
    let tx_hash = extract_tx_hash(&result);
    eprintln!("[eigencloud] undelegate tx: {}", tx_hash);

    let mut out = serde_json::json!({
        "ok":       true,
        "action":   "undelegate",
        "wallet":   wallet,
        "tx_hash":  tx_hash,
        "next_step": "After 7 days, complete your withdrawal via the EigenLayer app at app.eigenlayer.xyz",
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
