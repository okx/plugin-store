use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    asset: &str,        // collateral token address (or base asset address)
    amount_str: &str,   // human-readable amount (e.g. "0.5" for 0.5 WETH)
    from: Option<String>,
    dry_run: bool,
    confirm: bool,
) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;
    let asset_decimals = rpc::get_erc20_decimals(asset, cfg.rpc_url).await.unwrap_or(18);
    let amount = rpc::parse_human_amount(amount_str, asset_decimals)?;

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

    // Pre-flight: check on-chain balance so dust/rounding mismatches surface before any gas is spent
    let is_base = asset.eq_ignore_ascii_case(cfg.base_asset);
    let on_chain_balance: u128 = if is_base {
        rpc::get_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url).await.unwrap_or(0)
    } else {
        rpc::get_collateral_balance_of(cfg.comet_proxy, &wallet, asset, cfg.rpc_url).await.unwrap_or(0)
    };
    let balance_type = if is_base { "supply balance" } else { "collateral balance" };
    let asset_factor = 10f64.powi(asset_decimals as i32);
    if on_chain_balance == 0 {
        anyhow::bail!(
            "No {} of asset {} in this market. Supply the asset first before attempting to withdraw.",
            balance_type,
            asset
        );
    }
    if amount > on_chain_balance {
        anyhow::bail!(
            "Withdrawal amount {:.decimals$} exceeds your current {}: {:.decimals$} (raw: {}). \
             Use --amount {:.decimals$} or less.",
            amount as f64 / asset_factor,
            balance_type,
            on_chain_balance as f64 / asset_factor,
            on_chain_balance,
            on_chain_balance as f64 / asset_factor,
            decimals = asset_decimals as usize
        );
    }

    // Build withdraw(address,uint256) calldata
    // selector: 0xf3fef3a3
    let asset_padded = rpc::pad_address(asset);
    let amount_hex = rpc::pad_u128(amount);
    let withdraw_calldata = format!("0xf3fef3a3{}{}", asset_padded, amount_hex);

    let amount_human = format!("{:.decimals$}", amount as f64 / 10f64.powi(asset_decimals as i32), decimals = asset_decimals as usize);

    // Confirm gate: show preview and exit if --confirm not given (and not dry-run)
    if !dry_run && !confirm {
        let result = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "withdraw",
            "chain_id": chain_id,
            "market": market,
            "asset": asset,
            "amount": amount_human,
            "amount_raw": amount.to_string(),
            "current_balance": format!("{:.decimals$}", on_chain_balance as f64 / asset_factor, decimals = asset_decimals as usize),
            "current_balance_raw": on_chain_balance.to_string(),
            "balance_type": balance_type,
            "comet": cfg.comet_proxy,
            "pending_transactions": 1,
            "transactions": [
                {"step": 1, "action": "Comet.withdraw", "comet": cfg.comet_proxy, "asset": asset, "amount": amount_human.clone(), "amount_raw": amount.to_string(), "calldata": withdraw_calldata}
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
            "note": "Withdraw uses Comet.withdraw(asset, amount). No ERC-20 approve needed.",
            "steps": [
                {
                    "step": 1,
                    "action": "Comet.withdraw",
                    "comet": cfg.comet_proxy,
                    "asset": asset,
                    "amount": amount_human,
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
    let withdraw_tx = onchainos::extract_tx_hash_or_err(&withdraw_result)?;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "asset": asset,
            "amount": amount_human,
            "amount_raw": amount.to_string(),
            "wallet": wallet,
            "withdraw_tx_hash": withdraw_tx
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
