use clap::Args;
use serde_json::json;

use crate::config::{resolve_market_id, token_decimals, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, erc20_decimals, fmt_token_amount,
    human_to_atomic, native_balance, pad_u256, selectors, wait_for_tx,
};

#[derive(Args)]
pub struct SupplyArgs {
    /// Token symbol (USDC / USDT / WETH / DAI / WBTC / ARB) or 0x address
    #[arg(long)]
    pub token: String,
    /// Human-readable amount (e.g. 100 = 100 USDC)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Account number to deposit into (0 = main; default)
    #[arg(long, default_value = "0")]
    pub to_account_number: u128,
    /// Dry run — fetch state, prepare calldata, do not sign
    #[arg(long)]
    pub dry_run: bool,
    /// Required to actually submit
    #[arg(long)]
    pub confirm: bool,
    /// Approve confirmation timeout (default 180)
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: SupplyArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    // Resolve token → market_id + address
    let (market_id, symbol, token_addr) = match resolve_market_id(&args.token) {
        Some(t) => t,
        None => return print_err(
            &format!("Unknown token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of USDC / USDT / WETH / DAI / WBTC / ARB / USDC.e, or pass the 0x token address. Run `dolomite-plugin markets --all` for the full list.",
        ),
    };
    let decimals = token_decimals(symbol)
        .or(erc20_decimals(token_addr, chain.rpc).await.ok())
        .unwrap_or(18);

    let amount_raw = match human_to_atomic(&args.amount, decimals) {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Invalid --amount '{}': {}", args.amount, e),
            "INVALID_ARGUMENT",
            "Pass a positive number, e.g. --amount 100",
        ),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(
            &format!("Wallet resolve failed: {:#}", e),
            "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses` to verify login.",
        ),
    };

    // Pre-flight: token balance (EVM-001)
    let bal = match erc20_balance(token_addr, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(&format!("Failed to read {} balance: {:#}", symbol, e), "RPC_ERROR",
            "Public Arbitrum RPC may be limited; retry shortly."),
    };
    if bal < amount_raw {
        return print_err(
            &format!(
                "Insufficient {}: need {} (raw {}), have {} (raw {})",
                symbol, fmt_token_amount(amount_raw, decimals), amount_raw,
                fmt_token_amount(bal, decimals), bal,
            ),
            "INSUFFICIENT_BALANCE",
            "Top up the token, or reduce --amount.",
        );
    }

    // Pre-flight: native gas (EVM-012 — surface RPC error explicitly)
    let native = match native_balance(&from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(&format!("Failed to read ETH balance: {:#}", e), "RPC_ERROR",
            "Public Arbitrum RPC may be limited; retry shortly."),
    };
    let gas_floor: u128 = 500_000_000_000_000; // 0.0005 ETH
    if native < gas_floor {
        return print_err(
            &format!("Native ETH on Arbitrum is {} (~$1.15 floor needed)", fmt_token_amount(native, 18)),
            "INSUFFICIENT_GAS",
            "Top up ETH on Arbitrum.",
        );
    }

    // Build calldata: depositWei(toAccountNumber, marketId, amount)
    let calldata = format!(
        "{}{}{}{}",
        selectors::DEPOSIT_WEI,
        pad_u256(args.to_account_number),
        pad_u256(market_id as u128),
        pad_u256(amount_raw),
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "supply",
            "chain": chain.key,
            "from": from_addr,
            "token": symbol,
            "token_address": token_addr,
            "market_id": market_id,
            "to_account_number": args.to_account_number,
            "amount":     fmt_token_amount(amount_raw, decimals),
            "amount_raw": amount_raw.to_string(),
            "spender": chain.dolomite_margin,
            "call_target": chain.deposit_withdrawal_proxy,
            "wallet_balance":    fmt_token_amount(bal, decimals),
            "native_balance":    fmt_token_amount(native, 18),
        }
    }))?);

    if args.dry_run {
        eprintln!("[DRY RUN] Calldata fetched, balance + gas verified. Not signing.");
        return Ok(());
    }
    if !args.confirm {
        eprintln!("[PREVIEW] Add --confirm to sign and submit.");
        return Ok(());
    }

    // Approve token to DepositWithdrawalProxy (EVM-006 wait_for_tx).
    // EVM-012: surface RPC failures rather than silently re-approving on every blip.
    let allowance = match erc20_allowance(token_addr, &from_addr, chain.dolomite_margin, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read {} allowance for DolomiteMargin on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if allowance < amount_raw {
        let approve_data = build_approve_max(chain.dolomite_margin);
        eprintln!("[supply] Approving {} for DepositWithdrawalProxy…", symbol);
        let r = match wallet_contract_call(chain.id, token_addr, &approve_data, None, Some(60_000), false) {
            Ok(r) => r,
            Err(e) => return print_err(&format!("Approve failed: {:#}", e), "APPROVE_FAILED",
                "Check onchainos status."),
        };
        let h = extract_tx_hash(&r).ok_or_else(|| anyhow::anyhow!("approve tx hash missing"))?;
        eprintln!("[supply] Approve tx: {} — waiting…", h);
        if let Err(e) = wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await {
            return print_err(&format!("Approve confirm timeout: {:#}", e), "APPROVE_NOT_CONFIRMED",
                "Bump --approve-timeout-secs or check explorer.");
        }
        eprintln!("[supply] Approve confirmed.");
    } else {
        eprintln!("[supply] Existing allowance >= required; skipping approve.");
    }

    // Submit deposit (EVM-014 retry on allowance lag, EVM-015 explicit gas-limit)
    let result = match wallet_contract_call(chain.id, chain.deposit_withdrawal_proxy, &calldata, None, Some(400_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            let allowance_lag = emsg.contains("transfer amount exceeds allowance")
                || emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance");
            if allowance_lag {
                eprintln!("[supply] EVM-014 allowance-lag retry, sleeping 5s…");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, chain.deposit_withdrawal_proxy, &calldata, None, Some(400_000), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(&format!("Deposit submission failed: {:#}", emsg), "SUPPLY_SUBMIT_FAILED",
                    "Inspect onchainos output. Common: insufficient gas, RPC issue.");
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    // TX-001: confirm on-chain status
    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[supply] Submit tx: {} — waiting for on-chain confirmation…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(
                    &format!("Tx {} broadcast but reverted: {:#}", h, e),
                    "TX_REVERTED",
                    "On-chain revert. Inspect on Arbiscan.",
                );
            }
            eprintln!("[supply] On-chain confirmed (status 0x1).");
        }
        None => return print_err(
            "Supply broadcast but onchainos did not return a tx hash",
            "TX_HASH_MISSING",
            "Check `onchainos wallet history` for the tx.",
        ),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "supply",
        "chain": chain.key,
        "token": symbol,
        "amount":     fmt_token_amount(amount_raw, decimals),
        "amount_raw": amount_raw.to_string(),
        "to_account_number": args.to_account_number,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `dolomite-plugin positions` to see your accruing supply position.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
