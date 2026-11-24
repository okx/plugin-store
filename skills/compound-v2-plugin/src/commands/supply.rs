use clap::Args;
use serde_json::json;

use crate::config::{resolve_market, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, fmt_token_amount, human_to_atomic,
    is_mint_paused, native_balance, pad_u256, selectors, wait_for_tx,
};

#[derive(Args)]
pub struct SupplyArgs {
    /// Token symbol (DAI / USDC / USDT / ETH / WBTC / COMP) or 0x address (cToken or underlying)
    #[arg(long)]
    pub token: String,
    /// Human-readable amount of underlying (e.g. 100 = 100 DAI; 1 = 1 ETH)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Dry run — fetch state, prepare calldata, do not sign
    #[arg(long)]
    pub dry_run: bool,
    /// Required to actually submit
    #[arg(long)]
    pub confirm: bool,
    /// Approve confirmation timeout (default 180s — Ethereum L1 12s blocks)
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: SupplyArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let info = match resolve_market(&args.token) {
        Some(i) => i,
        None => return print_err(
            &format!("Unknown token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of DAI / USDC / USDT / ETH / WBTC / COMP, or pass cToken / underlying 0x address.",
        ),
    };

    let amount_raw = match human_to_atomic(&args.amount, info.underlying_decimals) {
        Ok(v) => v,
        Err(e) => return print_err(&format!("Invalid --amount '{}': {}", args.amount, e),
            "INVALID_ARGUMENT", "Pass a positive number, e.g. --amount 100"),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses` to verify login."),
    };

    // ---- THE BIG CHECK: V2 supply is paused — redirect to V3 ----
    let mint_paused = is_mint_paused(chain.comptroller, info.ctoken, chain.rpc).await
        .unwrap_or(true); // default to "paused" if RPC fails — safer to refuse than risk losing gas
    if mint_paused {
        return print_err(
            &format!(
                "Compound V2 {} ({}) supply is paused by governance (mintGuardianPaused=true). \
                 V2 is in wind-down mode; new supply is rejected on-chain.",
                info.ctoken_symbol_display(), info.underlying_symbol,
            ),
            "MARKET_PAUSED_USE_V3",
            "Install compound-v3-plugin for active supply: \
             `npx skills add okx/plugin-store --skill compound-v3-plugin`. \
             V3 (Comet) is the actively maintained Compound version with the same team.",
        );
    }

    // ↓ Below code is unreachable in v0.1.0 (all 6 markets paused) but kept for future
    //   if governance ever unpauses, OR if someone passes a different cToken address.

    // Pre-flight: balance check (EVM-001)
    let bal = if info.is_native {
        match native_balance(&from_addr, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(&format!("RPC: {}", e), "RPC_ERROR",
                "Public Ethereum RPC may be limited; retry shortly."),
        }
    } else {
        match erc20_balance(info.underlying, &from_addr, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(&format!("RPC: {}", e), "RPC_ERROR",
                "Public Ethereum RPC may be limited; retry shortly."),
        }
    };
    if bal < amount_raw {
        return print_err(
            &format!("Insufficient {}: need {} (raw {}), have {} (raw {}).",
                info.underlying_symbol, fmt_token_amount(amount_raw, info.underlying_decimals), amount_raw,
                fmt_token_amount(bal, info.underlying_decimals), bal),
            "INSUFFICIENT_BALANCE", "Top up the token, or reduce --amount.",
        );
    }

    // Pre-flight: native gas (GAS-001) — extra-strict on L1
    let native = match native_balance(&from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(&format!("RPC: {}", e), "RPC_ERROR",
            "Public Ethereum RPC may be limited; retry shortly."),
    };
    let gas_floor: u128 = if info.is_native { amount_raw + 5_000_000_000_000_000 } else { 5_000_000_000_000_000 };
    if native < gas_floor {
        return print_err(
            &format!("Native ETH on Ethereum is {} — supply needs ≥0.005 ETH for L1 gas{}",
                fmt_token_amount(native, 18),
                if info.is_native { format!(" PLUS the {} ETH being supplied", fmt_token_amount(amount_raw, 18)) } else { "".to_string() }),
            "INSUFFICIENT_GAS", "Top up Ethereum gas.",
        );
    }

    // Build calldata
    let (calldata, value_wei): (String, Option<u128>) = if info.is_native {
        // cETH: payable mint() (no args)
        (selectors::MINT_NATIVE.to_string(), Some(amount_raw))
    } else {
        // CErc20: mint(uint256 amount)
        (format!("{}{}", selectors::MINT_ERC20, pad_u256(amount_raw)), None)
    };

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "supply",
            "chain": chain.key,
            "from": from_addr,
            "ctoken": info.ctoken,
            "ctoken_symbol": info.symbol,
            "underlying": info.underlying,
            "underlying_symbol": info.underlying_symbol,
            "is_native": info.is_native,
            "amount":     fmt_token_amount(amount_raw, info.underlying_decimals),
            "amount_raw": amount_raw.to_string(),
            "wallet_balance":   fmt_token_amount(bal, info.underlying_decimals),
            "native_balance":   fmt_token_amount(native, 18),
            "value_wei":        value_wei.map(|v| v.to_string()),
        }
    }))?);

    if args.dry_run {
        eprintln!("[DRY RUN] Calldata built; balance + gas verified. Not signing.");
        return Ok(());
    }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to sign and submit."); return Ok(()); }

    // Approve (only for ERC-20). EVM-012: surface RPC failures rather than
    // silently re-approving on every blip (wastes gas).
    if !info.is_native {
        let allowance = match erc20_allowance(info.underlying, &from_addr, info.ctoken, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read {} allowance for cToken on {}: {:#}", info.underlying_symbol, chain.key, e),
                "RPC_ERROR",
                "Public RPC may be limited; retry shortly.",
            ),
        };
        if allowance < amount_raw {
            let approve_data = build_approve_max(info.ctoken);
            eprintln!("[supply] Approving {} for cToken contract…", info.underlying_symbol);
            let r = match wallet_contract_call(chain.id, info.underlying, &approve_data, None, Some(80_000), false) {
                Ok(r) => r,
                Err(e) => return print_err(&format!("Approve failed: {:#}", e), "APPROVE_FAILED",
                    "Inspect onchainos output."),
            };
            let h = match extract_tx_hash(&r) {
                Some(h) => h,
                None => return print_err("Approve broadcast but no tx hash",
                    "TX_HASH_MISSING", "Check `onchainos wallet history`."),
            };
            eprintln!("[supply] Approve tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Approve confirm timeout: {:#}", e),
                    "APPROVE_NOT_CONFIRMED", "Bump --approve-timeout-secs or check explorer.");
            }
            eprintln!("[supply] Approve confirmed.");
        }
    }

    // Submit mint (EVM-014 retry on allowance lag, EVM-015 explicit gas)
    let result = match wallet_contract_call(chain.id, info.ctoken, &calldata, value_wei, Some(280_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            let allowance_lag = !info.is_native && (
                emsg.contains("transfer amount exceeds allowance")
                || emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance")
            );
            if allowance_lag {
                eprintln!("[supply] EVM-014 allowance-lag retry, sleeping 5s…");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, info.ctoken, &calldata, value_wei, Some(280_000), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(&format!("Mint submission failed: {:#}", emsg), "SUPPLY_SUBMIT_FAILED",
                    "Common: market paused (try compound-v3-plugin), gas, RPC.");
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    // TX-001
    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[supply] Submit tx: {} — waiting for on-chain confirmation…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on Etherscan.");
            }
            eprintln!("[supply] On-chain confirmed (status 0x1).");
        }
        None => return print_err("Supply broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "supply",
        "chain": chain.key,
        "ctoken": info.ctoken,
        "underlying_symbol": info.underlying_symbol,
        "amount":     fmt_token_amount(amount_raw, info.underlying_decimals),
        "amount_raw": amount_raw.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `compound-v2-plugin positions` to see your accruing supply position. Note: V2 wind-down means rates are atypical and unsupported.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}

// Helper for nicer log strings
trait CTokenInfoExt {
    fn ctoken_symbol_display(&self) -> &str;
}
impl CTokenInfoExt for crate::config::CTokenInfo {
    fn ctoken_symbol_display(&self) -> &str { self.symbol }
}
