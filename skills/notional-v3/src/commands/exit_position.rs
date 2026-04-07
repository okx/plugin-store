use crate::{config, onchainos};
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct ExitPositionArgs {
    /// Vault contract address
    #[arg(long)]
    pub vault: String,

    /// Number of vault shares to redeem (raw units, or "all" to exit full position)
    #[arg(long)]
    pub shares: String,

    /// Wallet address (optional, defaults to onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(args: &ExitPositionArgs, dry_run: bool, chain_id: u64) -> anyhow::Result<()> {
    if dry_run {
        let calldata = build_exit_calldata(
            "0x0000000000000000000000000000000000000000",
            &args.vault,
            0,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "dry_run": true,
                "action": "exit-position",
                "vault": args.vault,
                "shares": args.shares,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "calldata": calldata
            }))?
        );
        return Ok(());
    }

    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    // Parse shares amount
    let shares_raw: u128 = if args.shares == "all" {
        // Get current share balance from contract
        let bal = crate::api::get_collateral_balance(&wallet, &args.vault).await?;
        if bal == 0 {
            anyhow::bail!("No position found in vault {}", args.vault);
        }
        bal
    } else {
        args.shares.parse::<u128>().map_err(|_| {
            anyhow::anyhow!(
                "Invalid shares amount '{}'. Use a number or 'all'.",
                args.shares
            )
        })?
    };

    let calldata = build_exit_calldata(&wallet, &args.vault, shares_raw);
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::MORPHO_LENDING_ROUTER,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "action": "exit-position",
            "vault": args.vault,
            "shares": shares_raw.to_string(),
            "wallet": wallet,
            "tx_hash": tx_hash,
            "etherscan": format!("https://etherscan.io/tx/{}", tx_hash)
        }))?
    );
    Ok(())
}

/// Build exitPosition calldata.
/// exitPosition(address onBehalf, address vault, uint256 sharesAmount, uint16 param, bytes data)
fn build_exit_calldata(on_behalf: &str, vault: &str, shares: u128) -> String {
    use alloy_sol_types::{sol, SolCall};

    sol! {
        function exitPosition(
            address onBehalf,
            address vault,
            uint256 sharesAmount,
            uint16 param,
            bytes data
        ) external;
    }

    let on_behalf_addr: alloy_primitives::Address = on_behalf.parse().unwrap_or_default();
    let vault_addr: alloy_primitives::Address = vault.parse().unwrap_or_default();

    let call = exitPositionCall {
        onBehalf: on_behalf_addr,
        vault: vault_addr,
        sharesAmount: alloy_primitives::U256::from(shares),
        param: 0u16,
        data: alloy_primitives::Bytes::new(),
    };

    format!("0x{}", hex::encode(call.abi_encode()))
}
