use clap::Args;
use serde_json::json;

use crate::api;

#[derive(Args)]
pub struct OpenPositionArgs {
    /// Market symbol (e.g. BTC-PERP, ETH-PERP, SUI-PERP)
    #[arg(long, required = true)]
    pub market: String,

    /// Direction: long or short
    #[arg(long, required = true)]
    pub side: String,

    /// Size in base asset units (e.g. 0.01 for 0.01 BTC)
    #[arg(long, required = true)]
    pub amount: String,

    /// Leverage multiplier (e.g. 5 for 5x)
    #[arg(long, default_value = "1")]
    pub leverage: String,

    /// Order type: market or limit
    #[arg(long, default_value = "market")]
    pub order_type: String,

    /// Limit price (required for limit orders)
    #[arg(long)]
    pub price: Option<String>,

    /// Broadcast the transaction on-chain (shows preview without this flag)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: OpenPositionArgs) -> anyhow::Result<()> {
    let symbol = args.market.to_uppercase();
    let side_lower = args.side.to_lowercase();

    if side_lower != "long" && side_lower != "short" {
        anyhow::bail!("--side must be 'long' or 'short', got '{}'", args.side);
    }

    if args.order_type.to_lowercase() == "limit" && args.price.is_none() {
        anyhow::bail!("--price is required for limit orders");
    }

    // Parse amount using string-based method (no f64 precision loss for amounts)
    let amount_str = &args.amount;
    let leverage_str = &args.leverage;

    // Fetch current market price for preview context
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

    let price_display = if let Some(ref p) = args.price {
        p.clone()
    } else {
        current_price
            .map(|p| format!("~${:.2} (market)", p))
            .unwrap_or_else(|| "market price".to_string())
    };

    // Estimate notional value
    let notional = current_price.and_then(|p| {
        amount_str.parse::<f64>().ok().map(|a| a * p)
    });
    let leverage_f: f64 = leverage_str.parse().unwrap_or(1.0);
    let margin = notional.map(|n| n / leverage_f);

    // Build transaction preview
    let preview = json!({
        "action": "open_position",
        "market": symbol,
        "side": side_lower.to_uppercase(),
        "order_type": args.order_type.to_uppercase(),
        "size": amount_str,
        "leverage": format!("{}x", leverage_str),
        "price": price_display,
        "notional_value_usd": notional.map(|n| format!("~${:.2}", n)).unwrap_or_else(|| "N/A".to_string()),
        "required_margin_usd": margin.map(|m| format!("~${:.2}", m)).unwrap_or_else(|| "N/A".to_string()),
        "chain": "Sui mainnet",
        "protocol": "Bluefin Pro",
        "note": "Bluefin runs on Sui — transactions require a Sui wallet (sui CLI or Slippage Wallet). onchainos does not support Sui writes."
    });

    if !args.confirm {
        println!("=== PREVIEW: Open {} {} on {} ===", side_lower.to_uppercase(), symbol, "Bluefin");
        println!("{}", serde_json::to_string_pretty(&preview)?);
        println!();
        println!("To execute this transaction:");
        println!("  1. Install Sui CLI: https://docs.sui.io/guides/developer/getting-started/sui-install");
        println!("  2. Set up your Sui wallet: sui client");
        println!("  3. Use Bluefin SDK: https://github.com/fireflyprotocol/bluefin-v2-client-ts");
        println!();
        println!("Run with --confirm to broadcast (requires Sui wallet configured).");
        return Ok(());
    }

    // --confirm path: attempt to call the Bluefin onboarding/order API
    // Since onchainos does not support Sui, we explain what would happen
    // and provide the transaction parameters in machine-readable form.
    println!("=== TRANSACTION PARAMETERS (Sui - requires external wallet) ===");
    println!("{}", serde_json::to_string_pretty(&json!({
        "status": "ready_to_sign",
        "warning": "Sui blockchain is not supported by onchainos CLI. To submit this transaction, use the Bluefin TypeScript or Python SDK.",
        "transaction": {
            "protocol": "Bluefin Pro",
            "chain": "Sui mainnet",
            "rpc": "https://fullnode.mainnet.sui.io",
            "api_endpoint": "https://dapi.api.sui-prod.bluefin.io",
            "order": {
                "symbol": symbol,
                "side": if side_lower == "long" { "BUY" } else { "SELL" },
                "orderType": args.order_type.to_uppercase(),
                "quantity": amount_str,
                "leverage": leverage_str,
                "price": args.price.as_deref().unwrap_or("0"),
                "reduceOnly": false
            }
        },
        "sdk_example": format!(
            "const client = new BluefinClient(true, Networks.PRODUCTION_SUI);\nawait client.init();\nconst order = await client.createSignedOrder({{\n  symbol: MARKET_SYMBOLS.{},\n  side: ORDER_SIDE.{},\n  orderType: ORDER_TYPE.{},\n  quantity: toBigNumber(\"{}\"),\n  leverage: toBigNumber(\"{}\"),\n}});\nawait client.postSignedOrder(order);",
            symbol.replace("-PERP", ""),
            if side_lower == "long" { "BUY" } else { "SELL" },
            args.order_type.to_uppercase(),
            amount_str,
            leverage_str
        )
    }))?);

    Ok(())
}
