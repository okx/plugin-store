use clap::Args;
use serde_json::json;

use crate::calldata::build_deposit_eth_calldata;
use crate::config::{format_units, parse_units, pufeth_address, puffer_vault_address, rpc_url, CHAIN_ID};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wait_for_tx, wallet_balance, wallet_contract_call};
use crate::rpc::convert_to_assets;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to deposit (e.g. "0.05", "1.5")
    #[arg(long)]
    pub amount: String,
    /// Dry run — build calldata but do not broadcast.
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("stake")));
    }
    Ok(())
}

async fn run_inner(args: StakeArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let vault = puffer_vault_address();

    let eth_wei = parse_units(&args.amount, 18)?;
    if eth_wei == 0 {
        anyhow::bail!("Amount must be greater than zero.");
    }

    let wallet = resolve_wallet(CHAIN_ID)?;

    // Quote receive amount via convertToAssets(1e18) → ETH per pufETH.
    // shares_out = eth_wei * 1e18 / assets_per_share. If rate call fails we fall back
    // to a conservative 1:1 estimate (pufETH rate never dips below 1:1 by design).
    let one_share_assets = convert_to_assets(vault, 1_000_000_000_000_000_000, rpc)
        .await
        .unwrap_or(1_000_000_000_000_000_000);
    let est_pufeth_out = if one_share_assets == 0 {
        eth_wei
    } else {
        (eth_wei
            .checked_mul(1_000_000_000_000_000_000)
            .ok_or_else(|| anyhow::anyhow!("overflow computing estimated pufETH out"))?)
            / one_share_assets
    };

    let calldata = build_deposit_eth_calldata(&wallet);

    // Gas + ETH-for-value pre-flight: wallet must have amount + estimated_gas_fee.
    let gas = super::check_gas_budget(&wallet, vault, &calldata, eth_wei, rpc).await?;

    eprintln!(
        "Staking {} ETH ({} wei) via PufferVault.depositETH()",
        args.amount, eth_wei
    );
    eprintln!("  PufferVault: {}", vault);
    eprintln!("  Wallet: {}", wallet);
    eprintln!("  Current rate: 1 pufETH = {} ETH", format_units(one_share_assets, 18));
    eprintln!("  Estimated receive: ~{} pufETH", format_units(est_pufeth_out, 18));
    eprintln!(
        "  Gas: ~{} units × {} gwei = {} ETH (wallet has {} ETH)",
        gas.gas_units,
        format_units(gas.gas_price_wei, 9),
        format_units(gas.estimated_fee_wei, 18),
        format_units(gas.wallet_eth_balance_wei, 18),
    );
    eprintln!("  Run with --confirm to broadcast.");

    let result = wallet_contract_call(
        CHAIN_ID,
        vault,
        &calldata,
        eth_wei,
        args.confirm,
        args.dry_run,
    )
    .await?;

    if result["preview"].as_bool() == Some(true) || result["dry_run"].as_bool() == Some(true) {
        let out = json!({
            "ok": true,
            "action": "stake",
            "step": "preview",
            "chain": "ethereum",
            "chain_id": CHAIN_ID,
            "amount_in": format_units(eth_wei, 18),
            "amount_in_raw": eth_wei.to_string(),
            "asset_in": "ETH",
            "estimated_pufeth_out": format_units(est_pufeth_out, 18),
            "estimated_pufeth_out_raw": est_pufeth_out.to_string(),
            "gas_check": gas.to_json(),
            "pufeth_to_eth_rate": format_units(one_share_assets, 18),
            "vault": vault,
            "wallet": wallet,
            "calldata": calldata,
            "next_action": "Re-run with --confirm to broadcast.",
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result).to_string();
    // Wait for the deposit tx to land on-chain before reading the post-state balance.
    // Otherwise `get_balance` hits the latest block while the tx is still in the mempool
    // and returns the stale (pre-deposit) value (→ EVM-006 rationale — also applies after writes).
    eprintln!("Stake tx: {} — waiting for confirmation...", tx_hash);
    wait_for_tx(tx_hash.clone(), wallet.clone()).await?;
    eprintln!("Stake confirmed.");
    // Force-refresh post-tx read so the onchainos cache doesn't return pre-stake value.
    let new_pufeth_raw = wallet_balance(CHAIN_ID, Some(pufeth_address()), true).await.unwrap_or(0);

    let out = json!({
        "ok": true,
        "action": "stake",
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "tx_hash": tx_hash,
        "amount_in": format_units(eth_wei, 18),
        "amount_in_raw": eth_wei.to_string(),
        "asset_in": "ETH",
        "estimated_pufeth_out": format_units(est_pufeth_out, 18),
        "estimated_pufeth_out_raw": est_pufeth_out.to_string(),
        "gas_check": gas.to_json(),
        "new_pufeth_balance": format_units(new_pufeth_raw, 18),
        "new_pufeth_balance_raw": new_pufeth_raw.to_string(),
        "pufeth_to_eth_rate": format_units(one_share_assets, 18),
        "vault": vault,
        "wallet": wallet,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
