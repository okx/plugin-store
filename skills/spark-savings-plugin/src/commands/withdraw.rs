use clap::Args;
use serde_json::json;

use crate::config::{ConvertMechanism, parse_chain, supported_chains_help, STABLE_DECIMALS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, fmt_token_amount, human_to_atomic,
    native_balance, pad_address, pad_u256, selectors, vault_preview_redeem, wait_for_tx,
};

#[derive(Args)]
pub struct WithdrawArgs {
    /// Source chain (id or key)
    #[arg(long)]
    pub chain: String,
    /// Human-readable USDS amount to withdraw (e.g. 5 = 5 USDS).
    /// Mutually exclusive with --shares-amount.
    #[arg(long, allow_hyphen_values = true, conflicts_with = "shares_amount")]
    pub amount: Option<String>,
    /// Alternative: redeem an exact number of sUSDS shares
    #[arg(long, allow_hyphen_values = true)]
    pub shares_amount: Option<String>,
    /// Redeem all sUSDS — mutually exclusive with --amount and --shares-amount
    #[arg(long, conflicts_with_all = ["amount", "shares_amount"])]
    pub all: bool,
    /// Slippage percent for Spark PSM path (default 0.5)
    #[arg(long, default_value = "0.5")]
    pub slippage_pct: f64,
    /// Override receiver — defaults to the caller's wallet
    #[arg(long)]
    pub receiver: Option<String>,
    /// Dry run — fetch state, prepare calldata, do not sign
    #[arg(long)]
    pub dry_run: bool,
    /// Required to actually submit
    #[arg(long)]
    pub confirm: bool,
    /// Approve confirmation timeout in seconds (default 180)
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
    let chain = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unsupported chain '{}'", args.chain),
            "UNSUPPORTED_CHAIN",
            &format!("Use one of: {}", supported_chains_help()),
        ),
    };

    if args.amount.is_none() && args.shares_amount.is_none() && !args.all {
        return print_err(
            "Must specify exactly one of: --amount, --shares-amount, or --all",
            "INVALID_ARGUMENT",
            "Use --amount 5 (USDS), --shares-amount 5 (sUSDS), or --all to redeem everything.",
        );
    }

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(
            &format!("Could not resolve wallet on chain {}: {:#}", chain.id, e),
            "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses` to verify login.",
        ),
    };
    let receiver = args.receiver.clone().unwrap_or_else(|| from_addr.clone());

    let susds_bal = match erc20_balance(chain.susds, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read sUSDS balance: {:#}", e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if susds_bal == 0 {
        return print_err(
            &format!("No sUSDS to withdraw on {}", chain.key),
            "NO_SUSDS",
            "Use `spark-savings-plugin balance` to find which chain has your sUSDS.",
        );
    }

    // Determine shares to redeem
    let shares_to_redeem: u128 = if args.all {
        susds_bal
    } else if let Some(s) = &args.shares_amount {
        match human_to_atomic(s, STABLE_DECIMALS) {
            Ok(v) => v.min(susds_bal),
            Err(e) => return print_err(
                &format!("Invalid --shares-amount '{}': {}", s, e),
                "INVALID_ARGUMENT",
                "Pass a positive number, e.g. --shares-amount 5",
            ),
        }
    } else {
        // --amount given: convert USDS amount → shares via previewDeposit's inverse.
        // Spark sUSDS share price ≈ 1 USDS rounding up over time, so for v0.1.0
        // we assume 1:1 (slight under-redemption is safer than over-redemption).
        let usds_amt = match human_to_atomic(args.amount.as_ref().unwrap(), STABLE_DECIMALS) {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Invalid --amount: {}", e),
                "INVALID_ARGUMENT",
                "Pass a positive number, e.g. --amount 5",
            ),
        };
        usds_amt.min(susds_bal)
    };

    if shares_to_redeem == 0 {
        return print_err(
            "Computed redemption shares = 0",
            "INVALID_ARGUMENT",
            "Reduce or change --amount.",
        );
    }

    // Preview USDS received (read-only)
    let preview_assets = vault_preview_redeem(chain.susds, shares_to_redeem, chain.rpc).await
        .unwrap_or(shares_to_redeem); // fallback ~1:1 if vault preview fails (L2 case)

    // Native gas check (EVM-012: surface RPC error, don't unwrap_or(0))
    let native_bal = match native_balance(&from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read native gas balance: {:#}", e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    let native_floor: u128 = 500_000_000_000_000;
    if native_bal < native_floor {
        return print_err(
            &format!("Native gas on {} below ~$1 floor", chain.key),
            "INSUFFICIENT_GAS",
            "Top up native gas on this chain.",
        );
    }

    // Build calldata + spender by mechanism
    let (spender, calldata, mechanism_label, needs_susds_approve) = match chain.mechanism {
        ConvertMechanism::Erc4626Vault => {
            // ERC-4626 redeem(shares, receiver, owner) — selector 0xba087652
            let calldata = format!(
                "0xba087652{}{}{}",
                pad_u256(shares_to_redeem),
                pad_address(&receiver),
                pad_address(&from_addr),
            );
            (chain.susds.to_string(), calldata, "ERC-4626 vault redeem", false)
        }
        ConvertMechanism::SparkPsm => {
            let psm = chain.spark_psm.expect("SparkPsm without psm address — config bug");
            // PSM swapExactIn(assetIn=sUSDS, assetOut=USDS, amountIn=shares, minAmountOut, receiver, referralCode)
            let slip = (preview_assets as f64 * (1.0 - args.slippage_pct / 100.0)) as u128;
            let calldata = format!(
                "{}{}{}{}{}{}{}",
                selectors::PSM_SWAP_EXACT_IN,
                pad_address(chain.susds),
                pad_address(chain.usds),
                pad_u256(shares_to_redeem),
                pad_u256(slip),
                pad_address(&receiver),
                pad_u256(0),
            );
            (psm.to_string(), calldata, "Spark PSM swapExactIn", true)
        }
    };

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "chain": chain.key,
            "chain_id": chain.id,
            "mechanism": mechanism_label,
            "from": from_addr,
            "receiver": receiver,
            "shares_to_redeem": fmt_token_amount(shares_to_redeem, STABLE_DECIMALS),
            "shares_to_redeem_raw": shares_to_redeem.to_string(),
            "expected_usds": fmt_token_amount(preview_assets, STABLE_DECIMALS),
            "expected_usds_raw": preview_assets.to_string(),
            "spender": spender,
            "needs_susds_approve": needs_susds_approve,
            "susds_balance": fmt_token_amount(susds_bal, STABLE_DECIMALS),
            "slippage_pct_l2_only": if matches!(chain.mechanism, ConvertMechanism::SparkPsm) { Some(args.slippage_pct) } else { None },
        }
    }))?);

    if args.dry_run {
        eprintln!("[DRY RUN] Calldata fetched, balance verified. Not signing.");
        return Ok(());
    }
    if !args.confirm {
        eprintln!("[PREVIEW] Add --confirm to sign and submit the redemption.");
        return Ok(());
    }

    // PSM path needs sUSDS approval to PSM contract; ERC-4626 redeem does not.
    // EVM-012: surface RPC failures rather than silently re-approving.
    if needs_susds_approve {
        let allowance = match erc20_allowance(chain.susds, &from_addr, &spender, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read sUSDS allowance for PSM {} on {}: {:#}", spender, chain.key, e),
                "RPC_ERROR",
                "Public RPC may be limited; retry shortly.",
            ),
        };
        if allowance < shares_to_redeem {
            let approve_data = build_approve_max(&spender);
            eprintln!("[withdraw] Approving sUSDS for PSM ({})…", spender);
            let result = wallet_contract_call(chain.id, chain.susds, &approve_data, None, Some(60_000), false)
                .map_err(|e| anyhow::anyhow!("approve failed: {:#}", e))?;
            let h = extract_tx_hash(&result)
                .ok_or_else(|| anyhow::anyhow!("approve tx hash missing"))?;
            eprintln!("[withdraw] Approve tx: {} — waiting…", h);
            wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await
                .map_err(|e| anyhow::anyhow!("approve confirm timeout: {:#}", e))?;
            eprintln!("[withdraw] Approve confirmed.");
        }
    }

    // Submit redeem (EVM-014 retry-on-allowance-revert)
    let result = match wallet_contract_call(chain.id, &spender, &calldata, None, Some(250_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            // EVM-014: also match DAI / ERC20-compatible custom formats
            let is_allowance_lag = emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance");
            if is_allowance_lag && needs_susds_approve {
                eprintln!("[withdraw] EVM-014 allowance-lag retry, sleeping 5s…");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, &spender, &calldata, None, Some(250_000), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(
                    &format!("Redeem submission failed: {:#}", emsg),
                    "WITHDRAW_SUBMIT_FAILED",
                    "Inspect onchainos output. Possible: low gas, slippage tightened (PSM only).",
                );
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    // TX-001: confirm on-chain status (broadcast ≠ success — see deposit.rs).
    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[withdraw] Submit tx: {} — waiting for on-chain confirmation…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(
                    &format!("Tx {} broadcast but on-chain execution failed: {:#}", h, e),
                    "TX_REVERTED",
                    "On-chain revert. Common causes for redeem: gas limit too low (250k explicit override is set), share-conversion rounding, or PSM slippage tightened (L2 only). Inspect on the block explorer.",
                );
            }
            eprintln!("[withdraw] On-chain confirmed (status 0x1).");
        }
        None => return print_err(
            "Withdraw broadcast but onchainos did not return a tx hash",
            "TX_HASH_MISSING",
            "Cannot verify on-chain status. Check `onchainos wallet history` to locate the tx.",
        ),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "withdraw",
        "chain": chain.key,
        "mechanism": mechanism_label,
        "shares_redeemed": fmt_token_amount(shares_to_redeem, STABLE_DECIMALS),
        "shares_redeemed_raw": shares_to_redeem.to_string(),
        "expected_usds": fmt_token_amount(preview_assets, STABLE_DECIMALS),
        "expected_usds_raw": preview_assets.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `spark-savings-plugin balance --chain <X>` to confirm USDS arrived.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
