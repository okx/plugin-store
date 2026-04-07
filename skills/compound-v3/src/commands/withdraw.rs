use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    asset: &str,   // collateral token address (or base asset address)
    amount: u128,  // raw amount in token's minimal units
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

    // Safety check: must clear all debt before withdrawing collateral
    let borrow_balance = rpc::get_borrow_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url).await?;
    if borrow_balance > 0 {
        let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;
        anyhow::bail!(
            "Account has outstanding debt of {:.6} {} on this market. \
             Repay all debt before withdrawing collateral to avoid liquidation.",
            borrow_balance as f64 / decimals_factor,
            cfg.base_asset_symbol
        );
    }

    // Build withdraw(address,uint256) calldata
    // selector: 0xf3fef3a3
    let asset_padded = rpc::pad_address(asset);
    let amount_hex = rpc::pad_u128(amount);
    let withdraw_calldata = format!("0xf3fef3a3{}{}", asset_padded, amount_hex);

    if dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "note": "Withdraw uses Comet.withdraw(asset, amount). No ERC-20 approve needed.",
            "steps": [
                {
                    "step": 1,
                    "action": "Comet.withdraw",
                    "comet": cfg.comet_proxy,
                    "asset": asset,
                    "amount_raw": amount.to_string(),
                    "calldata": withdraw_calldata
                }
            ]
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Execute Comet.withdraw
    let withdraw_result = onchainos::wallet_contract_call(
        chain_id,
        cfg.comet_proxy,
        &withdraw_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;
    let withdraw_tx = onchainos::extract_tx_hash(&withdraw_result)?;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "asset": asset,
            "amount_raw": amount.to_string(),
            "wallet": wallet,
            "withdraw_tx_hash": withdraw_tx
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
