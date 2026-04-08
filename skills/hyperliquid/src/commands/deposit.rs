use clap::Args;
use sha3::{Digest, Keccak256};
use crate::config::{ARBITRUM_CHAIN_ID, HL_BRIDGE_ARBITRUM, USDC_ARBITRUM};
use crate::onchainos::{resolve_wallet, wallet_contract_call};
use crate::rpc::{ARBITRUM_RPC, erc20_balance, erc20_allowance, pad_address, pad_u256};

#[derive(Args)]
pub struct DepositArgs {
    /// USDC amount to deposit (e.g. 100 for $100 USDC)
    #[arg(long)]
    pub amount: f64,

    /// Dry run — show calldata without submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and broadcast (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

/// ERC-20 approve(address,uint256) calldata.
fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let mut h = Keccak256::new();
    h.update(b"approve(address,uint256)");
    let selector = hex::encode(&h.finalize()[..4]);
    format!("0x{}{}{}", selector, pad_address(spender), pad_u256(amount))
}

/// Hyperliquid bridge deposit(address,uint64) calldata.
/// The bridge credits the user's HL account based on the sender address.
fn build_deposit_calldata(user: &str, usdc_units: u64) -> String {
    let mut h = Keccak256::new();
    h.update(b"deposit(address,uint64)");
    let selector = hex::encode(&h.finalize()[..4]);
    format!("0x{}{}{}", selector, pad_address(user), pad_u256(usdc_units as u128))
}

pub async fn run(args: DepositArgs) -> anyhow::Result<()> {
    if args.amount <= 0.0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // USDC has 6 decimals
    let usdc_units = (args.amount * 1_000_000.0).round() as u64;
    let usdc_u128 = usdc_units as u128;

    let wallet = resolve_wallet(ARBITRUM_CHAIN_ID)?;

    let approve_data = build_approve_calldata(HL_BRIDGE_ARBITRUM, usdc_u128);
    let deposit_data = build_deposit_calldata(&wallet, usdc_units);

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "wallet": wallet,
            "amount_usd": args.amount,
            "usdc_units": usdc_units,
            "bridge": HL_BRIDGE_ARBITRUM,
            "usdc_token": USDC_ARBITRUM,
            "chain": ARBITRUM_CHAIN_ID,
            "step1_approve_calldata": approve_data,
            "step2_deposit_calldata": deposit_data,
            "note": "Dry run — verify calldata before executing with --confirm"
        }));
        return Ok(());
    }

    // Check USDC balance on Arbitrum
    let balance = erc20_balance(USDC_ARBITRUM, &wallet, ARBITRUM_RPC).await?;
    let balance_usd = balance as f64 / 1_000_000.0;
    if balance < usdc_u128 {
        anyhow::bail!(
            "Insufficient USDC on Arbitrum: have {:.2} USDC, need {:.2} USDC",
            balance_usd,
            args.amount
        );
    }

    if !args.confirm {
        let allowance = erc20_allowance(USDC_ARBITRUM, &wallet, HL_BRIDGE_ARBITRUM, ARBITRUM_RPC).await?;
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": true,
            "wallet": wallet,
            "amount_usd": args.amount,
            "usdc_units": usdc_units,
            "usdc_balance": format!("{:.2}", balance_usd),
            "current_allowance": allowance,
            "needs_approve": allowance < usdc_u128,
            "bridge": HL_BRIDGE_ARBITRUM,
            "chain": "arbitrum",
            "note": "Add --confirm to execute"
        }));
        return Ok(());
    }

    // Step 1: Approve if needed
    let allowance = erc20_allowance(USDC_ARBITRUM, &wallet, HL_BRIDGE_ARBITRUM, ARBITRUM_RPC).await?;
    let mut approve_tx_hash = serde_json::Value::Null;
    if allowance < usdc_u128 {
        println!("Approving {} USDC to bridge...", args.amount);
        let approve_result = wallet_contract_call(
            ARBITRUM_CHAIN_ID,
            USDC_ARBITRUM,
            &approve_data,
            None,
            true,
            false,
        )?;
        approve_tx_hash = approve_result["data"]["txHash"]
            .clone()
            .into();
        println!("Approve tx: {}", approve_tx_hash);
        // Brief wait for approval to land
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: Deposit to HL bridge
    println!("Depositing {:.2} USDC to Hyperliquid via Arbitrum bridge...", args.amount);
    let deposit_result = wallet_contract_call(
        ARBITRUM_CHAIN_ID,
        HL_BRIDGE_ARBITRUM,
        &deposit_data,
        None,
        true,
        false,
    )?;
    let deposit_tx_hash = deposit_result["data"]["txHash"]
        .as_str()
        .unwrap_or("pending");

    println!("{}", serde_json::json!({
        "ok": true,
        "action": "deposit",
        "wallet": wallet,
        "amount_usd": args.amount,
        "usdc_units": usdc_units,
        "bridge": HL_BRIDGE_ARBITRUM,
        "approveTxHash": approve_tx_hash,
        "depositTxHash": deposit_tx_hash,
        "note": "USDC bridging from Arbitrum to Hyperliquid typically takes 2-5 minutes."
    }));

    Ok(())
}
