use clap::Args;
use serde_json::json;

use crate::api;

#[derive(Args)]
pub struct ClosePositionArgs {
    /// Market symbol (e.g. BTC-PERP, ETH-PERP)
    #[arg(long, required = true)]
    pub market: String,

    /// Size to close (in base asset units). Omit or use "all" to close entire position.
    #[arg(long, default_value = "all")]
    pub amount: String,

    /// Wallet address to look up current position details
    #[arg(long)]
    pub wallet: Option<String>,

    /// Broadcast the transaction on-chain (shows preview without this flag)
    #[arg(long)]
    pub confirm: bool,

    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: ClosePositionArgs) -> anyhow::Result<()> {
    let symbol = args.market.to_uppercase();

    // Try to fetch existing position for context
    let existing_position: Option<serde_json::Value> = if let Some(ref wallet) = args.wallet {
        match api::get_user_positions(wallet).await {
            Ok(pos) => {
                pos.as_array()
                    .and_then(|arr| arr.iter().find(|p| p["symbol"].as_str().unwrap_or("") == symbol).cloned())
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Fetch current market price
    let current_price = api::get_ticker(&symbol).await.ok().and_then(|t| {
        let t = if t.is_array() {
            t.as_array()?.first()?.clone()
        } else {
            t
        };
        t["markPrice"]
            .as_str()
            .or_else(|| t["lastPrice"].as_str())
            .and_then(|s| s.parse::<f64>().ok())
    });

    // Determine close size
    let close_amount = if args.amount == "all" {
        existing_position
            .as_ref()
            .and_then(|p| {
                p["positionQty"]
                    .as_str()
                    .or_else(|| p["quantity"].as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "all".to_string())
    } else {
        args.amount.clone()
    };

    // Build preview info
    let pos_side = existing_position
        .as_ref()
        .and_then(|p| p["side"].as_str().or_else(|| p["positionSide"].as_str()))
        .unwrap_or("unknown");

    let entry_price = existing_position
        .as_ref()
        .and_then(|p| p["avgEntryPrice"].as_str().or_else(|| p["entryPrice"].as_str()))
        .unwrap_or("N/A");

    let unrealized_pnl = existing_position
        .as_ref()
        .and_then(|p| p["unrealizedProfit"].as_str().or_else(|| p["unrealizedPnl"].as_str()))
        .unwrap_or("N/A");

    // Estimated close value
    let close_value = current_price.and_then(|p| {
        if close_amount == "all" {
            None
        } else {
            close_amount.parse::<f64>().ok().map(|a| format!("~${:.2}", a * p))
        }
    });

    let preview = json!({
        "action": "close_position",
        "market": symbol,
        "close_amount": close_amount,
        "close_side": if pos_side.to_lowercase() == "buy" || pos_side.to_lowercase() == "long" { "SELL (close long)" } else { "BUY (close short)" },
        "current_entry_price": entry_price,
        "current_mark_price": current_price.map(|p| format!("${:.4}", p)).unwrap_or_else(|| "N/A".to_string()),
        "unrealized_pnl": unrealized_pnl,
        "close_value_usd": close_value.unwrap_or_else(|| "N/A".to_string()),
        "chain": "Sui mainnet",
        "protocol": "Bluefin Pro",
        "reduce_only": true,
        "note": "Bluefin runs on Sui — transactions require a Sui wallet. onchainos does not support Sui writes."
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    if !args.confirm {
        println!("=== PREVIEW: Close {} on {} ===", symbol, "Bluefin");
        println!("{}", serde_json::to_string_pretty(&preview)?);
        println!();
        println!("To execute this transaction:");
        println!("  1. Install Sui CLI: https://docs.sui.io/guides/developer/getting-started/sui-install");
        println!("  2. Use Bluefin SDK: https://github.com/fireflyprotocol/bluefin-v2-client-ts");
        println!();
        println!("Run with --confirm to see the full transaction parameters.");
        return Ok(());
    }

    println!("=== TRANSACTION PARAMETERS (Sui - requires external wallet) ===");
    println!("{}", serde_json::to_string_pretty(&json!({
        "status": "ready_to_sign",
        "warning": "Sui blockchain is not supported by onchainos CLI. Use the Bluefin TypeScript or Python SDK to submit.",
        "transaction": {
            "protocol": "Bluefin Pro",
            "chain": "Sui mainnet",
            "rpc": "https://fullnode.mainnet.sui.io",
            "api_endpoint": "https://dapi.api.sui-prod.bluefin.io",
            "order": {
                "symbol": symbol,
                "side": if pos_side.to_lowercase() == "buy" || pos_side.to_lowercase() == "long" { "SELL" } else { "BUY" },
                "orderType": "MARKET",
                "quantity": close_amount,
                "reduceOnly": true
            }
        },
        "sdk_example": format!(
            "const client = new BluefinClient(true, Networks.PRODUCTION_SUI);\nawait client.init();\nconst order = await client.createSignedOrder({{\n  symbol: MARKET_SYMBOLS.{},\n  side: ORDER_SIDE.{},\n  orderType: ORDER_TYPE.MARKET,\n  quantity: toBigNumber(\"{}\"),\n  reduceOnly: true,\n}});\nawait client.postSignedOrder(order);",
            symbol.replace("-PERP", ""),
            if pos_side.to_lowercase() == "buy" || pos_side.to_lowercase() == "long" { "SELL" } else { "BUY" },
            close_amount
        )
    }))?);

    Ok(())
}
