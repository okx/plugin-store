use clap::Args;
use serde_json::json;

use crate::config::{parse_chain, supported_chains_help, ChainInfo, RateMode, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    erc20_decimals, erc20_symbol, fmt_1e18, fmt_token_amount, get_reserves_list,
    get_user_account_data, human_to_atomic, lp_get_reserve_data, native_balance,
    pad_address, pad_u256, selectors, wait_for_tx,
};

/// Borrow underlying token from Aave V2 LendingPool. Requires existing collateral
/// (i.e. previously supplied + enabled-as-collateral assets) such that
/// availableBorrowsETH > requested_amount_in_eth_equivalent.
///
/// `--rate-mode 1` = stable (V2 only; V3 removed); `--rate-mode 2` = variable.
/// Stable rate is fixed at borrow time but can be rebalanced by anyone if it drifts
/// significantly from market. Variable floats with utilization curve.
///
/// All operations require explicit `--confirm`. v0.1.0 ERC-20 only.
#[derive(Args)]
pub struct BorrowArgs {
    /// Chain key or id (ETH / POLYGON / AVAX).
    #[arg(long, default_value = "ETH")]
    pub chain: String,
    /// Token to borrow (case-insensitive symbol or 0x address).
    #[arg(long)]
    pub token: String,
    /// Underlying amount to borrow (human-readable).
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Interest rate mode: 1=stable, 2=variable. Default 2 (variable; recommended).
    #[arg(long, default_value = "2")]
    pub rate_mode: u8,
    /// Aave referral code (default 0)
    #[arg(long, default_value = "0")]
    pub referral_code: u16,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

/// HF threshold below which we refuse to borrow even at preview (1.10x safe margin).
const HF_BORROW_FLOOR: u128 = 1_100_000_000_000_000_000; // 1.10e18

pub async fn run(args: BorrowArgs) -> anyhow::Result<()> {
    let chain: &ChainInfo = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unknown --chain '{}'", args.chain),
            "INVALID_CHAIN",
            &format!("Supported: {}", supported_chains_help()),
        ),
    };

    let mode = match RateMode::from_u8(args.rate_mode) {
        Some(m) => m,
        None => return print_err(
            &format!("Invalid --rate-mode '{}': must be 1 (stable) or 2 (variable)", args.rate_mode),
            "INVALID_ARGUMENT", "Pass --rate-mode 2 for variable (recommended) or 1 for stable.",
        ),
    };

    let upper = args.token.to_uppercase();
    if upper == chain.native_symbol {
        return print_err(
            &format!("Native {} borrow deferred to v0.2.0. Borrow the wrapped W{} instead.",
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

    let amount_raw = match human_to_atomic(&args.amount, decimals) {
        Ok(v) => v,
        Err(e) => return print_err(&format!("Invalid --amount: {}", e),
            "INVALID_ARGUMENT", "Pass a positive number, e.g. --amount 100"),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Pre-flight: reserve must not be frozen / borrowing-disabled / stable-rate-disabled.
    // Frozen reserves reject borrow on-chain with VL_RESERVE_FROZEN ('3'); pre-checking
    // saves wasted gas if borrowingEnabled or stable rate isn't toggled.
    let rd = lp_get_reserve_data(chain.lending_pool, &asset_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("LendingPool.getReserveData: {}", e))?;
    let cfg = rd.decode_config();
    if !cfg.is_active {
        return print_err(
            &format!("Reserve {} on {} is inactive.", symbol, chain.key),
            "RESERVE_INACTIVE",
            "Try a different chain or use aave-v3-plugin.",
        );
    }
    if cfg.is_frozen {
        let v3_install_cmd = "npx skills add okx/plugin-store --skill aave-v3-plugin --yes --global";
        let v3_equivalent = format!(
            "aave-v3-plugin borrow --chain {} --token {} --amount {} --rate-mode {} --confirm",
            chain.key, symbol, args.amount, args.rate_mode
        );
        let payload = serde_json::json!({
            "ok": false,
            "error": format!("Reserve {} on {} is frozen by Aave governance - new borrow rejected on-chain (VL_RESERVE_FROZEN, error code 3). All Aave V2 reserves across Ethereum / Polygon / Avalanche are frozen as part of the V3 migration; existing borrows can still be repaid and rate-mode-swapped, but new borrows must use V3 (note: V3 removed stable rate, so --rate-mode 1 maps to variable on V3).",
                symbol, chain.key),
            "error_code": "RESERVE_FROZEN",
            "suggestion": format!("Install aave-v3-plugin and run the equivalent V3 borrow: `{}` then `{}`", v3_install_cmd, v3_equivalent),
            "redirect": {
                "reason": "Aave V2 wind-down: governance has frozen all reserves on this chain. V3 is the actively maintained version.",
                "install_command": v3_install_cmd,
                "equivalent_command": v3_equivalent,
                "alternative_plugin": "aave-v3-plugin",
                "rate_mode_note": "Aave V3 removed stable rate mode entirely; if you wanted stable on V2, V3 only supports variable.",
            },
        });
        println!("{}", serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!(r#"{{"ok":false,"error_code":"RESERVE_FROZEN"}}"#)));
        return Ok(());
    }
    if !cfg.borrowing_enabled {
        return print_err(
            &format!("Borrowing is disabled for reserve {} on {} (governance toggle).", symbol, chain.key),
            "BORROWING_DISABLED",
            "This reserve is supply-only on V2. Use aave-v3-plugin or another reserve.",
        );
    }
    if mode == RateMode::Stable && !cfg.stable_rate_enabled {
        return print_err(
            &format!("Stable rate is disabled for reserve {} on {}.", symbol, chain.key),
            "STABLE_RATE_DISABLED",
            "Pass --rate-mode 2 (variable) instead.",
        );
    }

    // Pre-flight: account liquidity & HF
    let (total_collateral_eth, total_debt_eth, available_borrows_eth, _liq_thresh, _ltv, hf) =
        get_user_account_data(chain.lending_pool, &from_addr, chain.rpc).await
            .map_err(|e| anyhow::anyhow!("getUserAccountData: {}", e))?;

    if total_collateral_eth == 0 {
        return print_err(
            "No collateral on Aave V2 - cannot borrow. Supply an asset first.",
            "NO_COLLATERAL",
            "Run `aave-v2-plugin supply --chain X --token Y --amount Z --confirm` to add collateral.",
        );
    }

    if available_borrows_eth == 0 {
        return print_err(
            &format!("Account is at borrow capacity (totalDebt={} ETH-eq, totalCollateral={} ETH-eq, HF={}).",
                fmt_1e18(total_debt_eth), fmt_1e18(total_collateral_eth), fmt_1e18(hf)),
            "NO_BORROW_CAPACITY",
            "Repay existing debt or add more collateral.",
        );
    }

    // Note: we can't directly translate amount_raw to ETH-equivalent without an oracle call.
    // Aave's borrow() will revert at execution if undercollateralized. We just warn at HF level.
    if hf > 0 && hf < HF_BORROW_FLOOR && total_debt_eth > 0 {
        return print_err(
            &format!("Health Factor {:.4} is below safe borrow threshold (1.10). Borrowing more would risk imminent liquidation.",
                hf as f64 / 1e18),
            "UNHEALTHY_HF",
            "Repay existing debt or add more collateral first.",
        );
    }

    // Pre-flight: native gas
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < chain.gas_floor_wei {
        return print_err(
            &format!("Native {} insufficient on {}", chain.native_symbol, chain.key),
            "INSUFFICIENT_GAS", "Top up native gas.",
        );
    }

    // Build calldata: borrow(asset, amount, rateMode, referralCode, onBehalfOf)
    let calldata = format!("{}{}{}{}{}{}",
        selectors::BORROW,
        pad_address(&asset_addr),
        pad_u256(amount_raw),
        pad_u256(mode.as_u128()),
        pad_u256(args.referral_code as u128),
        pad_address(&from_addr),
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "borrow",
            "chain": chain.key,
            "from": from_addr,
            "asset": asset_addr,
            "symbol": symbol,
            "amount":     fmt_token_amount(amount_raw, decimals),
            "amount_raw": amount_raw.to_string(),
            "rate_mode": args.rate_mode,
            "rate_mode_label": if args.rate_mode == 1 { "stable" } else { "variable" },
            "current_health_factor": fmt_1e18(hf),
            "available_borrows_eth_1e18": fmt_1e18(available_borrows_eth),
            "call_target": chain.lending_pool,
            "warning": "Aave checks collateralization at execution. If oracle price drops between preview and submit, borrow may revert. Variable rate (mode 2) is recommended; stable rate (mode 1) can be rebalanced by anyone if it drifts.",
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    let result = match wallet_contract_call(chain.id, chain.lending_pool, &calldata, None, Some(450_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("borrow failed: {:#}", e),
            "BORROW_SUBMIT_FAILED",
            "Common: undercollateralization at execution time, stable rate disabled for this reserve, frozen reserve, gas, RPC.",
        ),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[borrow] Submit tx: {} - waiting...", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED",
                    "Most common: undercollateralization at execution time. Run `positions` to inspect.");
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
        "asset": asset_addr,
        "symbol": symbol,
        "amount":     fmt_token_amount(amount_raw, decimals),
        "amount_raw": amount_raw.to_string(),
        "rate_mode": args.rate_mode,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `aave-v2-plugin positions --chain X` to verify Health Factor. Repay with `repay --token X --all --rate-mode N --confirm` (uint256.max sentinel - no dust).",
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
