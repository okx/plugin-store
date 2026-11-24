use clap::Args;
use serde_json::json;

use crate::config::{parse_chain, supported_chains_help, ChainInfo, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    erc20_balance, erc20_decimals, erc20_symbol, fmt_token_amount, get_reserves_list,
    human_to_atomic, lp_get_reserve_data, native_balance, pad_address, pad_u256,
    pad_u256_max, selectors, wait_for_tx,
};

/// Withdraw supplied underlying back to wallet via LendingPool.withdraw(asset, amount, to).
/// Pass `--amount all` (or `max`) to redeem the entire supply position; the LendingPool
/// caps internally at user's aToken balance, so passing uint256.max is safe.
///
/// All operations require explicit `--confirm`. v0.1.0 ERC-20 only - to recover native
/// ETH/MATIC/AVAX, withdraw the wrapped version (WETH/WMATIC/WAVAX) and unwrap externally.
#[derive(Args)]
pub struct WithdrawArgs {
    /// Chain key or id (ETH / POLYGON / AVAX).
    #[arg(long, default_value = "ETH")]
    pub chain: String,
    /// Token symbol (case-insensitive) or 0x asset address.
    #[arg(long)]
    pub token: String,
    /// Underlying amount to withdraw, e.g. 100. Pass `all` / `max` for full balance.
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
    let chain: &ChainInfo = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unknown --chain '{}'", args.chain),
            "INVALID_CHAIN",
            &format!("Supported: {}", supported_chains_help()),
        ),
    };

    let upper = args.token.to_uppercase();
    if upper == chain.native_symbol {
        return print_err(
            &format!("Native {} withdraw deferred to v0.2.0. Withdraw the wrapped W{} instead and unwrap externally.",
                chain.native_symbol, chain.native_symbol),
            "NATIVE_NOT_SUPPORTED_V01",
            &format!("Use --token W{} (wrapped).", chain.native_symbol),
        );
    }

    let (asset_addr, symbol, decimals) = if args.token.starts_with("0x") && args.token.len() == 42 {
        let dec = erc20_decimals(&args.token, chain.rpc).await
            .map_err(|e| anyhow::anyhow!("erc20 decimals: {}", e))?;
        let sym = erc20_symbol(&args.token, chain.rpc).await;
        (args.token.to_lowercase(), sym, dec)
    } else {
        match resolve_symbol(&args.token, chain).await {
            Some(t) => t,
            None => return print_err(
                &format!("Token '{}' not found among Aave V2 reserves on {}", args.token, chain.key),
                "TOKEN_NOT_FOUND",
                "Run `aave-v2-plugin markets --chain X` to see all listed reserves.",
            ),
        }
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Read current supply position via aToken.balanceOf(user)
    let rd = match lp_get_reserve_data(chain.lending_pool, &asset_addr, chain.rpc).await {
        Ok(r) => r,
        Err(e) => return print_err(&format!("LendingPool.getReserveData: {}", e), "RPC_ERROR",
            "Public RPC may be limited; retry shortly."),
    };
    // EVM-012: surface RPC failures distinctly from "user has no supply" — a
    // silent unwrap_or(0) here would tell users they have nothing to withdraw
    // when in fact the public RPC just couldn't fetch the aToken balance.
    let supply_raw = match erc20_balance(&rd.a_token, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read aToken balance for {} on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if supply_raw == 0 {
        return print_err(
            &format!("No {} supplied on Aave V2 {}.", symbol, chain.key),
            "NO_SUPPLY",
            "Nothing to withdraw. Run `aave-v2-plugin positions --chain X` to see all balances.",
        );
    }

    // Resolve amount
    let trim = args.amount.trim();
    let is_all = trim.eq_ignore_ascii_case("all") || trim.eq_ignore_ascii_case("max");
    let (amount_raw_for_check, amount_calldata): (u128, String) = if is_all {
        (supply_raw, pad_u256_max())
    } else {
        let user_atomic = match human_to_atomic(trim, decimals) {
            Ok(v) => v.min(supply_raw),
            Err(e) => return print_err(&format!("Invalid --amount: {}", e),
                "INVALID_ARGUMENT", "Pass a positive number or 'all'."),
        };
        (user_atomic, pad_u256(user_atomic))
    };

    // Pre-flight: native gas
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < chain.gas_floor_wei {
        return print_err(
            &format!("Native {} insufficient on {} (have {}, need {})",
                chain.native_symbol, chain.key,
                fmt_token_amount(native, 18), fmt_token_amount(chain.gas_floor_wei, 18)),
            "INSUFFICIENT_GAS", "Top up native gas.",
        );
    }

    // Build calldata: withdraw(asset, amount, to)
    let calldata = format!("{}{}{}{}",
        selectors::WITHDRAW, pad_address(&asset_addr), amount_calldata, pad_address(&from_addr));

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "withdraw",
            "chain": chain.key,
            "asset": asset_addr,
            "symbol": symbol,
            "current_supply":     fmt_token_amount(supply_raw, decimals),
            "current_supply_raw": supply_raw.to_string(),
            "amount_to_send":     if is_all { "uint256.max (all supply)".to_string() }
                                  else { fmt_token_amount(amount_raw_for_check, decimals) },
            "is_withdraw_all": is_all,
            "call_target": chain.lending_pool,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    let result = match wallet_contract_call(chain.id, chain.lending_pool, &calldata, None, Some(350_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("withdraw failed: {:#}", e),
            "WITHDRAW_SUBMIT_FAILED",
            "Common: insufficient supply / debt would push HF below 1, gas, RPC.",
        ),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[withdraw] Submit tx: {} - waiting...", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on the block explorer.");
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
        "asset": asset_addr,
        "symbol": symbol,
        "is_withdraw_all": is_all,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `aave-v2-plugin positions --chain X` to confirm new state.",
    }))?);
    Ok(())
}

async fn resolve_symbol(token: &str, chain: &ChainInfo) -> Option<(String, String, u32)> {
    let reserves = get_reserves_list(chain.lending_pool, chain.rpc).await.ok()?;
    let upper = token.to_uppercase();
    for asset in reserves {
        let sym = erc20_symbol(&asset, chain.rpc).await;
        if sym.to_uppercase() == upper {
            let dec = erc20_decimals(&asset, chain.rpc).await.unwrap_or(18);
            return Some((asset, sym, dec));
        }
    }
    None
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
