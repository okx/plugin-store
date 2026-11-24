use clap::Args;
use serde_json::json;

use crate::config::{resolve_market_id, token_decimals, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    erc20_decimals, fmt_token_amount, get_account_wei, human_to_atomic,
    native_balance, pad_u256, selectors, wait_for_tx,
};

#[derive(Args)]
pub struct WithdrawArgs {
    /// Token symbol or 0x address
    #[arg(long)]
    pub token: String,
    /// Human-readable amount to withdraw (e.g. 50 = 50 USDC)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Account number to withdraw FROM (default 0 = main)
    #[arg(long, default_value = "0")]
    pub from_account_number: u128,
    /// BalanceCheckFlag: 0=None 1=From 2=To 3=Both. Use 3 for plain withdraw to enforce no-overdraft.
    #[arg(long, default_value = "3")]
    pub balance_check_flag: u128,
    /// Dry run
    #[arg(long)]
    pub dry_run: bool,
    /// Required to actually submit
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let (market_id, symbol, token_addr) = match resolve_market_id(&args.token) {
        Some(t) => t,
        None => return print_err(
            &format!("Unknown token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of USDC / USDT / WETH / DAI / WBTC / ARB / USDC.e, or pass the 0x address.",
        ),
    };
    let decimals = token_decimals(symbol)
        .or(erc20_decimals(token_addr, chain.rpc).await.ok())
        .unwrap_or(18);

    let amount_raw = match human_to_atomic(&args.amount, decimals) {
        Ok(v) => v,
        Err(e) => return print_err(&format!("Invalid --amount: {}", e), "INVALID_ARGUMENT",
            "Pass a positive number, e.g. --amount 50"),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Pre-flight: check user has sufficient supply position. EVM-012:
    // RPC failure must not be silently rendered as "supply=0" — that
    // would tell users they have nothing to withdraw when they actually do.
    let (sign, supply_value) = match get_account_wei(
        chain.dolomite_margin, &from_addr, args.from_account_number, market_id as u128, chain.rpc,
    ).await {
        Ok(t) => t,
        Err(e) => return print_err(
            &format!("Failed to read {} supply position from DolomiteMargin on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if !sign || supply_value < amount_raw {
        return print_err(
            &format!(
                "Account {} has only {} {} supplied (raw {}); cannot withdraw {} (raw {})",
                args.from_account_number,
                fmt_token_amount(supply_value, decimals), symbol, supply_value,
                fmt_token_amount(amount_raw, decimals), amount_raw,
            ),
            "INSUFFICIENT_SUPPLY",
            "Reduce --amount or use the correct --from-account-number. Run `positions` to see balances.",
        );
    }

    // Native gas
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < 500_000_000_000_000 {
        return print_err("Native ETH on Arbitrum below floor", "INSUFFICIENT_GAS",
            "Top up at least 0.0005 ETH on Arbitrum.");
    }

    // Build calldata: withdrawWei(fromAccountNumber, marketId, amount, balanceCheckFlag)
    let calldata = format!(
        "{}{}{}{}{}",
        selectors::WITHDRAW_WEI,
        pad_u256(args.from_account_number),
        pad_u256(market_id as u128),
        pad_u256(amount_raw),
        pad_u256(args.balance_check_flag),
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "withdraw",
            "chain": chain.key,
            "token": symbol,
            "market_id": market_id,
            "from_account_number": args.from_account_number,
            "amount":     fmt_token_amount(amount_raw, decimals),
            "amount_raw": amount_raw.to_string(),
            "current_supply":     fmt_token_amount(supply_value, decimals),
            "current_supply_raw": supply_value.to_string(),
            "spender": chain.deposit_withdrawal_proxy,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm."); return Ok(()); }

    // Withdraw — no approve needed (DolomiteMargin owns the supplied tokens)
    let result = match wallet_contract_call(chain.id, chain.deposit_withdrawal_proxy, &calldata, None, Some(400_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("Withdraw submission failed: {:#}", e),
            "WITHDRAW_SUBMIT_FAILED",
            "Inspect onchainos output. Common: insufficient supply, gas, RPC.",
        ),
    };
    let tx_hash = extract_tx_hash(&result);

    // TX-001
    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[withdraw] Submit tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, 180).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e), "TX_REVERTED",
                    "On-chain revert. Inspect on Arbiscan.");
            }
            eprintln!("[withdraw] On-chain confirmed.");
        }
        None => return print_err("Withdraw broadcast but no tx hash", "TX_HASH_MISSING",
            "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "withdraw",
        "chain": chain.key,
        "token": symbol,
        "amount":     fmt_token_amount(amount_raw, decimals),
        "amount_raw": amount_raw.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `dolomite-plugin positions` to confirm new state.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
