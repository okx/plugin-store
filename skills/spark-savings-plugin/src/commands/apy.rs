use clap::Args;
use serde_json::json;

use crate::config::SUPPORTED_CHAINS;
use crate::rpc::{ssr_to_apy, susds_chi, susds_ssr, vault_total_assets, fmt_token_amount};
use crate::config::STABLE_DECIMALS;

#[derive(Args)]
pub struct ApyArgs {}

pub async fn run(_args: ApyArgs) -> anyhow::Result<()> {
    // SSR is governed at Ethereum mainnet; L2 sUSDS prices follow via oracle,
    // so the rate is effectively the same. Read canonical values from mainnet.
    let eth_chain = SUPPORTED_CHAINS.iter().find(|c| c.id == 1).unwrap();

    let ssr = match susds_ssr(eth_chain.susds, eth_chain.rpc).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("Failed to read sUSDS ssr() on Ethereum: {:#}", e),
                "RPC_ERROR",
                "Public RPC may be limited. Retry in a few seconds.",
            ));
            return Ok(());
        }
    };
    // EVM-012: chi + tvl reads are display-only (rendering doesn't depend on
    // them). Keep the soft 0 fallback but expose query errors so callers can
    // mark the rendered values as best-effort when RPC blips.
    let (chi, chi_query_error) = match susds_chi(eth_chain.susds, eth_chain.rpc).await {
        Ok(v) => (v, None::<String>),
        Err(e) => (0u128, Some(format!("{:#}", e))),
    };
    let apy_decimal = ssr_to_apy(ssr);

    // TVL = USDS held by sUSDS vault on Ethereum (canonical figure).
    let (tvl_assets, tvl_query_error) = match vault_total_assets(eth_chain.susds, eth_chain.rpc).await {
        Ok(v) => (v, None::<String>),
        Err(e) => (0u128, Some(format!("{:#}", e))),
    };

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "current_apy_pct": format!("{:.4}", apy_decimal * 100.0),
        "current_apy_decimal": apy_decimal,
        "ssr_ray": ssr.to_string(),
        "chi_ray": chi.to_string(),
        "chi_query_error": chi_query_error,
        "ray_scale": "1e27",
        "rate_canonical_chain": eth_chain.key,
        "tvl_usds": fmt_token_amount(tvl_assets, STABLE_DECIMALS),
        "tvl_usds_raw": tvl_assets.to_string(),
        "tvl_query_error": tvl_query_error,
        "note": "SSR is governance-set on Ethereum and applied across all chains via oracle. APY is a derived per-second rate compounded over 365 days.",
    }))?);
    Ok(())
}
