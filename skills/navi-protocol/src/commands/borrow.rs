use anyhow::Result;
use clap::Args;

use crate::api::{fetch_latest_package_id, fetch_pools, parse_pool};
use crate::config::{DEFAULT_PROTOCOL_PACKAGE, STORAGE_ID, PRICE_ORACLE_ID, INCENTIVE_V2_ID, CLOCK_ID, find_pool};

#[derive(Args, Debug)]
pub struct BorrowArgs {
    /// Asset to borrow (e.g. SUI, USDC, USDT, WETH)
    #[arg(long)]
    pub asset: String,

    /// Amount to borrow (in token units, e.g. 100)
    #[arg(long)]
    pub amount: String,

    /// Your Sui wallet address
    #[arg(long)]
    pub wallet: Option<String>,

    /// Execute the transaction (not supported — Sui write ops require native support)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: BorrowArgs) -> Result<()> {
    // Validate amount
    let amount_f: f64 = args.amount.parse::<f64>()
        .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'. Must be a number.", args.amount))?;
    if amount_f <= 0.0 {
        anyhow::bail!("Amount must be positive.");
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

    // Fetch current borrow rate for this asset
    let borrow_apy = fetch_borrow_apy(symbol).await;

    let preview = serde_json::json!({
        "preview": true,
        "action": "borrow",
        "protocol": "NAVI Protocol",
        "chain": "Sui Mainnet",
        "asset": symbol,
        "asset_id": asset_id,
        "amount": args.amount,
        "current_borrow_apy_pct": format!("{:.4}", borrow_apy),
        "wallet": args.wallet.as_deref().unwrap_or("<your-sui-address>"),
        "move_call": {
            "package": package_id,
            "module": "lending_logic",
            "function": "borrow",
            "type_args": [coin_type],
            "args": [
                {"name": "storage", "id": STORAGE_ID},
                {"name": "pool", "id": pool_id},
                {"name": "oracle", "id": PRICE_ORACLE_ID},
                {"name": "incentive_v2", "id": INCENTIVE_V2_ID},
                {"name": "clock", "id": CLOCK_ID},
                {"name": "amount", "value": args.amount, "description": "borrow amount in token base units"}
            ]
        },
        "warning": "Borrowing increases debt. Monitor your health factor to avoid liquidation.",
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

async fn fetch_borrow_apy(symbol: &str) -> f64 {
    match fetch_pools().await {
        Ok(pools) => {
            let sym_up = symbol.to_uppercase();
            for raw in &pools {
                if let Some(pool) = parse_pool(raw) {
                    if pool.symbol.to_uppercase() == sym_up {
                        return pool.borrow_apy_pct();
                    }
                }
            }
            0.0
        }
        Err(_) => 0.0,
    }
}
