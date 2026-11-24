use clap::Args;
use serde_json::json;

use crate::config::{resolve_market, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    fmt_token_amount, get_account_liquidity, get_assets_in, human_to_atomic, is_borrow_paused,
    native_balance, pad_address, pad_u256, selectors, wait_for_tx,
};

#[derive(Args)]
pub struct BorrowArgs {
    /// Token to borrow (DAI / USDC / USDT / ETH / WBTC / COMP) or 0x address
    #[arg(long)]
    pub token: String,
    /// Underlying amount to borrow (human-readable, e.g. 100)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Skip the auto enterMarkets pre-step (only if you know your account already entered all collateral cTokens)
    #[arg(long)]
    pub skip_enter_markets: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

pub async fn run(args: BorrowArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let info = match resolve_market(&args.token) {
        Some(i) => i,
        None => return print_err(
            &format!("Unknown token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of DAI / USDC / USDT / ETH / WBTC / COMP.",
        ),
    };

    let amount_raw = match human_to_atomic(&args.amount, info.underlying_decimals) {
        Ok(v) => v,
        Err(e) => return print_err(&format!("Invalid --amount: {}", e),
            "INVALID_ARGUMENT", "Pass a positive number, e.g. --amount 100"),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Pre-flight: borrow paused check
    let bp = is_borrow_paused(chain.comptroller, info.ctoken, chain.rpc).await
        .unwrap_or(true);
    if bp {
        return print_err(
            &format!(
                "Compound V2 borrow is paused for {} ({}) by governance.",
                info.symbol, info.underlying_symbol,
            ),
            "BORROW_PAUSED_USE_V3",
            "Install compound-v3-plugin: `npx skills add okx/plugin-store --skill compound-v3-plugin`. \
             V3 (Comet) is the actively maintained Compound version.",
        );
    }

    // Pre-flight: account_liquidity must be > 0
    let (err, liquidity, shortfall) = get_account_liquidity(chain.comptroller, &from_addr, chain.rpc)
        .await.unwrap_or((1, 0, 0));
    if err != 0 {
        return print_err(
            &format!("Comptroller getAccountLiquidity error code: {}", err),
            "LIQUIDITY_QUERY_FAILED",
            "Could not read account liquidity. RPC may be degraded; retry.",
        );
    }
    if shortfall > 0 {
        return print_err(
            &format!("Account already under-collateralized — shortfall {} (1e18 USD).", fmt_token_amount(shortfall, 18)),
            "UNDERCOLLATERALIZED",
            "Repay existing debt or supply more collateral first.",
        );
    }
    if liquidity == 0 {
        return print_err(
            "Account liquidity is zero — no collateral entered. Supply collateral and enterMarkets first (or run `enter-markets` after supplying).",
            "NO_COLLATERAL",
            "Compound V2 supply is currently paused — for new positions, install compound-v3-plugin instead.",
        );
    }

    // Native gas check
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < 5_000_000_000_000_000 {
        return print_err("Native ETH below 0.005 floor", "INSUFFICIENT_GAS",
            "Top up at least 0.005 ETH on mainnet.");
    }

    // Auto enterMarkets if not in (unless --skip-enter-markets)
    let assets_in = get_assets_in(chain.comptroller, &from_addr, chain.rpc).await.unwrap_or_default();
    let already_in = assets_in.iter().any(|a| a.eq_ignore_ascii_case(info.ctoken));
    let need_enter = !already_in && !args.skip_enter_markets;

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "borrow",
            "chain": chain.key,
            "from": from_addr,
            "ctoken": info.ctoken,
            "ctoken_symbol": info.symbol,
            "underlying_symbol": info.underlying_symbol,
            "amount":     fmt_token_amount(amount_raw, info.underlying_decimals),
            "amount_raw": amount_raw.to_string(),
            "current_liquidity_usd_1e18": fmt_token_amount(liquidity, 18),
            "needs_enter_markets": need_enter,
            "step1_target": if need_enter { Some(chain.comptroller.to_string()) } else { None },
            "step2_target": info.ctoken,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN] Calldata built; not signing."); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    // Step 1: enterMarkets if needed
    if need_enter {
        // enterMarkets(address[]) — encode array of one cToken
        // ABI: selector + offset(0x20) + length(1) + ctoken (padded)
        let enter_calldata = format!(
            "{}{}{}{}",
            selectors::ENTER_MARKETS,
            pad_u256(32),                         // offset to dynamic array
            pad_u256(1),                          // array length
            pad_address(info.ctoken),
        );
        eprintln!("[borrow] Step 1: enterMarkets([{}])…", info.symbol);
        let r = match wallet_contract_call(chain.id, chain.comptroller, &enter_calldata, None, Some(150_000), false) {
            Ok(r) => r,
            Err(e) => return print_err(&format!("enterMarkets failed: {:#}", e),
                "ENTER_MARKETS_FAILED", "Inspect onchainos output."),
        };
        let h = match extract_tx_hash(&r) {
            Some(h) => h,
            None => return print_err("enterMarkets broadcast but no tx hash",
                "TX_HASH_MISSING", "Check `onchainos wallet history`."),
        };
        eprintln!("[borrow] enterMarkets tx: {} — waiting…", h);
        if let Err(e) = wait_for_tx(&h, chain.rpc, args.timeout_secs).await {
            return print_err(&format!("enterMarkets tx {} reverted: {:#}", h, e),
                "ENTER_MARKETS_REVERTED", "Step 1 reverted; borrow not attempted.");
        }
        eprintln!("[borrow] enterMarkets confirmed.");
    } else if already_in {
        eprintln!("[borrow] cToken already in assets — skipping enterMarkets.");
    }

    // Step 2: cToken.borrow(amount)
    let borrow_calldata = format!("{}{}", selectors::BORROW, pad_u256(amount_raw));
    eprintln!("[borrow] Step 2: cToken.borrow({})…", fmt_token_amount(amount_raw, info.underlying_decimals));
    let result = match wallet_contract_call(chain.id, info.ctoken, &borrow_calldata, None, Some(450_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("borrow failed: {:#}", e),
            "BORROW_SUBMIT_FAILED",
            "Common: under-collateralization, market borrow paused, RPC. Compound V2 returns enum errors codes via Failure event; check Etherscan for the specific cause.",
        ),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[borrow] Submit tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Most common: under-collateralized at execution time.");
            }
            eprintln!("[borrow] On-chain confirmed.");
        }
        None => return print_err("Borrow broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "borrow",
        "chain": chain.key,
        "ctoken": info.ctoken,
        "underlying_symbol": info.underlying_symbol,
        "amount":     fmt_token_amount(amount_raw, info.underlying_decimals),
        "amount_raw": amount_raw.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `compound-v2-plugin positions` to monitor utilization. Repay with `repay --token X --all --confirm` (uint256.max sentinel — no dust).",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
