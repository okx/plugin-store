use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    market: &str,
    asset: &str,        // token contract address to supply
    amount_str: &str,   // human-readable amount (e.g. "1.5" for 1.5 USDC, "0.001" for 0.001 WETH)
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

    // BUG-5 pre-check: verify wallet has sufficient token balance before spending gas on approve
    let wallet_token_balance = rpc::get_erc20_balance(asset, &wallet, cfg.rpc_url).await.unwrap_or(u128::MAX);
    if wallet_token_balance < amount {
        let decimals_factor = 10u128.pow(asset_decimals as u32) as f64;
        anyhow::bail!(
            "Insufficient wallet balance: wallet {} has {:.decimals$} of token {} but needs {:.decimals$}. \
             Acquire more of this token before supplying.",
            wallet,
            wallet_token_balance as f64 / decimals_factor,
            asset,
            amount as f64 / decimals_factor,
            decimals = asset_decimals as usize
        );
    }

    // Build supply(address,uint256) calldata
    // selector: 0xf2b9fdb8
    let asset_padded = rpc::pad_address(asset);
    let amount_hex = rpc::pad_u128(amount);
    let supply_calldata = format!("0xf2b9fdb8{}{}", asset_padded, amount_hex);

    // Confirm gate: show preview and exit if --confirm not given (and not dry-run)
    if !dry_run && !confirm {
        let decimals_factor = 10u128.pow(asset_decimals as u32) as f64;
        let result = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "supply",
            "chain_id": chain_id,
            "market": market,
            "asset": asset,
            "amount": amount_str,
            "amount_raw": amount.to_string(),
            "amount_human": format!("{:.decimals$}", amount as f64 / decimals_factor, decimals = asset_decimals as usize),
            "comet": cfg.comet_proxy,
            "pending_transactions": 2,
            "transactions": [
                {"step": 1, "action": "ERC-20 approve", "token": asset, "spender": cfg.comet_proxy, "amount_raw": amount.to_string()},
                {"step": 2, "action": "Comet.supply", "comet": cfg.comet_proxy, "asset": asset, "amount_raw": amount.to_string(), "calldata": supply_calldata}
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
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;

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
    let supply_tx = onchainos::extract_tx_hash_or_err(&supply_result)?;

    // Wait for supply tx to confirm before reading post-tx balances
    onchainos::wait_for_tx(&supply_tx, cfg.rpc_url).await;

    // Read the correct post-tx balance:
    // - base asset supplied → read balanceOf (supply position)
    // - collateral asset supplied → read collateralBalanceOf (collateral slot)
    let is_base = asset.eq_ignore_ascii_case(cfg.base_asset);
    let (balance_key, balance_raw_key, new_balance_human, new_balance_raw) = if is_base {
        let bal = rpc::get_balance_of(cfg.comet_proxy, &wallet, cfg.rpc_url).await.unwrap_or(0);
        let factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;
        ("new_supply_balance", "new_supply_balance_raw", format!("{:.6}", bal as f64 / factor), bal.to_string())
    } else {
        let bal = rpc::get_collateral_balance_of(cfg.comet_proxy, &wallet, asset, cfg.rpc_url).await.unwrap_or(0);
        let factor = 10u128.pow(asset_decimals as u32) as f64;
        ("new_collateral_balance", "new_collateral_balance_raw", format!("{:.decimals$}", bal as f64 / factor, decimals = asset_decimals as usize), bal.to_string())
    };

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
            balance_key: new_balance_human,
            balance_raw_key: new_balance_raw
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
