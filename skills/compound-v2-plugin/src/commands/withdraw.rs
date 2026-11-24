use clap::Args;
use serde_json::json;

use crate::config::{resolve_market, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    balance_of_underlying, fmt_token_amount, human_to_atomic, native_balance, pad_u256,
    selectors, wait_for_tx,
};

#[derive(Args)]
pub struct WithdrawArgs {
    /// Token symbol (DAI / USDC / USDT / ETH / WBTC / COMP) or 0x address
    #[arg(long)]
    pub token: String,
    /// Amount of underlying to withdraw, e.g. 100. Pass `all` to redeem entire supply position.
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let info = match resolve_market(&args.token) {
        Some(i) => i,
        None => return print_err(
            &format!("Unknown token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of DAI / USDC / USDT / ETH / WBTC / COMP.",
        ),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Read current supply position
    let supply_raw = match balance_of_underlying(info.ctoken, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(&format!("RPC: {}", e), "RPC_ERROR",
            "Public Ethereum RPC may be limited; retry shortly."),
    };
    if supply_raw == 0 {
        return print_err(
            &format!("No {} supplied on Compound V2.", info.underlying_symbol),
            "NO_SUPPLY",
            "Nothing to withdraw. Run `compound-v2-plugin positions` to see all balances.",
        );
    }

    // Resolve amount: numeric or "all"
    let is_all = args.amount.trim().eq_ignore_ascii_case("all")
        || args.amount.trim().eq_ignore_ascii_case("max");
    let amount_raw: u128 = if is_all {
        supply_raw  // exact stored balance — Compound caps redeemUnderlying at user balance
    } else {
        match human_to_atomic(&args.amount, info.underlying_decimals) {
            Ok(v) => v.min(supply_raw),
            Err(e) => return print_err(&format!("Invalid --amount: {}", e),
                "INVALID_ARGUMENT", "Pass a positive number or 'all'."),
        }
    };

    // Pre-flight: native gas
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < 5_000_000_000_000_000 {
        return print_err("Native ETH below 0.005 floor (Ethereum L1 gas-heavy)",
            "INSUFFICIENT_GAS", "Top up at least 0.005 ETH on mainnet.");
    }

    // Build calldata: redeemUnderlying(amount)
    let calldata = format!("{}{}", selectors::REDEEM_UNDERLYING, pad_u256(amount_raw));

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "withdraw",
            "chain": chain.key,
            "ctoken": info.ctoken,
            "ctoken_symbol": info.symbol,
            "underlying_symbol": info.underlying_symbol,
            "current_supply":     fmt_token_amount(supply_raw, info.underlying_decimals),
            "current_supply_raw": supply_raw.to_string(),
            "amount":     fmt_token_amount(amount_raw, info.underlying_decimals),
            "amount_raw": amount_raw.to_string(),
            "is_redeem_all": is_all,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    // Submit redeemUnderlying — gas: 250k for ERC20 cToken, 280k for cETH (extra ETH transfer)
    let gas_limit = if info.is_native { 320_000 } else { 280_000 };
    let result = match wallet_contract_call(chain.id, info.ctoken, &calldata, None, Some(gas_limit), false) {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("redeemUnderlying failed: {:#}", e),
            "WITHDRAW_SUBMIT_FAILED",
            "Common: insufficient supply (interest-only redeem can fail at exact 'all'), liquidity exhaustion, gas, RPC.",
        ),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[withdraw] Submit tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on Etherscan.");
            }
            eprintln!("[withdraw] On-chain confirmed.");
        }
        None => return print_err("Withdraw broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "withdraw",
        "chain": chain.key,
        "underlying_symbol": info.underlying_symbol,
        "amount":     fmt_token_amount(amount_raw, info.underlying_decimals),
        "amount_raw": amount_raw.to_string(),
        "is_redeem_all": is_all,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `compound-v2-plugin positions` to confirm new state. To migrate yield, deposit to compound-v3-plugin.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
