use clap::Args;
use crate::api::{get_asset_index, get_all_mids, get_clearinghouse_state};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_close_action, submit_exchange_request};

#[derive(Args)]
pub struct CloseArgs {
    /// Coin whose position to close (e.g. BTC, ETH, SOL)
    #[arg(long)]
    pub coin: String,

    /// Close only this many base units instead of the entire position
    #[arg(long)]
    pub size: Option<String>,

    /// Dry run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: CloseArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    let coin = normalize_coin(&args.coin);
    let nonce = now_ms();

    // Look up asset index
    let asset_idx = get_asset_index(info, &coin).await?;

    // Resolve wallet
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Fetch current position to determine direction and full size
    let state = get_clearinghouse_state(info, &wallet).await?;
    let empty_vec = vec![];
    let positions = state["assetPositions"].as_array().unwrap_or(&empty_vec);

    let mut position_szi: Option<f64> = None;
    for pw in positions {
        let pos = &pw["position"];
        if pos["coin"].as_str().map(|c| c.to_uppercase()) == Some(coin.to_uppercase()) {
            if let Some(s) = pos["szi"].as_str() {
                position_szi = s.parse().ok();
                break;
            }
        }
    }

    let szi = position_szi.ok_or_else(|| {
        anyhow::anyhow!("No open {} position found. Use `hyperliquid positions` to check.", coin)
    })?;

    if szi == 0.0 {
        anyhow::bail!("No open {} position (size is 0).", coin);
    }

    let position_is_long = szi > 0.0;
    let position_size = szi.abs();
    let position_side = if position_is_long { "long" } else { "short" };

    // Determine close size
    let close_size = match &args.size {
        Some(s) => {
            let v: f64 = s.parse().map_err(|_| {
                anyhow::anyhow!("Invalid size '{}' — must be a number", s)
            })?;
            if v <= 0.0 {
                anyhow::bail!("Close size must be positive");
            }
            if v > position_size {
                anyhow::bail!(
                    "Close size {} exceeds position size {}",
                    v,
                    position_size
                );
            }
            s.clone()
        }
        None => format!("{}", position_size),
    };

    // Fetch current price for display
    let mids = get_all_mids(info).await?;
    let current_price = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let closing_side = if position_is_long { "sell" } else { "buy" };

    let action = build_close_action(asset_idx, position_is_long, &close_size);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "coin": coin,
                "positionSide": position_side,
                "positionSize": position_size,
                "closingSize": close_size,
                "closingSide": closing_side,
                "currentMidPrice": current_price,
                "type": "market",
                "reduceOnly": true,
                "nonce": nonce
            },
            "action": action
        }))?
    );

    if args.dry_run {
        println!("\n[DRY RUN] Not signed or submitted.");
        return Ok(());
    }

    if !args.confirm {
        println!("\n[PREVIEW] Add --confirm to sign and market-close this position.");
        println!("WARNING: Market orders execute immediately at prevailing price.");
        return Ok(());
    }

    let signed = onchainos_hl_sign(&action, nonce, &wallet, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "action": "close",
            "coin": coin,
            "side": closing_side,
            "size": close_size,
            "result": result
        }))?
    );

    Ok(())
}
