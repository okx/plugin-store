use clap::Args;

use crate::{api, config, onchainos};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address (optional; defaults to current onchainos Solana wallet)
    #[arg(long)]
    pub wallet: Option<String>,

    /// Market address (optional; defaults to main market)
    #[arg(long)]
    pub market: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let wallet = match args.wallet {
        Some(w) => w,
        None => onchainos::resolve_wallet_solana()?,
    };

    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --wallet or ensure onchainos is logged in.");
    }

    let market = args.market.as_deref().unwrap_or(config::MAIN_MARKET);

    let obligations = api::get_obligations(market, &wallet).await?;

    let result = if obligations.as_array().map(|a| a.is_empty()).unwrap_or(false) {
        serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "market": market,
                "has_positions": false,
                "message": "No active positions found for this wallet on Kamino Lend",
                "obligations": []
            }
        })
    } else {
        serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "market": market,
                "has_positions": true,
                "obligations": obligations
            }
        })
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
