use crate::config::{format_amount, VAULTS};
use crate::onchainos;
use crate::rpc;
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct PositionsArgs {
    /// Wallet address to query (default: logged-in wallet)
    #[arg(long)]
    pub wallet: Option<String>,

    /// Custom RPC URL (default: mevblocker)
    #[arg(long)]
    pub rpc: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc_url_owned: Option<String> = args.rpc.clone();
    let rpc_urls: Vec<&str> = if let Some(ref url) = rpc_url_owned {
        vec![url.as_str()]
    } else {
        crate::config::RPC_URLS.to_vec()
    };

    let wallet = if let Some(w) = args.wallet {
        w
    } else {
        let w = onchainos::resolve_wallet(crate::config::CHAIN_ID)?;
        if w.is_empty() {
            anyhow::bail!(
                "Could not resolve wallet address. Pass --wallet or ensure onchainos is logged in."
            );
        }
        w
    };

    let mut positions = Vec::new();

    for vault in VAULTS.iter() {
        let shares = rpc::balance_of(&rpc_urls, vault.address, &wallet)
            .await
            .unwrap_or(0);

        let underlying_value = if shares > 0 {
            rpc::convert_to_assets(&rpc_urls, vault.address, shares)
                .await
                .unwrap_or(0)
        } else {
            0
        };

        let tvl = rpc::total_assets(&rpc_urls, vault.address)
            .await
            .unwrap_or(0);

        positions.push(json!({
            "name": vault.name,
            "vault_address": vault.address,
            "underlying_symbol": vault.underlying_symbol,
            "shares_raw": shares.to_string(),
            "shares_formatted": format_amount(shares, 18),
            "underlying_value_raw": underlying_value.to_string(),
            "underlying_value_formatted": format!("{} {}", format_amount(underlying_value, vault.underlying_decimals), vault.underlying_symbol),
            "has_position": shares > 0,
            "vault_tvl_raw": tvl.to_string(),
            "vault_tvl_formatted": format!("{} {}", format_amount(tvl, vault.underlying_decimals), vault.underlying_symbol),
        }));
    }

    let output = json!({
        "protocol": "Archimedes Finance V2",
        "chain": "Ethereum",
        "chain_id": 1,
        "wallet": wallet,
        "positions": positions,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
