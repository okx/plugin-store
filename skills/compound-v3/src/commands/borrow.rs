use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    amount: u128,  // raw amount of base asset to borrow (minimal units)
    from: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;

    // Resolve wallet address — must not default to zero address
    let wallet = from
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or log in via onchainos.");
    }

    // Pre-flight checks
    let base_borrow_min = rpc::get_base_borrow_min(cfg.comet_proxy, cfg.rpc_url).await?;
    if amount < base_borrow_min {
        let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;
        anyhow::bail!(
            "Borrow amount {:.6} {} is below minimum borrow {:.6} {}. Increase amount.",
            amount as f64 / decimals_factor,
            cfg.base_asset_symbol,
            base_borrow_min as f64 / decimals_factor,
            cfg.base_asset_symbol
        );
    }

    let is_collateralized = rpc::is_borrow_collateralized(cfg.comet_proxy, &wallet, cfg.rpc_url).await?;
    if !is_collateralized {
        anyhow::bail!(
            "Account is not sufficiently collateralized. Supply collateral first before borrowing."
        );
    }

    // Build withdraw(address,uint256) calldata (borrow = withdraw base asset)
    // selector: 0xf3fef3a3
    let base_padded = rpc::pad_address(cfg.base_asset);
    let amount_hex = rpc::pad_u128(amount);
    let borrow_calldata = format!("0xf3fef3a3{}{}", base_padded, amount_hex);

    if dry_run {
        let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "note": "Borrow uses Comet.withdraw(base_asset, amount). No ERC-20 approve needed.",
            "steps": [
                {
                    "step": 1,
                    "action": "Comet.withdraw (borrow base asset)",
                    "comet": cfg.comet_proxy,
                    "base_asset": cfg.base_asset,
                    "amount": format!("{:.6}", amount as f64 / decimals_factor),
                    "amount_raw": amount.to_string(),
                    "calldata": borrow_calldata
                }
            ]
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Execute Comet.withdraw (which initiates borrow when supply < amount)
    let borrow_result = onchainos::wallet_contract_call(
        chain_id,
        cfg.comet_proxy,
        &borrow_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;
    let borrow_tx = onchainos::extract_tx_hash(&borrow_result)?;

    // Read updated borrow balance
    let new_borrow = rpc::get_borrow_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url)
        .await
        .unwrap_or(0);
    let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "base_asset": cfg.base_asset_symbol,
            "amount_raw": amount.to_string(),
            "amount": format!("{:.6}", amount as f64 / decimals_factor),
            "wallet": wallet,
            "borrow_tx_hash": borrow_tx,
            "new_borrow_balance": format!("{:.6}", new_borrow as f64 / decimals_factor),
            "new_borrow_balance_raw": new_borrow.to_string()
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
