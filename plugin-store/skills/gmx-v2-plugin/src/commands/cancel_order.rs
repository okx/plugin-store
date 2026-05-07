use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct CancelOrderArgs {
    /// Order key (bytes32 hex, from get-orders)
    #[arg(long)]
    pub key: String,

    /// Wallet address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(chain: &str, dry_run: bool, confirm: bool, args: CancelOrderArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.from.clone().unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Validate the key looks like a bytes32
    let key_clean = args.key.trim_start_matches("0x");
    if key_clean.len() != 64 {
        anyhow::bail!("Order key must be a 32-byte hex string (64 hex chars). Got: '{}'", args.key);
    }

    let calldata_hex = crate::abi::encode_cancel_order(&args.key);
    let calldata = format!("0x{}", calldata_hex);

    eprintln!("=== Cancel Order Preview ===");
    eprintln!("Order key: {}", args.key);
    eprintln!("Exchange router: {}", cfg.exchange_router);
    if !confirm { eprintln!("Add --confirm to broadcast."); }

    if !confirm && !dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "status": "preview",
                "message": "Add --confirm to broadcast this transaction",
                "chain": chain,
                "orderKey": args.key,
                "calldata": calldata
            }))?
        );
        return Ok(());
    }

    let result = crate::onchainos::wallet_contract_call_with_gas(
        cfg.chain_id,
        cfg.exchange_router,
        &calldata,
        Some(&wallet),
        None,
        dry_run,
        confirm,
        Some(300_000),
    ).await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&result);

    // G17: verify the cancel tx actually landed on-chain before reporting ok:true
    if !dry_run {
        if tx_hash == "pending" || tx_hash.is_empty() {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "error": "TX_NOT_SUBMITTED",
                    "reason": "onchainos did not return a tx hash — transaction may not have been submitted",
                    "chain": chain,
                    "orderKey": args.key
                }))?
            );
            return Ok(());
        }
        crate::onchainos::wait_for_tx(cfg.chain_id, &tx_hash, &wallet, 60)?;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "dry_run": dry_run,
            "chain": chain,
            "txHash": tx_hash,
            "orderKey": args.key,
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
