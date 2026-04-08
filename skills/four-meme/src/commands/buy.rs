use clap::Args;

use crate::config::{CHAIN_ID, BSC_RPC, TOKEN_MANAGER_HELPER_V3};
use crate::calldata::{
    build_try_buy_calldata, build_buy_amap_calldata,
    parse_bnb_to_wei, format_wei_as_bnb, format_token_amount, apply_slippage_min,
};
use crate::rpc::{eth_call, decode_uint256, decode_address};
use crate::onchainos::wallet_contract_call;

#[derive(Args)]
pub struct BuyArgs {
    /// Token contract address on BSC
    #[arg(long)]
    pub token: String,

    /// Amount of BNB to spend (e.g. "0.001")
    #[arg(long)]
    pub amount_bnb: String,

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

pub async fn run(args: &BuyArgs) -> anyhow::Result<()> {
    let token = &args.token;
    let funds_wei = parse_bnb_to_wei(&args.amount_bnb);

    if funds_wei == 0 {
        anyhow::bail!("Invalid BNB amount: {}", args.amount_bnb);
    }

    println!("Four.meme Buy");
    println!("{}", "=".repeat(60));
    println!("Token:        {}", token);
    println!("Spend:        {} BNB ({} wei)", args.amount_bnb, funds_wei);
    println!("Slippage:     {} bps ({:.2}%)", args.slippage_bps, args.slippage_bps as f64 / 100.0);

    // 1. Pre-calculate via tryBuy
    let try_buy_data = build_try_buy_calldata(token, 0, funds_wei);
    let try_buy_raw = eth_call(TOKEN_MANAGER_HELPER_V3, &try_buy_data, BSC_RPC).await
        .unwrap_or_else(|_| "0x".to_string());

    // tryBuy returns 8 slots:
    // 0: tokenManager, 1: quote, 2: estimatedAmount, 3: estimatedCost
    // 4: estimatedFee, 5: amountMsgValue, 6: amountApproval, 7: amountFunds
    let est_amount = decode_uint256(&try_buy_raw, 2);
    let est_cost = decode_uint256(&try_buy_raw, 3);
    let est_fee = decode_uint256(&try_buy_raw, 4);
    let amount_msg_value = decode_uint256(&try_buy_raw, 5);
    let token_manager = decode_address(&try_buy_raw, 0);
    let quote = decode_address(&try_buy_raw, 1);

    if est_amount == 0 && try_buy_raw.len() < 10 {
        println!("\nWarning: Could not get price quote from helper contract.");
        println!("The token may have graduated to PancakeSwap.");
    } else if est_amount > 0 {
        println!("\n--- Quote ---");
        println!("Est. Tokens:  {}", format_token_amount(est_amount));
        println!("Est. Cost:    {} BNB", format_wei_as_bnb(est_cost));
        println!("Est. Fee:     {} BNB", format_wei_as_bnb(est_fee));

        if quote != crate::config::ZERO_ADDRESS {
            println!("Quote Token:  {} (ERC20 pair, not BNB)", quote);
            println!("\nNote: This token uses an ERC20 base token, not BNB.");
            println!("Use the correct base token for this pair.");
        }
    }

    // Use amountMsgValue from tryBuy if available, else use funds_wei directly
    let msg_value = if amount_msg_value > 0 { amount_msg_value } else { funds_wei };

    // Use amountFunds from tryBuy slot 7 for the funds param
    let amount_funds_raw = decode_uint256(&try_buy_raw, 7);
    let actual_funds = if amount_funds_raw > 0 { amount_funds_raw } else { funds_wei };

    // Compute min_amount with slippage
    let min_amount = apply_slippage_min(est_amount, args.slippage_bps);
    println!("Min Tokens:   {} (after slippage)", format_token_amount(min_amount));

    // Determine token manager to use
    let tm_to_use = if token_manager != crate::config::ZERO_ADDRESS
        && token_manager != "0x0000000000000000000000000000000000000000"
    {
        token_manager.clone()
    } else {
        crate::config::TOKEN_MANAGER_V2.to_string()
    };

    // Build buy calldata: buyTokenAMAP(address token, uint256 funds, uint256 minAmount)
    let calldata = build_buy_amap_calldata(token, actual_funds, min_amount);

    println!("\n--- Transaction ---");
    println!("To:           {}", tm_to_use);
    println!("Value:        {} BNB ({} wei)", format_wei_as_bnb(msg_value), msg_value);
    println!("Calldata:     {}...{}", &calldata[..10], &calldata[calldata.len() - 8..]);

    if !args.confirm && !args.dry_run {
        println!("\nPreview mode. Add --confirm to broadcast.");
        println!("Or add --dry-run to simulate without broadcasting.");
    }

    let result = wallet_contract_call(
        CHAIN_ID,
        &tm_to_use,
        &calldata,
        msg_value,
        args.confirm,
        args.dry_run,
    )
    .await?;

    if args.confirm || args.dry_run {
        println!("\n--- Result ---");
        println!("{}", serde_json::to_string_pretty(&result)?);

        if let Some(tx_hash) = result["data"]["txHash"].as_str() {
            if tx_hash != "0x0000000000000000000000000000000000000000000000000000000000000000" {
                println!("\nBscScan: https://bscscan.com/tx/{}", tx_hash);
            }
        }
    } else {
        println!("\n{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}
