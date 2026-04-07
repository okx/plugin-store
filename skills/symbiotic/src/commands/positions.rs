use clap::Args;
use serde_json::Value;
use crate::{api, config, onchainos, rpc};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (defaults to logged-in wallet)
    #[arg(long)]
    pub address: Option<String>,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value = "1")]
    pub chain: u64,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<Value> {
    // Resolve wallet address
    let wallet = match args.address {
        Some(ref addr) => addr.clone(),
        None => {
            let w = onchainos::resolve_wallet(args.chain)?;
            if w.is_empty() {
                anyhow::bail!("Cannot resolve wallet address. Pass --address or log in via onchainos.");
            }
            w
        }
    };

    // Fetch all vaults from API
    let vaults = api::fetch_vaults(100).await?;
    let rpc = config::ETH_RPC;

    let mut positions: Vec<Value> = vec![];

    for vault in &vaults {
        // Query active balance
        let balance = match rpc::active_balance_of(&vault.address, &wallet, rpc).await {
            Ok(b) => b,
            Err(_) => continue, // Skip vaults that fail (e.g. legacy/different ABI)
        };

        if balance == 0 {
            continue;
        }

        // Format balance with token decimals
        let decimals = vault.token.decimals as u32;
        let divisor = 10u128.pow(decimals);
        let balance_fmt = format!("{}.{:0>width$}",
            balance / divisor,
            balance % divisor,
            width = decimals as usize
        );

        // Check for pending withdrawals (current epoch - 1)
        let current_epoch = rpc::current_epoch(&vault.address, rpc).await.unwrap_or(0);
        let epoch_dur = rpc::epoch_duration(&vault.address, rpc).await.unwrap_or(0);

        let pending_withdrawal = if current_epoch > 0 {
            rpc::withdrawals_of(&vault.address, current_epoch, &wallet, rpc).await.unwrap_or(0)
        } else {
            0
        };

        let pending_fmt = if pending_withdrawal > 0 {
            format!("{}.{:0>width$}",
                pending_withdrawal / divisor,
                pending_withdrawal % divisor,
                width = decimals as usize
            )
        } else {
            "0".to_string()
        };

        let vault_name = vault.meta.as_ref()
            .and_then(|m| m.name.as_deref())
            .unwrap_or(&vault.token.symbol);

        // Calculate USD value
        let usd_value = vault.token.usd_price.map(|price| {
            let bal_f = balance as f64 / 10f64.powi(decimals as i32);
            price * bal_f
        });

        let epoch_days = epoch_dur / 86400; // seconds to days

        positions.push(serde_json::json!({
            "vault_address": vault.address,
            "vault_name": vault_name,
            "token_symbol": vault.token.symbol,
            "token_address": vault.token.address,
            "active_balance": balance_fmt,
            "active_balance_raw": balance.to_string(),
            "usd_value": usd_value.map(|v| format!("${:.2}", v)),
            "pending_withdrawal": pending_fmt,
            "current_epoch": current_epoch.to_string(),
            "epoch_duration_days": epoch_days.to_string(),
            "apr": api::format_apr(vault.vault_rewards_apr),
        }));
    }

    Ok(serde_json::json!({
        "ok": true,
        "wallet": wallet,
        "chain": args.chain,
        "total_positions": positions.len(),
        "positions": positions
    }))
}
