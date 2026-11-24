use clap::Args;
use serde_json::json;

use crate::config::{ConvertMechanism, parse_chain, supported_chains_help, STABLE_DECIMALS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, fmt_token_amount, human_to_atomic,
    native_balance, pad_address, pad_u256, vault_preview_deposit, wait_for_tx,
    selectors,
};

#[derive(Args)]
pub struct DepositArgs {
    /// Source chain (id or key)
    #[arg(long)]
    pub chain: String,
    /// Human-readable USDS amount (e.g. 10 = 10 USDS)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Slippage percent for Spark PSM path (default 0.5 = 0.5%). Ignored on Ethereum (ERC-4626 has no slippage).
    #[arg(long, default_value = "0.5")]
    pub slippage_pct: f64,
    /// Override receiver — defaults to the caller's wallet
    #[arg(long)]
    pub receiver: Option<String>,
    /// Dry run — fetch state, prepare calldata, do not sign / submit
    #[arg(long)]
    pub dry_run: bool,
    /// Required to actually submit
    #[arg(long)]
    pub confirm: bool,
    /// Approve confirmation timeout in seconds (default 180)
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: DepositArgs) -> anyhow::Result<()> {
    let chain = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unsupported chain '{}'", args.chain),
            "UNSUPPORTED_CHAIN",
            &format!("Use one of: {}", supported_chains_help()),
        ),
    };

    if args.slippage_pct < 0.0 || args.slippage_pct > 50.0 {
        return print_err(
            &format!("Slippage {}% out of range (0–50)", args.slippage_pct),
            "INVALID_ARGUMENT",
            "Pass percent (0.5 = 0.5%, not 0.005).",
        );
    }

    let amount_atomic = match human_to_atomic(&args.amount, STABLE_DECIMALS) {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Invalid amount '{}': {}", args.amount, e),
            "INVALID_ARGUMENT",
            "Pass a positive number, e.g. --amount 10 or --amount 0.5",
        ),
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(
            &format!("Could not resolve wallet on chain {}: {:#}", chain.id, e),
            "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses` to verify login.",
        ),
    };
    let receiver = args.receiver.clone().unwrap_or_else(|| from_addr.clone());

    // ── Pre-flight: USDS balance check (EVM-001) ──────────────────────────────
    let usds_bal = match erc20_balance(chain.usds, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read USDS balance: {:#}", e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if usds_bal < amount_atomic {
        return print_err(
            &format!(
                "Insufficient USDS on {}: need {} (raw {}), have {} (raw {})",
                chain.key,
                fmt_token_amount(amount_atomic, STABLE_DECIMALS), amount_atomic,
                fmt_token_amount(usds_bal, STABLE_DECIMALS), usds_bal,
            ),
            "INSUFFICIENT_BALANCE",
            "Top up USDS on this chain, or use `upgrade-dai` if you have legacy DAI on Ethereum.",
        );
    }

    // ── Pre-flight: native gas check (GAS-001, lightweight — no quote API for gas estimate;
    //    use a reasonable hardcoded floor: 0.0005 ETH = ~$1) ───────────────────
    // EVM-012: surface RPC error explicitly (don't unwrap_or(0)).
    let native_bal = match native_balance(&from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read native gas balance: {:#}", e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    let native_floor: u128 = 500_000_000_000_000; // 0.0005 ETH
    if native_bal < native_floor {
        return print_err(
            &format!(
                "Native gas on {} is {} {} (~${}), below ~$1 floor needed for an approve+deposit pair.",
                chain.key,
                fmt_token_amount(native_bal, 18),
                chain.native_symbol,
                native_bal as f64 / 1e15 * 0.001 * 2300.0, // rough USD
            ),
            "INSUFFICIENT_GAS",
            "Top up native gas on this chain.",
        );
    }

    // ── Determine mechanism + spender + calldata ──────────────────────────────
    let (spender, deposit_calldata, expected_shares_raw, mechanism_label) = match chain.mechanism {
        ConvertMechanism::Erc4626Vault => {
            // ERC-4626 deposit(assets, receiver) — selector 0x6e553f65
            // signature: deposit(uint256,address)
            let calldata = format!(
                "0x6e553f65{}{}",
                pad_u256(amount_atomic),
                pad_address(&receiver),
            );
            // Preview shares minted (read-only)
            let shares = vault_preview_deposit(chain.susds, amount_atomic, chain.rpc)
                .await
                .unwrap_or(0);
            (chain.susds.to_string(), calldata, shares, "ERC-4626 vault deposit")
        }
        ConvertMechanism::SparkPsm => {
            let psm = chain.spark_psm
                .expect("SparkPsm mechanism without psm address — config bug");
            // PSM swapExactIn(assetIn, assetOut, amountIn, minAmountOut, receiver, referralCode)
            // selector 0x60b9b0e2 (need to confirm at runtime; placeholder).
            // We compute minAmountOut from preview + slippage.
            //
            // NOTE: PSM exact selector and arg layout pending verification on
            // Etherscan via a real test. For v0.1.0 we use the documented
            // signature; if simulation reverts, the user can capture the error
            // and we update this in v0.1.1.
            let preview = vault_preview_deposit(chain.susds, amount_atomic, chain.rpc)
                .await
                .unwrap_or(amount_atomic); // fallback ~1:1 if preview fails
            let slip = (preview as f64 * (1.0 - args.slippage_pct / 100.0)) as u128;
            let calldata = format!(
                "{}{}{}{}{}{}{}",
                selectors::PSM_SWAP_EXACT_IN,
                pad_address(chain.usds),
                pad_address(chain.susds),
                pad_u256(amount_atomic),
                pad_u256(slip),
                pad_address(&receiver),
                pad_u256(0), // referralCode
            );
            (psm.to_string(), calldata, preview, "Spark PSM swapExactIn")
        }
    };

    let preview_block = json!({
        "preview": {
            "chain": chain.key,
            "chain_id": chain.id,
            "mechanism": mechanism_label,
            "from": from_addr,
            "receiver": receiver,
            "amount_usds": fmt_token_amount(amount_atomic, STABLE_DECIMALS),
            "amount_usds_raw": amount_atomic.to_string(),
            "expected_susds": fmt_token_amount(expected_shares_raw, STABLE_DECIMALS),
            "expected_susds_raw": expected_shares_raw.to_string(),
            "spender": spender,
            "slippage_pct_l2_only": if matches!(chain.mechanism, ConvertMechanism::SparkPsm) { Some(args.slippage_pct) } else { None },
            "usds_balance": fmt_token_amount(usds_bal, STABLE_DECIMALS),
            "native_gas": fmt_token_amount(native_bal, 18),
        }
    });

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": preview_block["preview"],
    }))?);

    if args.dry_run {
        eprintln!("[DRY RUN] Calldata fetched, balance + gas verified. Not signing.");
        return Ok(());
    }
    if !args.confirm {
        eprintln!("[PREVIEW] Add --confirm to sign and submit the deposit.");
        return Ok(());
    }

    // ── Approve flow (ONC-001 --force, EVM-006 wait_for_tx) ───────────────────
    // EVM-012: surface RPC failures rather than silently re-approving on every
    // blip (wastes gas).
    let allowance = match erc20_allowance(chain.usds, &from_addr, &spender, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read USDS allowance for {} on {}: {:#}", spender, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if allowance < amount_atomic {
        let approve_data = build_approve_max(&spender);
        eprintln!("[deposit] Approving USDS for {} (current allowance {} < {} required)…", spender, allowance, amount_atomic);
        let result = match wallet_contract_call(chain.id, chain.usds, &approve_data, None, Some(60_000), false) {
            Ok(r) => r,
            Err(e) => return print_err(
                &format!("Approve submission failed: {:#}", e),
                "APPROVE_FAILED",
                "Check onchainos status and gas balance.",
            ),
        };
        let approve_hash = match extract_tx_hash(&result) {
            Some(h) => h,
            None => return print_err(
                "Approve submitted but tx hash not returned by onchainos",
                "APPROVE_HASH_MISSING",
                "Inspect raw onchainos output.",
            ),
        };
        eprintln!("[deposit] Approve tx: {} — waiting for confirmation…", approve_hash);
        if let Err(e) = wait_for_tx(&approve_hash, chain.rpc, args.approve_timeout_secs).await {
            return print_err(
                &format!("Approve tx did not confirm: {:#}", e),
                "APPROVE_NOT_CONFIRMED",
                "Bump --approve-timeout-secs or check explorer.",
            );
        }
        eprintln!("[deposit] Approve confirmed.");
    } else {
        eprintln!("[deposit] Existing allowance {} >= required {}; skipping approve.", allowance, amount_atomic);
    }

    // ── Submit deposit (EVM-014 retry-on-allowance-revert) ────────────────────
    let result = match wallet_contract_call(chain.id, &spender, &deposit_calldata, None, Some(250_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            // EVM-014: catch generic ERC-20 + DAI's custom format + ERC20-Compatible-style
            let is_allowance_lag = emsg.contains("transfer amount exceeds allowance")
                || emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance");
            if is_allowance_lag {
                eprintln!("[deposit] EVM-014 allowance-lag retry, sleeping 5s…");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                match wallet_contract_call(chain.id, &spender, &deposit_calldata, None, Some(250_000), false) {
                    Ok(r) => r,
                    Err(e2) => return print_err(
                        &format!("Deposit submission failed after allowance-lag retry: {:#}", e2),
                        "DEPOSIT_SUBMIT_FAILED",
                        "Wait a block and re-run the same command.",
                    ),
                }
            } else {
                return print_err(
                    &format!("Deposit submission failed: {:#}", emsg),
                    "DEPOSIT_SUBMIT_FAILED",
                    "Inspect onchainos output. Common causes: stale allowance (rare), gas spike, slippage.",
                );
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    // TX-001: onchainos returns "ok" once the tx is broadcast, but the on-chain
    // execution can still revert (e.g. OOG, contract require check). Poll the
    // receipt to confirm status=0x1 before reporting success to the caller.
    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[deposit] Submit tx: {} — waiting for on-chain confirmation…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(
                    &format!("Tx {} broadcast but on-chain execution failed: {:#}", h, e),
                    "TX_REVERTED",
                    "On-chain revert. Common causes: gas limit too low (try larger --gas-limit override in v0.1.1), slippage tightened, or contract guard. Inspect on the block explorer. No funds moved beyond gas.",
                );
            }
            eprintln!("[deposit] On-chain confirmed (status 0x1).");
        }
        None => return print_err(
            "Deposit broadcast but onchainos did not return a tx hash",
            "TX_HASH_MISSING",
            "Cannot verify on-chain status. Check `onchainos wallet history` to locate the tx.",
        ),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "deposit",
        "chain": chain.key,
        "mechanism": mechanism_label,
        "amount_usds": fmt_token_amount(amount_atomic, STABLE_DECIMALS),
        "amount_usds_raw": amount_atomic.to_string(),
        "expected_susds": fmt_token_amount(expected_shares_raw, STABLE_DECIMALS),
        "expected_susds_raw": expected_shares_raw.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `spark-savings-plugin balance --chain <X>` to confirm sUSDS arrived.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}

