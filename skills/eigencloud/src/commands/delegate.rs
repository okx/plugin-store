use clap::Args;
use serde_json::json;

use crate::config::{build_delegate_calldata, chain_id, delegation_manager, rpc_url};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{get_delegated_operator, is_operator};

#[derive(Args)]
pub struct DelegateArgs {
    /// Operator address to delegate to
    #[arg(long)]
    pub operator: String,

    /// Broadcast the transaction (required to execute; omit to preview)
    #[arg(long)]
    pub confirm: bool,

    /// Simulate without broadcasting (for testing)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: DelegateArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let dm = delegation_manager();
    let cid = chain_id();

    // Validate operator address format
    if !args.operator.starts_with("0x") || args.operator.len() != 42 {
        anyhow::bail!("Invalid operator address: '{}' — must be a 42-character hex address starting with 0x", args.operator);
    }

    let wallet = resolve_wallet(cid)?;

    // Check current delegation
    let current_operator = get_delegated_operator(dm, &wallet, rpc).await?;
    if current_operator != "0x0000000000000000000000000000000000000000" {
        anyhow::bail!(
            "Already delegated to operator {}. Call queue-withdraw to undelegate first.",
            current_operator
        );
    }

    // Verify target is a registered operator
    let operator_registered = is_operator(dm, &args.operator, rpc).await?;
    if !args.dry_run && !operator_registered {
        anyhow::bail!(
            "Address {} is not a registered EigenLayer operator",
            args.operator
        );
    }

    let calldata = build_delegate_calldata(&args.operator);

    let result = wallet_contract_call(cid, dm, &calldata, 0, args.confirm, args.dry_run).await?;
    let tx_hash = extract_tx_hash(&result).to_string();

    let output = json!({
        "ok": true,
        "action": "delegate",
        "wallet": wallet,
        "operator": args.operator,
        "operatorRegistered": operator_registered,
        "delegationManager": dm,
        "txHash": tx_hash,
        "confirmed": args.confirm,
        "dryRun": args.dry_run
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
