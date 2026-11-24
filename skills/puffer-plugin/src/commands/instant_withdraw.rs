use clap::Args;
use serde_json::json;

use crate::calldata::build_redeem_calldata;
use crate::config::{
    format_units, parse_units, pufeth_address, puffer_vault_address, rpc_url, weth_address,
    CHAIN_ID,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wait_for_tx, wallet_balance, wallet_contract_call};
use crate::rpc::{convert_to_assets, get_total_exit_fee_bps, max_redeem, preview_redeem};

#[derive(Args)]
pub struct InstantWithdrawArgs {
    /// Amount of pufETH to redeem (e.g. "0.1"). Will be burned in the same tx.
    #[arg(long)]
    pub amount: String,
    /// Dry run — build calldata but do not broadcast.
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: InstantWithdrawArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("instant-withdraw")));
    }
    Ok(())
}

async fn run_inner(args: InstantWithdrawArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let vault = puffer_vault_address();
    let pufeth = pufeth_address();

    let amount_raw = parse_units(&args.amount, 18)?;
    if amount_raw == 0 {
        anyhow::bail!("Amount must be greater than zero.");
    }

    let wallet = resolve_wallet(CHAIN_ID)?;

    // Pre-flight balance check via onchainos (→ EVM-001).
    let bal_raw = wallet_balance(CHAIN_ID, Some(pufeth), false).await?;
    if bal_raw < amount_raw {
        anyhow::bail!(
            "Insufficient pufETH balance: need {}, have {}.",
            format_units(amount_raw, 18),
            format_units(bal_raw, 18)
        );
    }

    // Vault liquidity check — if maxRedeem < amount, the redeem will revert.
    let max = max_redeem(vault, &wallet, rpc).await?;
    if max < amount_raw {
        anyhow::bail!(
            "Vault liquidity limit: maxRedeem = {} pufETH, requested {}. Use 2-step request-withdraw for larger amounts.",
            format_units(max, 18),
            format_units(amount_raw, 18)
        );
    }

    let gross_eth_raw = convert_to_assets(vault, amount_raw, rpc).await?;
    let net_weth_raw = preview_redeem(vault, amount_raw, rpc).await?;
    let fee_raw = gross_eth_raw.saturating_sub(net_weth_raw);
    let exit_fee_bps = get_total_exit_fee_bps(vault, rpc).await?;

    // owner = receiver = wallet (no external allowance needed since caller IS the owner).
    let calldata = build_redeem_calldata(amount_raw, &wallet, &wallet);

    // Gas pre-flight (no value sent on redeem).
    let gas = super::check_gas_budget(&wallet, vault, &calldata, 0, rpc).await?;

    eprintln!(
        "Instant (1-step) withdraw: redeem {} pufETH → ~{} WETH (fee {} WETH, {}%)",
        format_units(amount_raw, 18),
        format_units(net_weth_raw, 18),
        format_units(fee_raw, 18),
        (exit_fee_bps as f64) / 100.0
    );
    eprintln!(
        "  Gas: ~{} units × {} gwei = {} ETH (wallet has {} ETH)",
        gas.gas_units,
        format_units(gas.gas_price_wei, 9),
        format_units(gas.estimated_fee_wei, 18),
        format_units(gas.wallet_eth_balance_wei, 18),
    );

    let result = wallet_contract_call(
        CHAIN_ID,
        vault,
        &calldata,
        0,
        args.confirm,
        args.dry_run,
    )
    .await?;

    if result["preview"].as_bool() == Some(true) || result["dry_run"].as_bool() == Some(true) {
        let out = json!({
            "ok": true,
            "action": "instant-withdraw",
            "step": "preview",
            "chain": "ethereum",
            "chain_id": CHAIN_ID,
            "method": "1-step (redeem)",
            "pufeth_amount": format_units(amount_raw, 18),
            "pufeth_amount_raw": amount_raw.to_string(),
            "estimated_weth_out": format_units(net_weth_raw, 18),
            "estimated_weth_out_raw": net_weth_raw.to_string(),
            "gross_eth_equivalent": format_units(gross_eth_raw, 18),
            "gross_eth_equivalent_raw": gross_eth_raw.to_string(),
            "fee_weth": format_units(fee_raw, 18),
            "fee_weth_raw": fee_raw.to_string(),
            "fee_bps": exit_fee_bps,
            "fee_pct": (exit_fee_bps as f64) / 100.0,
            "gas_check": gas.to_json(),
            "delivery": "immediate (same tx)",
            "vault": vault,
            "wallet": wallet,
            "calldata": calldata,
            "next_action": "Re-run with --confirm to broadcast.",
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result).to_string();
    eprintln!("Redeem tx: {} — waiting for confirmation...", tx_hash);
    wait_for_tx(tx_hash.clone(), wallet.clone()).await?;
    eprintln!("Redeem confirmed.");
    // Post-tx reads with cache bypass.
    let new_pufeth = wallet_balance(CHAIN_ID, Some(pufeth), true).await.unwrap_or(0);
    let new_weth = wallet_balance(CHAIN_ID, Some(weth_address()), true).await.unwrap_or(0);

    let out = json!({
        "ok": true,
        "action": "instant-withdraw",
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "method": "1-step (redeem)",
        "tx_hash": tx_hash,
        "pufeth_burned": format_units(amount_raw, 18),
        "pufeth_burned_raw": amount_raw.to_string(),
        "estimated_weth_out": format_units(net_weth_raw, 18),
        "estimated_weth_out_raw": net_weth_raw.to_string(),
        "fee_weth": format_units(fee_raw, 18),
        "fee_weth_raw": fee_raw.to_string(),
        "fee_bps": exit_fee_bps,
        "fee_pct": (exit_fee_bps as f64) / 100.0,
        "delivery": "immediate",
        "new_pufeth_balance": format_units(new_pufeth, 18),
        "new_pufeth_balance_raw": new_pufeth.to_string(),
        "new_weth_balance": format_units(new_weth, 18),
        "new_weth_balance_raw": new_weth.to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
