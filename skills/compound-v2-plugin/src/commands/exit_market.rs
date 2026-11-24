use clap::Args;
use serde_json::json;

use crate::config::{resolve_market, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    borrow_balance_current, get_assets_in, native_balance, pad_address, selectors, wait_for_tx,
};

/// Remove a cToken from the collateral set via Comptroller.exitMarket(cToken).
///
/// Will revert if any of:
///   - You have an outstanding borrow in this cToken (must `repay --all` first)
///   - Removing this collateral would push your account into shortfall (other debts unsupported)
#[derive(Args)]
pub struct ExitMarketArgs {
    /// cToken symbol (cDAI, cUSDC, …) or 0x cToken address.
    /// Plural `--ctokens` is accepted as an alias for symmetry with `enter-markets`.
    #[arg(long, alias = "ctokens")]
    pub ctoken: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

pub async fn run(args: ExitMarketArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let info = match resolve_market(&args.ctoken) {
        Some(i) => i,
        None => return print_err(
            &format!("Unknown cToken '{}'", args.ctoken),
            "TOKEN_NOT_FOUND",
            "Use one of cDAI/cUSDC/cUSDT/cETH/cWBTC2/cCOMP.",
        ),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Pre-flight: gas, current borrow on this cToken (must be 0), entered status
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < 5_000_000_000_000_000 {
        return print_err("Native ETH below 0.005 floor", "INSUFFICIENT_GAS",
            "Top up at least 0.005 ETH on mainnet.");
    }
    // EVM-012: distinguish RPC failure from "no debt". A silent unwrap_or(0)
    // would let exit_market proceed despite a real outstanding borrow (since
    // the L54 `if debt > 0` guard would be skipped) — risking liquidation if
    // the user removed collateral they actually still need.
    let debt = match borrow_balance_current(info.ctoken, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read borrow balance for {} on {}: {:#}", info.symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly. exit_market needs an authoritative \
             debt read to know it's safe to exit.",
        ),
    };
    if debt > 0 {
        return print_err(
            &format!(
                "Cannot exit {} market — you still owe {} {}. Repay first via `repay --token {} --all --confirm`.",
                info.symbol, crate::rpc::fmt_token_amount(debt, info.underlying_decimals),
                info.underlying_symbol, info.underlying_symbol,
            ),
            "ACTIVE_BORROW",
            "Repay your debt in this market first.",
        );
    }
    let assets_in = get_assets_in(chain.comptroller, &from_addr, chain.rpc).await.unwrap_or_default();
    let already_in = assets_in.iter().any(|a| a.eq_ignore_ascii_case(info.ctoken));
    if !already_in {
        return print_err(
            &format!("{} is not currently in your collateral set — nothing to exit.", info.symbol),
            "NOT_IN_MARKET",
            "Run `compound-v2-plugin positions` to see entered cTokens.",
        );
    }

    // Build calldata: exitMarket(cToken)
    let calldata = format!("{}{}", selectors::EXIT_MARKET, pad_address(info.ctoken));

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "exit_market",
            "chain": chain.key,
            "holder": from_addr,
            "comptroller": chain.comptroller,
            "ctoken": info.ctoken,
            "ctoken_symbol": info.symbol,
            "underlying_symbol": info.underlying_symbol,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    let result = match wallet_contract_call(chain.id, chain.comptroller, &calldata, None, Some(150_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(&format!("exitMarket failed: {:#}", e),
            "EXIT_MARKET_FAILED",
            "Common: removing this collateral would cause shortfall on existing borrows. Reduce other debts first."),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[exit-market] Submit tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on Etherscan.");
            }
            eprintln!("[exit-market] On-chain confirmed.");
        }
        None => return print_err("exitMarket broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "exit_market",
        "chain": chain.key,
        "ctoken_exited": info.ctoken,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "cToken no longer counts as collateral. Run `compound-v2-plugin positions` to verify.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
