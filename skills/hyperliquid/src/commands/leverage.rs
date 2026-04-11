use clap::Args;
use crate::api::get_asset_meta;
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_update_leverage_action, submit_exchange_request};

#[derive(Args)]
pub struct LeverageArgs {
    /// Coin to set leverage for (e.g. BTC, ETH, SOL)
    #[arg(long)]
    pub coin: String,

    /// Leverage multiplier (e.g. 2 for 2x, 10 for 10x; max 50)
    #[arg(long)]
    pub leverage: u32,

    /// Margin mode: cross or isolated [default: cross]
    #[arg(long, value_parser = ["cross", "isolated"], default_value = "cross")]
    pub mode: String,

    /// Confirm and submit (without this flag, prints a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: LeverageArgs) -> anyhow::Result<()> {
    if args.leverage < 1 || args.leverage > 50 {
        anyhow::bail!("Leverage must be between 1 and 50 (got {})", args.leverage);
    }

    let info = info_url();
    let exchange = exchange_url();
    let coin = normalize_coin(&args.coin);
    let is_cross = args.mode == "cross";
    let nonce = now_ms();

    let (asset_idx, _) = get_asset_meta(info, &coin).await?;

    let action = build_update_leverage_action(asset_idx, is_cross, args.leverage);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "coin": coin,
                "assetIndex": asset_idx,
                "leverage": args.leverage,
                "mode": args.mode,
                "isCross": is_cross
            },
            "action": action
        }))?
    );

    if !args.confirm {
        println!("\n[PREVIEW] Add --confirm to submit this leverage update.");
        return Ok(());
    }

    let wallet = resolve_wallet(CHAIN_ID)?;
    let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "coin": coin,
            "leverage": args.leverage,
            "mode": args.mode,
            "result": result
        }))?
    );

    Ok(())
}
