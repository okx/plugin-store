use crate::{api, config, onchainos};
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct GetPositionsArgs {
    /// Wallet address (optional, defaults to onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(args: &GetPositionsArgs, dry_run: bool) -> anyhow::Result<()> {
    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        if dry_run {
            "0x0000000000000000000000000000000000000000".to_string()
        } else {
            onchainos::resolve_wallet(config::ETHEREUM_CHAIN_ID)?
        }
    };

    let balances = api::get_account_balances(&wallet).await?;

    let mut positions = vec![];
    for b in &balances {
        let balance_str = b.current.current_balance.as_deref().unwrap_or("0");
        let balance_val: i128 = balance_str.parse().unwrap_or(0);
        if balance_val == 0 {
            continue;
        }

        let vault_id = b
            .token
            .vault_address
            .as_ref()
            .map(|v| v.id.as_str())
            .unwrap_or(&b.token.id);

        let vault_name = api::vault_name(vault_id);
        let router_name = b
            .lending_router
            .as_ref()
            .and_then(|r| r.name.as_deref())
            .unwrap_or("Morpho");

        // Get health factor if it's a VaultShare position
        let mut health_factor_str = None;
        let mut collateral_str = None;

        if b.token.token_type == "VaultShare" {
            if let Ok(hf) = api::get_health_factor(&wallet, vault_id).await {
                if hf > 0 {
                    let hf_f = hf as f64 / 1e18;
                    health_factor_str = Some(format!("{:.4}", hf_f));
                }
            }
            if let Ok(col) = api::get_collateral_balance(&wallet, vault_id).await {
                if col > 0 {
                    collateral_str = Some(col.to_string());
                }
            }
        }

        let mut pos = json!({
            "token_id": b.token.id,
            "token_type": b.token.token_type,
            "token_symbol": b.token.symbol,
            "vault": vault_id,
            "vault_name": vault_name,
            "lending_router": router_name,
            "current_balance": balance_str,
        });

        if let Some(hf) = &health_factor_str {
            pos["health_factor"] = json!(hf);
        }
        if let Some(col) = &collateral_str {
            pos["collateral_balance"] = json!(col);
        }
        if let Some(pnl) = &b.current.pnl {
            pos["pnl"] = json!(pnl);
        }

        positions.push(pos);
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "wallet": wallet,
            "chain": 1,
            "protocol": "Notional Exponent",
            "positions": positions,
            "count": positions.len()
        }))?
    );
    Ok(())
}
