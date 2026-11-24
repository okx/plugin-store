use clap::Args;
use serde_json::json;

use crate::config::{parse_chain, supported_chains_help, ChainInfo, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, erc20_decimals, erc20_symbol,
    fmt_token_amount, get_reserves_list, human_to_atomic, lp_get_reserve_data,
    native_balance, pad_address, pad_u256, selectors, wait_for_tx,
};

/// Supply (deposit) underlying token to Aave V2 LendingPool. Emits aTokens 1:1 to the user.
///
/// All operations require explicit `--confirm`. v0.1.0 supports ERC-20 only - if you want
/// to supply native ETH/MATIC/AVAX, wrap it first (WETH/WMATIC/WAVAX) and supply the wrapped
/// version. WETHGateway path (native ETH directly) is deferred to v0.2.0 to avoid
/// asymmetric handling (gateway needs aWETH approval for withdraw + variableDebtToken
/// approveDelegation for borrow, which would add UX friction).
#[derive(Args)]
pub struct SupplyArgs {
    /// Chain key or id (ETH / POLYGON / AVAX).
    #[arg(long, default_value = "ETH")]
    pub chain: String,
    /// Token symbol (USDC / USDT / DAI / WETH / WBTC / AAVE / etc) — case-insensitive.
    /// Or 0x underlying address. Special: "ETH" / "MATIC" / "AVAX" route to native via WETHGateway.
    #[arg(long)]
    pub token: String,
    /// Human-readable amount (e.g. 100 = 100 USDC; 1.5 = 1.5 ETH)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Aave referral code (default 0 - no referral)
    #[arg(long, default_value = "0")]
    pub referral_code: u16,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: SupplyArgs) -> anyhow::Result<()> {
    let chain: &ChainInfo = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unknown --chain '{}'", args.chain),
            "INVALID_CHAIN",
            &format!("Supported: {}", supported_chains_help()),
        ),
    };

    // Resolve token: ERC-20 only for v0.1.0. Reject native symbol with helpful message.
    let upper = args.token.to_uppercase();
    if upper == chain.native_symbol {
        return print_err(
            &format!("Native {} supply is deferred to v0.2.0. For now, wrap to W{} via the wrapped-token contract and supply the wrapped version.",
                chain.native_symbol, chain.native_symbol),
            "NATIVE_NOT_SUPPORTED_V01",
            &format!("Use --token W{} (wrapped) instead. For ETH on mainnet that's WETH; for MATIC/AVAX similar.",
                chain.native_symbol),
        );
    }

    let (asset_addr, symbol, decimals) = if args.token.starts_with("0x") && args.token.len() == 42 {
        let dec = erc20_decimals(&args.token, chain.rpc).await
            .map_err(|e| anyhow::anyhow!("erc20 decimals on {}: {}", args.token, e))?;
        let sym = erc20_symbol(&args.token, chain.rpc).await;
        (args.token.to_lowercase(), sym, dec)
    } else {
        // Resolve symbol → address by enumerating reserves
        match resolve_symbol_to_asset(&args.token, chain).await {
            Some((a, s, d)) => (a, s, d),
            None => return print_err(
                &format!("Token '{}' not found among Aave V2 reserves on {}", args.token, chain.key),
                "TOKEN_NOT_FOUND",
                "Run `aave-v2-plugin markets --chain X` to see all listed reserves; pass a 0x address if not in default list.",
            ),
        }
    };

    let amount_raw = match human_to_atomic(&args.amount, decimals) {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Invalid --amount '{}': {}", args.amount, e),
            "INVALID_ARGUMENT", "Pass a positive number, e.g. --amount 100",
        ),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses` to verify login."),
    };

    // Pre-flight: reserve must not be frozen / inactive / borrowing-disabled at supply path.
    // Frozen markets reject deposit on-chain with VL_RESERVE_FROZEN ('3'); pre-checking
    // saves the user the wasted approve gas. All Ethereum V2 mainnet reserves are
    // currently frozen as part of the V3 migration wind-down.
    let rd = lp_get_reserve_data(chain.lending_pool, &asset_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("LendingPool.getReserveData: {}", e))?;
    let cfg = rd.decode_config();
    if !cfg.is_active {
        return print_err(
            &format!("Reserve {} on {} is inactive (governance-disabled).", symbol, chain.key),
            "RESERVE_INACTIVE",
            "Use a different chain (Polygon V2 / Avalanche V2) or aave-v3-plugin for active markets.",
        );
    }
    if cfg.is_frozen {
        // Structured redirect to aave-v3-plugin: provide install command + equivalent V3 invocation
        // so an Agent can automatically route. Aave V2 is fully wound-down across all 3 chains
        // (37/13/7 reserves all frozen as of the V3 migration).
        let v3_install_cmd = "npx skills add okx/plugin-store --skill aave-v3-plugin --yes --global";
        let v3_equivalent = format!(
            "aave-v3-plugin supply --chain {} --token {} --amount {} --confirm",
            chain.key, symbol, args.amount
        );
        let payload = serde_json::json!({
            "ok": false,
            "error": format!("Reserve {} on {} is frozen by Aave governance - new supply rejected on-chain (VL_RESERVE_FROZEN, error code 3). All Aave V2 reserves across Ethereum / Polygon / Avalanche are frozen as part of the V3 migration; redeem/repay paths still work for legacy positions but new supply must use V3.",
                symbol, chain.key),
            "error_code": "RESERVE_FROZEN",
            "suggestion": format!("Install aave-v3-plugin and run the equivalent V3 supply: `{}` then `{}`", v3_install_cmd, v3_equivalent),
            "redirect": {
                "reason": "Aave V2 wind-down: governance has frozen all reserves on this chain. V3 (Comet-style architecture) is the actively maintained version with the same Aave team.",
                "install_command": v3_install_cmd,
                "equivalent_command": v3_equivalent,
                "alternative_chains": [],
                "alternative_plugin": "aave-v3-plugin",
            },
        });
        println!("{}", serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| format!(r#"{{"ok":false,"error_code":"RESERVE_FROZEN"}}"#)));
        return Ok(());
    }

    // Pre-flight: balance (EVM-001)
    let bal = erc20_balance(&asset_addr, &from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("erc20 balance: {}", e))?;
    if bal < amount_raw {
        return print_err(
            &format!("Insufficient {}: need {} (raw {}), have {} (raw {}).",
                symbol, fmt_token_amount(amount_raw, decimals), amount_raw,
                fmt_token_amount(bal, decimals), bal),
            "INSUFFICIENT_BALANCE", "Top up the token, or reduce --amount.",
        );
    }

    // Pre-flight: native gas (GAS-001)
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    let gas_required = chain.gas_floor_wei;
    if native < gas_required {
        return print_err(
            &format!("Native {} insufficient on {}: have {}, need {} (incl. amount + gas)",
                chain.native_symbol, chain.key,
                fmt_token_amount(native, 18), fmt_token_amount(gas_required, 18)),
            "INSUFFICIENT_GAS", &format!("Top up native {}.", chain.native_symbol),
        );
    }

    // Build calldata: LendingPool.deposit(asset, amount, onBehalfOf, referralCode)
    let calldata = format!("{}{}{}{}{}",
        selectors::DEPOSIT,
        pad_address(&asset_addr),
        pad_u256(amount_raw),
        pad_address(&from_addr),
        pad_u256(args.referral_code as u128),
    );
    let call_target: &str = chain.lending_pool;
    let value_wei: Option<u128> = None;

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "supply",
            "chain": chain.key,
            "from": from_addr,
            "asset": asset_addr,
            "symbol": symbol,
            "amount":     fmt_token_amount(amount_raw, decimals),
            "amount_raw": amount_raw.to_string(),
            "wallet_balance":   fmt_token_amount(bal, decimals),
            "native_balance":   fmt_token_amount(native, 18),
            "call_target":      call_target,
            "via": "LendingPool",
        }
    }))?);

    if args.dry_run {
        eprintln!("[DRY RUN] Calldata built; balance + gas verified. Not signing.");
        return Ok(());
    }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to sign and submit."); return Ok(()); }

    // ERC-20 path: approve LendingPool first (EVM-006). EVM-012: surface RPC
    // failures rather than silently re-approving on every blip.
    {
        let allowance = match erc20_allowance(&asset_addr, &from_addr, chain.lending_pool, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read {} allowance for LendingPool on {}: {:#}", symbol, chain.key, e),
                "RPC_ERROR",
                "Public RPC may be limited; retry shortly.",
            ),
        };
        if allowance < amount_raw {
            let approve_data = build_approve_max(chain.lending_pool);
            eprintln!("[supply] Approving {} for LendingPool...", symbol);
            let r = match wallet_contract_call(chain.id, &asset_addr, &approve_data, None, Some(80_000), false) {
                Ok(r) => r,
                Err(e) => return print_err(&format!("Approve failed: {:#}", e), "APPROVE_FAILED",
                    "Inspect onchainos output."),
            };
            let h = match extract_tx_hash(&r) {
                Some(h) => h,
                None => return print_err("Approve broadcast but no tx hash",
                    "TX_HASH_MISSING", "Check `onchainos wallet history`."),
            };
            eprintln!("[supply] Approve tx: {} - waiting...", h);
            if let Err(e) = wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Approve confirm timeout: {:#}", e),
                    "APPROVE_NOT_CONFIRMED", "Bump --approve-timeout-secs or check explorer.");
            }
            eprintln!("[supply] Approve confirmed.");
        }
    }

    // Submit deposit
    let gas_limit = 350_000u64;
    let result = match wallet_contract_call(chain.id, call_target, &calldata, value_wei, Some(gas_limit), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            let allowance_lag = emsg.contains("transfer amount exceeds allowance")
                || emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance");
            if allowance_lag {
                eprintln!("[supply] EVM-014 allowance-lag retry, sleeping 5s...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, call_target, &calldata, value_wei, Some(gas_limit), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(&format!("Deposit submission failed: {:#}", emsg),
                    "SUPPLY_SUBMIT_FAILED",
                    "Inspect onchainos output. Common: gas, RPC, frozen reserve.");
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    // TX-001
    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[supply] Submit tx: {} - waiting for on-chain confirmation...", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on the block explorer.");
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
        "asset": asset_addr,
        "symbol": symbol,
        "amount":     fmt_token_amount(amount_raw, decimals),
        "amount_raw": amount_raw.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `aave-v2-plugin positions --chain X` to see your accruing supply position. The aToken balance grows automatically as interest accrues.",
    }))?);
    Ok(())
}

/// Resolve a symbol like "USDC" to (asset_address, symbol, decimals) by enumerating reserves.
/// Returns None if no reserve symbol matches case-insensitively.
async fn resolve_symbol_to_asset(token: &str, chain: &ChainInfo) -> Option<(String, String, u32)> {
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
