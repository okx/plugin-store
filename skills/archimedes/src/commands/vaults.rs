use crate::config::{format_amount, VAULTS};
use crate::rpc;
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct VaultsArgs {
    /// Custom RPC URL (default: mevblocker)
    #[arg(long)]
    pub rpc: Option<String>,
}

pub async fn run(args: VaultsArgs) -> anyhow::Result<()> {
    let rpc_url_owned: Option<String> = args.rpc.clone();
    let rpc_urls: Vec<&str> = if let Some(ref url) = rpc_url_owned {
        vec![url.as_str()]
    } else {
        crate::config::RPC_URLS.to_vec()
    };

    let mut vault_list = Vec::new();

    for vault in VAULTS.iter() {
        let tvl = rpc::total_assets(&rpc_urls, vault.address)
            .await
            .unwrap_or(0);
        let tvl_formatted = format_amount(tvl, vault.underlying_decimals);

        vault_list.push(json!({
            "name": vault.name,
            "vault_address": vault.address,
            "underlying_symbol": vault.underlying_symbol,
            "underlying_address": vault.underlying_address,
            "tvl_raw": tvl.to_string(),
            "tvl_formatted": format!("{} {}", tvl_formatted, vault.underlying_symbol),
        }));
    }

    let output = json!({
        "protocol": "Archimedes Finance V2",
        "chain": "Ethereum",
        "chain_id": 1,
        "vaults": vault_list,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
