// Show user's Beefy vault positions (mooToken balances)

use anyhow::Result;
use serde_json::{json, Value};

use crate::api;
use crate::config::chain_id_to_beefy_name;
use crate::onchainos;
use crate::rpc;

pub async fn execute(chain_id: u64, wallet: Option<&str>) -> Result<Value> {
    let chain_name = chain_id_to_beefy_name(chain_id)
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain ID: {}", chain_id))?;

    let wallet_addr = match wallet {
        Some(w) => w.to_string(),
        None => onchainos::resolve_wallet(chain_id)?,
    };

    let vaults = api::fetch_vaults(chain_id).await?;
    let active_vaults: Vec<_> = vaults.iter().filter(|v| v.is_active()).collect();

    let mut positions = Vec::new();

    // Check mooToken balance for each active vault
    // To avoid rate limits, batch calls but cap at 50 vaults
    let cap = active_vaults.len().min(50);
    for v in &active_vaults[..cap] {
        let earn_addr = match v.earn_contract_address.as_deref() {
            Some(a) => a,
            None => continue,
        };

        let balance = rpc::get_moo_balance(chain_id, earn_addr, &wallet_addr)
            .await
            .unwrap_or(0);

        if balance == 0 {
            continue;
        }

        // Get pricePerFullShare to compute underlying value
        let ppfs = rpc::get_price_per_full_share(chain_id, earn_addr)
            .await
            .unwrap_or(1_000_000_000_000_000_000);

        // Underlying value = balance * ppfs / 1e18
        let underlying = balance
            .checked_mul(ppfs)
            .map(|v| v / 1_000_000_000_000_000_000)
            .unwrap_or(0);

        // Determine decimals for display
        let decimals = if let Some(d) = v.token_decimals {
            d
        } else if let Some(ta) = v.token_address.as_deref() {
            rpc::get_decimals(chain_id, ta).await.unwrap_or(18)
        } else {
            18
        };

        let denom = 10u128.pow(decimals);
        let underlying_human = underlying as f64 / denom as f64;

        positions.push(json!({
            "vault_id": v.id,
            "vault_name": v.name,
            "earn_contract": earn_addr,
            "assets": v.assets,
            "token": v.token,
            "moo_balance_raw": balance.to_string(),
            "underlying_assets": format!("{:.6}", underlying_human),
            "decimals": decimals,
        }));
    }

    Ok(json!({
        "ok": true,
        "wallet": wallet_addr,
        "chain": chain_name,
        "chain_id": chain_id,
        "positions_found": positions.len(),
        "positions": positions,
    }))
}
