use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    amount_str: &str,  // human-readable amount (e.g. "0.1" for 0.1 USDC)
    from: Option<String>,
    dry_run: bool,
    confirm: bool,
) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;
    let amount = rpc::parse_human_amount(amount_str, cfg.base_asset_decimals)?;

    // Resolve wallet address — must not default to zero address
    let wallet = from
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or log in via onchainos.");
    }

    // Pre-flight: simulate borrow to catch NotCollateralized() and other reverts
    // before spending gas. isBorrowCollateralized() is a false positive for new
    // accounts (principal=0 → returns true even with zero collateral), so we
    // simulate the actual calldata instead.
    rpc::simulate_borrow(
        cfg.comet_proxy,
        cfg.base_asset,
        amount,
        &wallet,
        cfg.rpc_url,
        cfg.base_asset_decimals,
        cfg.base_asset_symbol,
    ).await?;

    // Fetch baseBorrowMin for preview — gives agents/users the minimum position size upfront
    let min_borrow_raw = rpc::get_base_borrow_min(cfg.comet_proxy, cfg.rpc_url).await.unwrap_or(0);

    // Build withdraw(address,uint256) calldata (borrow = withdraw base asset)
    // selector: 0xf3fef3a3
    let base_padded = rpc::pad_address(cfg.base_asset);
    let amount_hex = rpc::pad_u128(amount);
    let borrow_calldata = format!("0xf3fef3a3{}{}", base_padded, amount_hex);

    let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;

    // Confirm gate: show preview and exit if --confirm not given (and not dry-run)
    if !dry_run && !confirm {
        let result = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "borrow",
            "chain_id": chain_id,
            "market": market,
            "base_asset": cfg.base_asset_symbol,
            "amount": amount_str,
            "amount_raw": amount.to_string(),
            "amount_human": format!("{:.6}", amount as f64 / decimals_factor),
            "min_borrow_amount": format!("{:.6}", min_borrow_raw as f64 / decimals_factor),
            "min_borrow_amount_raw": min_borrow_raw.to_string(),
            "comet": cfg.comet_proxy,
            "pending_transactions": 1,
            "transactions": [
                {"step": 1, "action": "Comet.withdraw (borrow base asset)", "comet": cfg.comet_proxy, "base_asset": cfg.base_asset, "amount_raw": amount.to_string(), "calldata": borrow_calldata}
            ],
            "note": "Re-run with --confirm to execute this transaction on-chain."
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "note": "Borrow uses Comet.withdraw(base_asset, amount). No ERC-20 approve needed.",
            "min_borrow_amount": format!("{:.6}", min_borrow_raw as f64 / decimals_factor),
            "min_borrow_amount_raw": min_borrow_raw.to_string(),
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
    let borrow_tx = onchainos::extract_tx_hash_or_err(&borrow_result)?;

    // Wait for borrow tx to confirm before reading post-tx balance
    onchainos::wait_for_tx(&borrow_tx, cfg.rpc_url).await;

    let new_borrow = rpc::get_borrow_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url)
        .await
        .unwrap_or(0);

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
