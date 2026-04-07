use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    asset: &str,   // token contract address to supply
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

    // Build supply(address,uint256) calldata
    // selector: 0xf2b9fdb8
    let asset_padded = rpc::pad_address(asset);
    let amount_hex = rpc::pad_u128(amount);
    let supply_calldata = format!("0xf2b9fdb8{}{}", asset_padded, amount_hex);

    if dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "steps": [
                {
                    "step": 1,
                    "action": "ERC-20 approve",
                    "token": asset,
                    "spender": cfg.comet_proxy,
                    "amount_raw": amount.to_string()
                },
                {
                    "step": 2,
                    "action": "wait 3s"
                },
                {
                    "step": 3,
                    "action": "Comet.supply",
                    "comet": cfg.comet_proxy,
                    "asset": asset,
                    "amount_raw": amount.to_string(),
                    "calldata": supply_calldata
                }
            ]
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Step 1: ERC-20 approve
    let approve_result = onchainos::erc20_approve(
        chain_id,
        asset,
        cfg.comet_proxy,
        amount,
        Some(&wallet),
        false,
    )
    .await?;
    let approve_tx = onchainos::extract_tx_hash(&approve_result)?;

    // Step 2: 3-second delay to avoid nonce collision
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Step 3: Comet.supply
    let supply_result = onchainos::wallet_contract_call(
        chain_id,
        cfg.comet_proxy,
        &supply_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;
    let supply_tx = onchainos::extract_tx_hash(&supply_result)?;

    // Read updated supply balance
    let new_balance = rpc::get_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url)
        .await
        .unwrap_or(0);
    let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "asset": asset,
            "amount_raw": amount.to_string(),
            "wallet": wallet,
            "approve_tx_hash": approve_tx,
            "supply_tx_hash": supply_tx,
            "new_supply_balance": format!("{:.6}", new_balance as f64 / decimals_factor),
            "new_supply_balance_raw": new_balance.to_string()
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
