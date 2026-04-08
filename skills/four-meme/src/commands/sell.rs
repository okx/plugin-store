use clap::Args;

use crate::config::{CHAIN_ID, BSC_RPC, TOKEN_MANAGER_HELPER_V3};
use crate::calldata::{
    build_try_sell_calldata, build_sell_calldata, build_approve_calldata,
    format_wei_as_bnb, format_token_amount, apply_slippage_min,
};
use crate::rpc::{eth_call, decode_uint256, decode_address};
use crate::onchainos::wallet_contract_call;

#[derive(Args)]
pub struct SellArgs {
    /// Token contract address on BSC
    #[arg(long)]
    pub token: String,

    /// Amount of tokens to sell (in token units, e.g. "1000000" for 1M tokens)
    #[arg(long)]
    pub amount: String,

    /// Slippage tolerance in basis points (e.g. 100 = 1%, default 100)
    #[arg(long, default_value = "100")]
    pub slippage_bps: u64,

    /// Broadcast the transaction on-chain
    #[arg(long)]
    pub confirm: bool,

    /// Simulate without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

/// Parse token amount string (whole tokens, e.g. "1000000") to raw wei-like units (18 decimals)
fn parse_token_amount(amount_str: &str) -> anyhow::Result<u128> {
    let parts: Vec<&str> = amount_str.split('.').collect();
    let whole: u128 = parts[0].parse()
        .map_err(|_| anyhow::anyhow!("Invalid token amount: {}", amount_str))?;
    let frac_str = parts.get(1).copied().unwrap_or("0");
    let frac_pad = format!("{:0<18}", frac_str);
    let frac18: u128 = frac_pad[..18].parse().unwrap_or(0);
    Ok(whole * 1_000_000_000_000_000_000u128 + frac18)
}

pub async fn run(args: &SellArgs) -> anyhow::Result<()> {
    let token = &args.token;
    let token_amount = parse_token_amount(&args.amount)?;

    if token_amount == 0 {
        anyhow::bail!("Invalid token amount: {}", args.amount);
    }

    println!("Four.meme Sell");
    println!("{}", "=".repeat(60));
    println!("Token:        {}", token);
    println!("Amount:       {} tokens", format_token_amount(token_amount));
    println!("Slippage:     {} bps ({:.2}%)", args.slippage_bps, args.slippage_bps as f64 / 100.0);

    // 1. Pre-calculate via trySell
    let try_sell_data = build_try_sell_calldata(token, token_amount);
    let try_sell_raw = eth_call(TOKEN_MANAGER_HELPER_V3, &try_sell_data, BSC_RPC).await
        .unwrap_or_else(|_| "0x".to_string());

    // trySell returns 4 slots: tokenManager, quote, funds, fee
    let token_manager = decode_address(&try_sell_raw, 0);
    let quote = decode_address(&try_sell_raw, 1);
    let est_funds = decode_uint256(&try_sell_raw, 2);
    let est_fee = decode_uint256(&try_sell_raw, 3);

    if est_funds > 0 {
        println!("\n--- Quote ---");
        println!("Est. Receive: {} BNB", format_wei_as_bnb(est_funds));
        println!("Est. Fee:     {} BNB", format_wei_as_bnb(est_fee));
        if quote != crate::config::ZERO_ADDRESS {
            println!("Quote Token:  {} (ERC20 pair)", quote);
        }
    } else if try_sell_raw.len() < 10 {
        println!("\nWarning: Could not get sell quote. Token may have graduated.");
    }

    // Compute min funds with slippage
    let min_funds = apply_slippage_min(est_funds, args.slippage_bps);
    println!("Min Receive:  {} BNB (after slippage)", format_wei_as_bnb(min_funds));

    // Determine token manager
    let tm_to_use = if token_manager != crate::config::ZERO_ADDRESS
        && token_manager != "0x0000000000000000000000000000000000000000"
    {
        token_manager.clone()
    } else {
        crate::config::TOKEN_MANAGER_V2.to_string()
    };

    println!("\n--- Approve Step ---");
    println!("Before selling, the token contract must approve the TokenManager.");
    println!("Token Manager: {}", tm_to_use);

    // Build approve calldata: approve(address spender, uint256 amount)
    let approve_calldata = build_approve_calldata(&tm_to_use, token_amount);
    println!("Approve:      {} tokens to {}", format_token_amount(token_amount), tm_to_use);

    // Build sell calldata: sellToken(address token, uint256 amount, uint256 minFunds)
    let sell_calldata = build_sell_calldata(token, token_amount, min_funds);

    println!("\n--- Transaction ---");
    println!("Step 1: Approve {} to spend tokens", tm_to_use);
    println!("Step 2: Call sellToken on {}", tm_to_use);
    println!("Sell Calldata: {}...{}", &sell_calldata[..10], &sell_calldata[sell_calldata.len() - 8..]);

    if !args.confirm && !args.dry_run {
        println!("\nPreview mode. Add --confirm to broadcast both transactions.");
        println!("Or add --dry-run to simulate without broadcasting.");
    }

    // Step 1: Approve
    println!("\n--- Step 1: Approve ---");
    let approve_result = wallet_contract_call(
        CHAIN_ID,
        token,          // call ERC20 token contract
        &approve_calldata,
        0,              // no BNB value for approve
        args.confirm,
        args.dry_run,
    )
    .await?;

    println!("{}", serde_json::to_string_pretty(&approve_result)?);

    // Only proceed to sell if approve was sent
    if args.confirm || args.dry_run {
        if let Some(tx_hash) = approve_result["data"]["txHash"].as_str() {
            if tx_hash != "0x0000000000000000000000000000000000000000000000000000000000000000"
                && !tx_hash.is_empty()
            {
                println!("Approve TX: https://bscscan.com/tx/{}", tx_hash);
            }
        }

        // Step 2: Sell
        println!("\n--- Step 2: Sell ---");
        let sell_result = wallet_contract_call(
            CHAIN_ID,
            &tm_to_use,
            &sell_calldata,
            0,           // no BNB value for sell
            args.confirm,
            args.dry_run,
        )
        .await?;

        println!("{}", serde_json::to_string_pretty(&sell_result)?);

        if let Some(tx_hash) = sell_result["data"]["txHash"].as_str() {
            if tx_hash != "0x0000000000000000000000000000000000000000000000000000000000000000"
                && !tx_hash.is_empty()
            {
                println!("\nSell TX: https://bscscan.com/tx/{}", tx_hash);
            }
        }
    }

    Ok(())
}
