use clap::Args;
use serde_json::Value;
use crate::api;

#[derive(Args)]
pub struct RatesArgs {
    /// Filter by token symbol (e.g. wstETH, rETH)
    #[arg(long)]
    pub token: Option<String>,

    /// Maximum number of vaults to show
    #[arg(long, default_value = "20")]
    pub limit: u64,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value = "1")]
    pub chain: u64,
}

pub async fn run(args: RatesArgs) -> anyhow::Result<Value> {
    let vaults = api::fetch_vaults(100).await?;

    let filtered: Vec<&api::Vault> = if let Some(ref tok) = args.token {
        vaults.iter().filter(|v| v.token.symbol.to_lowercase() == tok.to_lowercase()).collect()
    } else {
        vaults.iter().collect()
    };

    // Sort by APR descending
    let mut sorted: Vec<&api::Vault> = filtered;
    sorted.sort_by(|a, b| {
        b.vault_rewards_apr.unwrap_or(0.0)
            .partial_cmp(&a.vault_rewards_apr.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let limited: Vec<&api::Vault> = sorted.into_iter().take(args.limit as usize).collect();

    let rates: Vec<Value> = limited.iter().map(|v| {
        let name = v.meta.as_ref()
            .and_then(|m| m.name.as_deref())
            .unwrap_or(&v.token.symbol);

        let reward_tokens: Vec<String> = v.vault_rewards.as_ref()
            .map(|rewards| {
                rewards.iter()
                    .filter_map(|r| {
                        r.reward_token.as_ref()
                            .and_then(|t| t.get("symbol"))
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        serde_json::json!({
            "vault_address": v.address,
            "vault_name": name,
            "token_symbol": v.token.symbol,
            "apr": api::format_apr(v.vault_rewards_apr),
            "apr_raw": v.vault_rewards_apr.unwrap_or(0.0),
            "tvl": api::format_tvl(v.tvl),
            "reward_tokens": reward_tokens,
        })
    }).collect();

    Ok(serde_json::json!({
        "ok": true,
        "chain": args.chain,
        "note": "APR includes restaking rewards. Epoch rewards vary based on network slashing conditions.",
        "rates": rates
    }))
}
