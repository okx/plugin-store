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
    eprintln!("Ask user to confirm before proceeding.");

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
