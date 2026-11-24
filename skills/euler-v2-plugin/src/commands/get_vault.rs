/// `euler-v2-plugin get-vault` — full details for a single EVK vault.
///
/// Combines:
///   - Static metadata from `/api/vaults` (name, asset, IRM, oracle)
///   - Live on-chain reads (`vault.totalAssets()`, `vault.totalSupply()` via direct RPC)
///
/// Read-only. No wallet required.

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain};

#[derive(Args)]
pub struct GetVaultArgs {
    /// Vault address (0x-prefixed hex)
    #[arg(long)]
    pub address: String,

    /// Chain ID: 1 / 8453 / 42161
    #[arg(long, default_value_t = 1)]
    pub chain: u64,
}

pub async fn run(args: GetVaultArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("get-vault"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: GetVaultArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!(
            "Chain {} not supported in v0.1. Use 1 (Ethereum), 8453 (Base), or 42161 (Arbitrum).",
            args.chain
        );
    }
    let target_addr = args.address.to_lowercase();
    if !target_addr.starts_with("0x") || target_addr.len() != 42 {
        anyhow::bail!(
            "Invalid vault address '{}'. Expect 0x-prefixed 40-hex-char address.",
            args.address
        );
    }

    // 1. Look up the vault in the API response
    let vaults = crate::api::get_vaults_raw(args.chain).await?;
    let evk = vaults["evkVaults"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Euler API returned no evkVaults field for chain {}", args.chain))?;
    let entry = evk.iter()
        .find(|v| v["address"].as_str().map(|s| s.to_lowercase()) == Some(target_addr.clone()))
        .ok_or_else(|| anyhow::anyhow!(
            "Vault {} not found in Euler API for chain {}. \
             Run `list-vaults --chain {}` to see available vaults.",
            args.address, args.chain, args.chain
        ))?;

    // 2. Live on-chain reads (parallel)
    use crate::rpc::{build_address_call, eth_call, parse_uint256_to_u128, SELECTOR_BALANCE_OF};
    // totalAssets selector: keccak256("totalAssets()")[:4] = 0x01e1d114
    // totalSupply selector: keccak256("totalSupply()")[:4]  = 0x18160ddd
    let total_assets_calldata = "0x01e1d114".to_string();
    let total_supply_calldata = "0x18160ddd".to_string();
    let cash_calldata = "0x47e7ef24".to_string();  // placeholder; cash() may differ — kept as best-effort
    let _ = SELECTOR_BALANCE_OF;
    let _ = build_address_call;

    let (ta_res, ts_res) = tokio::join!(
        eth_call(args.chain, &target_addr, &total_assets_calldata),
        eth_call(args.chain, &target_addr, &total_supply_calldata),
    );
    let total_assets = ta_res.ok().map(|h| parse_uint256_to_u128(&h));
    let total_supply = ts_res.ok().map(|h| parse_uint256_to_u128(&h));
    let _ = cash_calldata;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "chain": chain_name(args.chain),
                "chain_id": args.chain,
                "address": entry["address"],
                "name": entry["name"],
                "verified": entry["verified"],
                "asset": entry.get("asset"),
                "irm": entry.get("irm"),
                "supply_raw_api": entry["supply"]["__bi"].as_str().unwrap_or("0"),
                "borrow_raw_api": entry["borrow"]["__bi"].as_str().unwrap_or("0"),
                "live": {
                    "total_assets_raw":  total_assets.map(|v| v.to_string()),
                    "total_supply_raw":  total_supply.map(|v| v.to_string()),
                },
                "tip": "Use `supply --vault <address> --amount <N>` to deposit; \
                        `positions --chain <id>` to see your stake."
            }
        }))?
    );
    Ok(())
}
