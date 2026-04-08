use anyhow::Result;
use clap::Args;

use crate::api::fetch_latest_package_id;
use crate::config::{DEFAULT_PROTOCOL_PACKAGE, STORAGE_ID, PRICE_ORACLE_ID, INCENTIVE_V2_ID, CLOCK_ID, find_pool};

#[derive(Args, Debug)]
pub struct RepayArgs {
    /// Asset to repay (e.g. SUI, USDC, USDT, WETH)
    #[arg(long)]
    pub asset: String,

    /// Amount to repay (in token units). Use "max" for full repayment.
    #[arg(long)]
    pub amount: String,

    /// Your Sui wallet address
    #[arg(long)]
    pub wallet: Option<String>,

    /// Execute the transaction (not supported — Sui write ops require native support)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: RepayArgs) -> Result<()> {
    // Validate amount
    if args.amount.to_lowercase() != "max" {
        let amount_f: f64 = args.amount.parse::<f64>()
            .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'. Must be a number or 'max'.", args.amount))?;
        if amount_f <= 0.0 {
            anyhow::bail!("Amount must be positive.");
        }
    }

    // Resolve pool
    let pool = find_pool(&args.asset)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown asset '{}'. Supported: SUI, wUSDC, USDT, WETH, CETUS, NAVX, nUSDC, ETH, suiUSDT",
            args.asset
        ))?;

    let (symbol, asset_id, pool_id, coin_type) = pool;

    // Fetch latest package ID
    let package_id = fetch_latest_package_id()
        .await
        .unwrap_or_else(|_| DEFAULT_PROTOCOL_PACKAGE.to_string());

    let amount_description = if args.amount.to_lowercase() == "max" {
        "full outstanding debt".to_string()
    } else {
        format!("{} {}", args.amount, symbol)
    };

    let preview = serde_json::json!({
        "preview": true,
        "action": "repay",
        "protocol": "NAVI Protocol",
        "chain": "Sui Mainnet",
        "asset": symbol,
        "asset_id": asset_id,
        "amount": amount_description,
        "wallet": args.wallet.as_deref().unwrap_or("<your-sui-address>"),
        "move_call": {
            "package": package_id,
            "module": "lending_logic",
            "function": "repay",
            "type_args": [coin_type],
            "args": [
                {"name": "storage", "id": STORAGE_ID},
                {"name": "pool", "id": pool_id},
                {"name": "oracle", "id": PRICE_ORACLE_ID},
                {"name": "incentive_v2", "id": INCENTIVE_V2_ID},
                {"name": "clock", "id": CLOCK_ID},
                {"name": "coin_object", "description": "coin object to repay with"},
                {"name": "amount", "description": "repay amount in base units (use very large number for full repay)"}
            ]
        },
        "tip": "Repaying increases your health factor and reduces liquidation risk.",
        "note": "Sui transaction submission requires a Sui wallet and native Sui CLI/SDK. Preview only."
    });

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    anyhow::bail!(
        "Sui transaction submission not yet supported by onchainos CLI.\n\
         Remove --confirm to see the Move call preview.\n\
         To execute, use the NAVI app at https://app.naviprotocol.io or the NAVI TypeScript SDK."
    );
}
