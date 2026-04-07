use crate::{config, onchainos};
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct EnterPositionArgs {
    /// Vault contract address
    #[arg(long)]
    pub vault: String,

    /// Amount of underlying asset to deposit (in UI units, e.g. "0.01" for 0.01 USDC)
    #[arg(long)]
    pub amount: f64,

    /// Borrow amount in asset units (0 = no leverage, default 0)
    #[arg(long, default_value = "0")]
    pub borrow_amount: f64,

    /// Asset token symbol or address (USDC or WETH, default USDC)
    #[arg(long, default_value = "USDC")]
    pub asset: String,

    /// Wallet address (optional, defaults to onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(args: &EnterPositionArgs, dry_run: bool, chain_id: u64) -> anyhow::Result<()> {
    // dry-run early return before wallet resolution
    if dry_run {
        let calldata = build_enter_calldata(
            "0x0000000000000000000000000000000000000000",
            &args.vault,
            0,
            0,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "dry_run": true,
                "action": "enter-position",
                "vault": args.vault,
                "amount": args.amount,
                "borrow_amount": args.borrow_amount,
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

    // Resolve asset address and decimals
    let (asset_addr, decimals) = resolve_asset(&args.asset)?;

    // Convert amount to raw units
    let amount_raw = (args.amount * 10f64.powi(decimals as i32)) as u128;
    let borrow_raw = (args.borrow_amount * 10f64.powi(decimals as i32)) as u128;

    // Step 1: ERC-20 approve
    println!("Step 1/2: Approving {} to MorphoLendingRouter...", &args.asset);
    let approve_result = onchainos::erc20_approve(
        chain_id,
        asset_addr,
        config::MORPHO_LENDING_ROUTER,
        amount_raw,
        Some(&wallet),
        false,
    )
    .await?;

    if !approve_result["ok"].as_bool().unwrap_or(false) {
        anyhow::bail!(
            "Approve failed: {}",
            approve_result.to_string()
        );
    }

    // Wait 15s between approve and deposit — approve must be confirmed before enterPosition
    // Per KNOWLEDGE_HUB: 2-tx flows (approve → deposit) need ~15s delay on Ethereum
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    // Step 2: enterPosition
    println!("Step 2/2: Entering vault position...");
    let calldata = build_enter_calldata(&wallet, &args.vault, amount_raw, borrow_raw);
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
            "action": "enter-position",
            "vault": args.vault,
            "amount": args.amount,
            "borrow_amount": args.borrow_amount,
            "asset": args.asset,
            "wallet": wallet,
            "tx_hash": tx_hash,
            "etherscan": format!("https://etherscan.io/tx/{}", tx_hash)
        }))?
    );
    Ok(())
}

/// Build enterPosition calldata.
/// enterPosition(address onBehalf, address vault, uint256 depositAmount, uint256 borrowAmount, bytes depositData)
fn build_enter_calldata(
    on_behalf: &str,
    vault: &str,
    deposit_amount: u128,
    borrow_amount: u128,
) -> String {
    use alloy_sol_types::{sol, SolCall};

    sol! {
        function enterPosition(
            address onBehalf,
            address vault,
            uint256 depositAssetAmount,
            uint256 borrowAmount,
            bytes depositData
        ) external;
    }

    let on_behalf_addr: alloy_primitives::Address = on_behalf.parse().unwrap_or_default();
    let vault_addr: alloy_primitives::Address = vault.parse().unwrap_or_default();

    let call = enterPositionCall {
        onBehalf: on_behalf_addr,
        vault: vault_addr,
        depositAssetAmount: alloy_primitives::U256::from(deposit_amount),
        borrowAmount: alloy_primitives::U256::from(borrow_amount),
        depositData: alloy_primitives::Bytes::new(),
    };

    format!("0x{}", hex::encode(call.abi_encode()))
}

fn resolve_asset(asset: &str) -> anyhow::Result<(&'static str, u8)> {
    match asset.to_uppercase().as_str() {
        "USDC" => Ok((config::USDC_ETH, 6)),
        "WETH" | "ETH" => Ok((config::WETH_ETH, 18)),
        other => {
            // Treat as address
            if other.starts_with("0X") || other.starts_with("0x") {
                // Return as-is with 18 decimals (unknown)
                Ok((config::USDC_ETH, 18)) // fallback
            } else {
                anyhow::bail!("Unknown asset: {}. Use USDC or WETH.", asset)
            }
        }
    }
}
