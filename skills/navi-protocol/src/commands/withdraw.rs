use anyhow::Result;
use clap::Args;

use crate::api::fetch_latest_package_id;
use crate::config::{DEFAULT_PROTOCOL_PACKAGE, STORAGE_ID, PRICE_ORACLE_ID, INCENTIVE_V2_ID, CLOCK_ID, find_pool};

#[derive(Args, Debug)]
pub struct WithdrawArgs {
    /// Asset to withdraw (e.g. SUI, USDC, USDT, WETH)
    #[arg(long)]
    pub asset: String,

    /// Amount to withdraw (in token units). Use "max" for full withdrawal.
    #[arg(long)]
    pub amount: String,

    /// Your Sui wallet address
    #[arg(long)]
    pub wallet: Option<String>,

    /// Execute the transaction (not supported — Sui write ops require native support)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: WithdrawArgs) -> Result<()> {
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
        "full balance".to_string()
    } else {
        format!("{} {}", args.amount, symbol)
    };

    let preview = serde_json::json!({
        "preview": true,
        "action": "withdraw",
        "protocol": "NAVI Protocol",
        "chain": "Sui Mainnet",
        "asset": symbol,
        "asset_id": asset_id,
        "amount": amount_description,
        "wallet": args.wallet.as_deref().unwrap_or("<your-sui-address>"),
        "move_call": {
            "package": package_id,
            "module": "lending_logic",
            "function": "withdraw",
            "type_args": [coin_type],
            "args": [
                {"name": "storage", "id": STORAGE_ID},
                {"name": "pool", "id": pool_id},
                {"name": "oracle", "id": PRICE_ORACLE_ID},
                {"name": "incentive_v2", "id": INCENTIVE_V2_ID},
                {"name": "clock", "id": CLOCK_ID},
                {"name": "amount", "description": "withdraw amount in base units or U64_MAX for full withdrawal"}
            ]
        },
        "warning": "Ensure withdrawal does not drop health factor below 1.0 if you have borrows.",
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
