use clap::Args;
use crate::api::{get_asset_index, get_open_orders};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_cancel_action, submit_exchange_request};

#[derive(Args)]
pub struct CancelArgs {
    /// Order ID to cancel
    #[arg(long)]
    pub order_id: u64,

    /// Coin symbol (e.g. BTC, ETH). Required to determine asset index.
    #[arg(long)]
    pub coin: String,

    /// Dry run — preview cancel payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit the cancellation (without this flag, prints a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: CancelArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    let coin = normalize_coin(&args.coin);
    let nonce = now_ms();

    // Look up asset index
    let asset_idx = get_asset_index(info, &coin).await?;

    // Build cancel action
    let action = build_cancel_action(asset_idx, args.order_id);

    // Show preview
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "coin": coin,
                "assetIndex": asset_idx,
                "orderId": args.order_id,
                "nonce": nonce
            },
            "action": action
        }))?
    );

    if args.dry_run {
        println!("\n[DRY RUN] Cancel not signed or submitted.");
        return Ok(());
    }

    if !args.confirm {
        println!("\n[PREVIEW] Add --confirm to sign and submit this cancellation.");
        return Ok(());
    }

    // Resolve wallet
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Verify the order exists before cancelling
    let open_orders = get_open_orders(info, &wallet).await?;
    let order_exists = open_orders
        .as_array()
        .map(|arr| arr.iter().any(|o| o["oid"].as_u64() == Some(args.order_id)))
        .unwrap_or(false);

    if !order_exists {
        eprintln!(
            "Warning: order {} not found in open orders for {}. Proceeding anyway.",
            args.order_id, wallet
        );
    }

    // Sign via onchainos
    let signed = onchainos_hl_sign(&action, nonce, &wallet, true, false)?;

    // Submit to exchange
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "coin": coin,
            "orderId": args.order_id,
            "result": result
        }))?
    );

    Ok(())
}
