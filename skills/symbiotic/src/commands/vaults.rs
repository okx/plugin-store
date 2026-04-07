use clap::Args;
use serde_json::Value;
use crate::api;

#[derive(Args)]
pub struct VaultsArgs {
    /// Maximum number of vaults to return
    #[arg(long, default_value = "20")]
    pub limit: u64,

    /// Filter by token symbol (e.g. wstETH, rETH, cbETH)
    #[arg(long)]
    pub token: Option<String>,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value = "1")]
    pub chain: u64,
}

pub async fn run(args: VaultsArgs) -> anyhow::Result<Value> {
    let vaults = api::fetch_vaults(100).await?;

    let filtered: Vec<&api::Vault> = if let Some(ref tok) = args.token {
        vaults.iter().filter(|v| v.token.symbol.to_lowercase() == tok.to_lowercase()).collect()
    } else {
        vaults.iter().collect()
    };

    let limited: Vec<&api::Vault> = filtered.into_iter().take(args.limit as usize).collect();

    let vault_list: Vec<Value> = limited.iter().map(|v| {
        let name = v.meta.as_ref()
            .and_then(|m| m.name.as_deref())
            .unwrap_or(&v.token.symbol);

        serde_json::json!({
            "address": v.address,
            "name": name,
            "token_symbol": v.token.symbol,
            "token_address": v.token.address,
            "token_decimals": v.token.decimals,
            "token_usd_price": v.token.usd_price,
            "tvl": api::format_tvl(v.tvl),
            "apr": api::format_apr(v.vault_rewards_apr),
            "restricted": v.restricted.unwrap_or(false),
            "slashable": v.slashable.unwrap_or(false),
            "legacy": v.legacy.unwrap_or(false),
        })
    }).collect();

    Ok(serde_json::json!({
        "ok": true,
        "chain": args.chain,
        "total": vault_list.len(),
        "vaults": vault_list
    }))
}
