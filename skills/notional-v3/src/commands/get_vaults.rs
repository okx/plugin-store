use crate::api;
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct GetVaultsArgs {
    /// Filter by asset symbol (e.g. USDC, WETH)
    #[arg(long)]
    pub asset: Option<String>,
}

pub async fn execute(args: &GetVaultsArgs) -> anyhow::Result<()> {
    let vaults = api::get_vaults().await?;

    let mut results = vec![];
    for v in &vaults {
        let asset_sym = v.asset.symbol.as_deref().unwrap_or("?");
        let yt_sym = v.yield_token.symbol.as_deref().unwrap_or("?");

        // Filter by asset if provided
        if let Some(filter) = &args.asset {
            if !asset_sym.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }

        let name = api::vault_name(&v.id);
        results.push(json!({
            "vault": v.id,
            "name": name,
            "asset_symbol": asset_sym,
            "asset_address": v.asset.id,
            "yield_token_symbol": yt_sym,
            "yield_token_address": v.yield_token.id,
            "is_whitelisted": v.is_whitelisted,
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": 1,
            "protocol": "Notional Exponent",
            "router": crate::config::MORPHO_LENDING_ROUTER,
            "vaults": results,
            "count": results.len()
        }))?
    );
    Ok(())
}
