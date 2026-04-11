use clap::Args;
use crate::api::MeteoraClient;
use crate::onchainos;

#[derive(Args, Debug)]
pub struct GetUserPositionsArgs {
    /// Wallet address (Solana pubkey). If omitted, uses the currently logged-in wallet.
    #[arg(long)]
    pub wallet: Option<String>,

    /// Filter by pool address (optional)
    #[arg(long)]
    pub pool: Option<String>,
}

pub async fn execute(args: &GetUserPositionsArgs) -> anyhow::Result<()> {
    // Resolve wallet address
    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!("Cannot resolve wallet address. Pass --wallet <address> or log in via onchainos.\nError: {e}")
        })?
    };

    if wallet.is_empty() {
        anyhow::bail!("Wallet address is empty. Pass --wallet <address> or log in via onchainos.");
    }

    let client = MeteoraClient::new();
    let positions = client.get_positions(&wallet).await?;

    // Filter by pool if specified
    let positions: Vec<_> = if let Some(pool_addr) = &args.pool {
        positions
            .into_iter()
            .filter(|p| p.pair_address == *pool_addr)
            .collect()
    } else {
        positions
    };

    if positions.is_empty() {
        let output = serde_json::json!({
            "ok": true,
            "wallet": wallet,
            "positions": [],
            "message": "No positions found for this wallet",
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let total_value_usd: f64 = positions.iter().map(|p| p.total_value_usd).sum();
    let total_fee_usd: f64 = positions.iter().map(|p| p.total_fee_usd).sum();

    let positions_out: Vec<serde_json::Value> = positions
        .iter()
        .map(|p| {
            serde_json::json!({
                "position_address": p.address,
                "pool_address": p.pair_address,
                "owner": p.owner,
                "token_x_amount": p.total_x_amount,
                "token_y_amount": p.total_y_amount,
                "fee_x_unclaimed": p.fee_x,
                "fee_y_unclaimed": p.fee_y,
                "total_fee_usd": p.total_fee_usd,
                "total_value_usd": p.total_value_usd,
                "bin_range": {
                    "lower_bin_id": p.lower_bin_id,
                    "upper_bin_id": p.upper_bin_id,
                },
                "bin_data_count": p.data.len(),
            })
        })
        .collect();

    let output = serde_json::json!({
        "ok": true,
        "wallet": wallet,
        "positions_count": positions_out.len(),
        "summary": {
            "total_value_usd": total_value_usd,
            "total_unclaimed_fees_usd": total_fee_usd,
        },
        "positions": positions_out,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
