use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    amount_str: Option<&str>, // None = repay all; human-readable (e.g. "5.0")
    from: Option<String>,
    dry_run: bool,
    confirm: bool,
) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;

    // Resolve wallet address — must not default to zero address
    let wallet = from
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or log in via onchainos.");
    }

    let borrow_balance = rpc::get_borrow_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url).await?;
    if borrow_balance == 0 {
        let result = serde_json::json!({
            "ok": true,
            "data": {
                "message": "No outstanding borrow balance to repay.",
                "borrow_balance": "0"
            }
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let wallet_balance = rpc::get_erc20_balance(cfg.base_asset, &wallet, cfg.rpc_url).await?;
    let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;

    // Determine repay amount:
    // - If specified: use that amount (capped by borrow balance)
    // - If "repay all": use min(borrow_balance, wallet_balance) to avoid overflow revert
    let amount: Option<u128> = match amount_str {
        Some(s) => Some(rpc::parse_human_amount(s, cfg.base_asset_decimals)?),
        None => None,
    };
    let repay_amount = match amount {
        Some(a) => a.min(borrow_balance),
        None => {
            if wallet_balance < borrow_balance {
                anyhow::bail!(
                    "Wallet {} balance {:.6} {} is less than borrow balance {:.6} {}. \
                     Acquire {:.6} more {} to repay fully.",
                    wallet,
                    wallet_balance as f64 / decimals_factor,
                    cfg.base_asset_symbol,
                    borrow_balance as f64 / decimals_factor,
                    cfg.base_asset_symbol,
                    (borrow_balance - wallet_balance) as f64 / decimals_factor,
                    cfg.base_asset_symbol
                );
            }
            borrow_balance.min(wallet_balance)
        }
    };

    // For explicit --amount: check wallet has enough before spending gas on approve
    if amount.is_some() && wallet_balance < repay_amount {
        anyhow::bail!(
            "Insufficient wallet balance to repay: wallet has {:.6} {} but repay amount is {:.6} {}. \
             Acquire {:.6} more {}, or omit --amount to repay as much as your wallet holds.",
            wallet_balance as f64 / decimals_factor,
            cfg.base_asset_symbol,
            repay_amount as f64 / decimals_factor,
            cfg.base_asset_symbol,
            (repay_amount - wallet_balance) as f64 / decimals_factor,
            cfg.base_asset_symbol
        );
    }

    // Repay uses Comet.supply(base_asset, repay_amount) — same method as supply
    // selector: 0xf2b9fdb8
    let base_padded = rpc::pad_address(cfg.base_asset);
    let amount_hex = rpc::pad_u128(repay_amount);
    let repay_calldata = format!("0xf2b9fdb8{}{}", base_padded, amount_hex);

    // Confirm gate: show preview and exit if --confirm not given (and not dry-run)
    if !dry_run && !confirm {
        let result = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "repay",
            "chain_id": chain_id,
            "market": market,
            "base_asset": cfg.base_asset_symbol,
            "repay_amount": format!("{:.6}", repay_amount as f64 / decimals_factor),
            "repay_amount_raw": repay_amount.to_string(),
            "borrow_balance": format!("{:.6}", borrow_balance as f64 / decimals_factor),
            "wallet_balance": format!("{:.6}", wallet_balance as f64 / decimals_factor),
            "comet": cfg.comet_proxy,
            "pending_transactions": 2,
            "transactions": [
                {"step": 1, "action": "ERC-20 approve", "token": cfg.base_asset, "spender": cfg.comet_proxy, "amount_raw": repay_amount.to_string()},
                {"step": 2, "action": "Comet.supply (repay)", "comet": cfg.comet_proxy, "base_asset": cfg.base_asset, "amount_raw": repay_amount.to_string(), "calldata": repay_calldata}
            ],
            "note": "Re-run with --confirm to execute these transactions on-chain."
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "note": "Repay uses Comet.supply(base_asset, amount). supply with base asset = repay debt.",
            "borrow_balance": format!("{:.6}", borrow_balance as f64 / decimals_factor),
            "wallet_balance": format!("{:.6}", wallet_balance as f64 / decimals_factor),
            "steps": [
                {
                    "step": 1,
                    "action": "ERC-20 approve",
                    "token": cfg.base_asset,
                    "spender": cfg.comet_proxy,
                    "amount_raw": repay_amount.to_string()
                },
                {
                    "step": 2,
                    "action": "wait 3s"
                },
                {
                    "step": 3,
                    "action": "Comet.supply (repay)",
                    "comet": cfg.comet_proxy,
                    "base_asset": cfg.base_asset,
                    "amount": format!("{:.6}", repay_amount as f64 / decimals_factor),
                    "amount_raw": repay_amount.to_string(),
                    "calldata": repay_calldata
                }
            ]
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Step 1: ERC-20 approve
    let approve_result = onchainos::erc20_approve(
        chain_id,
        cfg.base_asset,
        cfg.comet_proxy,
        repay_amount,
        Some(&wallet),
        false,
    )
    .await?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;

    // Step 2: 3-second delay to avoid nonce collision
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Step 3: Comet.supply (= repay)
    let repay_result = onchainos::wallet_contract_call(
        chain_id,
        cfg.comet_proxy,
        &repay_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;
    let repay_tx = onchainos::extract_tx_hash_or_err(&repay_result)?;

    // Wait for repay tx to confirm before reading remaining balance
    onchainos::wait_for_tx(&repay_tx, cfg.rpc_url).await;

    let remaining = rpc::get_borrow_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url)
        .await
        .unwrap_or(0);

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "base_asset": cfg.base_asset_symbol,
            "repaid_amount": format!("{:.6}", repay_amount as f64 / decimals_factor),
            "repaid_amount_raw": repay_amount.to_string(),
            "wallet": wallet,
            "approve_tx_hash": approve_tx,
            "repay_tx_hash": repay_tx,
            "remaining_borrow_balance": format!("{:.6}", remaining as f64 / decimals_factor),
            "remaining_borrow_balance_raw": remaining.to_string()
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
