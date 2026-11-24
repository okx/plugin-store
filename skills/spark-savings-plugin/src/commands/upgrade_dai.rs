use clap::Args;
use serde_json::json;

use crate::config::{chain_by_id, STABLE_DECIMALS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, fmt_token_amount, human_to_atomic,
    native_balance, pad_address, pad_u256, selectors, wait_for_tx,
};

const ETHEREUM_CHAIN_ID: u64 = 1;

#[derive(Args)]
pub struct UpgradeDaiArgs {
    /// Human-readable DAI amount to upgrade (e.g. 10 = 10 DAI). Use --all to upgrade entire balance.
    #[arg(long, allow_hyphen_values = true, conflicts_with = "all")]
    pub amount: Option<String>,
    /// Upgrade entire DAI balance
    #[arg(long)]
    pub all: bool,
    /// Override receiver of resulting USDS (default: caller's wallet)
    #[arg(long)]
    pub receiver: Option<String>,
    /// Dry run — no signing, no submission
    #[arg(long)]
    pub dry_run: bool,
    /// Required to actually submit
    #[arg(long)]
    pub confirm: bool,
    /// Approve confirmation timeout (seconds, default 180)
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: UpgradeDaiArgs) -> anyhow::Result<()> {
    // upgrade-dai is Ethereum-only — there is no DaiUsds migrator on L2.
    let chain = chain_by_id(ETHEREUM_CHAIN_ID).expect("Ethereum must be in SUPPORTED_CHAINS");
    let dai = chain.dai.expect("Ethereum DAI address missing in config");
    let migrator = chain.dai_usds_migrator.expect("Ethereum DaiUsds migrator address missing");

    if args.amount.is_none() && !args.all {
        return print_err(
            "Must specify --amount or --all",
            "INVALID_ARGUMENT",
            "Use --amount 10 or --all to upgrade entire DAI balance.",
        );
    }

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(
            &format!("Could not resolve wallet on Ethereum: {:#}", e),
            "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses` to verify login.",
        ),
    };
    let receiver = args.receiver.clone().unwrap_or_else(|| from_addr.clone());

    // Read DAI balance + native gas in parallel
    let dai_fut = erc20_balance(dai, &from_addr, chain.rpc);
    let native_fut = native_balance(&from_addr, chain.rpc);
    let (dai_bal_res, native_res) = tokio::join!(dai_fut, native_fut);

    let dai_bal = match dai_bal_res {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read DAI balance: {:#}", e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    // EVM-012: surface RPC error explicitly, don't unwrap_or(0) — that turns a
    // transient publicnode hiccup into a confusing INSUFFICIENT_GAS message.
    let native_bal = match native_res {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read native ETH balance on Ethereum: {:#}", e),
            "RPC_ERROR",
            "Public RPC may be limited; retry in a few seconds. (Your wallet's actual balance is unaffected.)",
        ),
    };
    let native_floor: u128 = 1_000_000_000_000_000; // ~$2.30 — L1 needs more gas
    if native_bal < native_floor {
        return print_err(
            &format!(
                "Native ETH on Ethereum is {} (~${}), below ~$2 floor needed for an L1 approve+upgrade pair",
                fmt_token_amount(native_bal, 18),
                native_bal as f64 / 1e18 * 2300.0,
            ),
            "INSUFFICIENT_GAS",
            "Top up ETH on Ethereum mainnet.",
        );
    }

    // Determine amount
    let amount_atomic: u128 = if args.all {
        dai_bal
    } else {
        match human_to_atomic(args.amount.as_ref().unwrap(), STABLE_DECIMALS) {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Invalid --amount: {}", e),
                "INVALID_ARGUMENT",
                "Pass a positive number, e.g. --amount 10",
            ),
        }
    };

    if dai_bal < amount_atomic {
        return print_err(
            &format!(
                "Insufficient DAI: need {} (raw {}), have {} (raw {})",
                fmt_token_amount(amount_atomic, STABLE_DECIMALS), amount_atomic,
                fmt_token_amount(dai_bal, STABLE_DECIMALS), dai_bal,
            ),
            "INSUFFICIENT_BALANCE",
            "Reduce --amount, or top up DAI.",
        );
    }
    if amount_atomic == 0 {
        return print_err(
            "DAI balance is 0 (or --amount resolved to 0)",
            "NO_DAI",
            "Use `spark-savings-plugin balance --chain ETH` to confirm holdings.",
        );
    }

    // daiToUsds(address usr, uint256 wad) — selector verified by keccak256.
    // Contract: https://etherscan.io/address/0x3225737a9Bbb6473CB4a45b7244ACa2BeFdB276A
    let calldata = format!(
        "{}{}{}",
        selectors::DAI_TO_USDS,
        pad_address(&receiver),
        pad_u256(amount_atomic),
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "upgrade-dai",
            "chain": chain.key,
            "chain_id": chain.id,
            "from": from_addr,
            "receiver": receiver,
            "amount_dai":  fmt_token_amount(amount_atomic, STABLE_DECIMALS),
            "amount_dai_raw": amount_atomic.to_string(),
            "expected_usds": fmt_token_amount(amount_atomic, STABLE_DECIMALS),
            "expected_usds_raw": amount_atomic.to_string(),
            "rate": "1:1, no slippage, no fees",
            "migrator": migrator,
            "dai_balance": fmt_token_amount(dai_bal, STABLE_DECIMALS),
            "native_balance": fmt_token_amount(native_bal, 18),
        }
    }))?);

    if args.dry_run {
        eprintln!("[DRY RUN] Calldata prepared, balance + gas verified. Not signing.");
        return Ok(());
    }
    if !args.confirm {
        eprintln!("[PREVIEW] Add --confirm to sign and submit the upgrade.");
        return Ok(());
    }

    // Approve DAI to migrator (the migrator needs to pull DAI from us).
    // EVM-012: surface RPC failures rather than silently re-approving.
    let allowance = match erc20_allowance(dai, &from_addr, migrator, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read DAI allowance for migrator {} on {}: {:#}", migrator, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if allowance < amount_atomic {
        let approve_data = build_approve_max(migrator);
        eprintln!("[upgrade-dai] Approving DAI for migrator ({})…", migrator);
        let result = wallet_contract_call(chain.id, dai, &approve_data, None, Some(60_000), false)
            .map_err(|e| anyhow::anyhow!("approve failed: {:#}", e))?;
        let h = extract_tx_hash(&result)
            .ok_or_else(|| anyhow::anyhow!("approve tx hash missing"))?;
        eprintln!("[upgrade-dai] Approve tx: {} — waiting…", h);
        wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await
            .map_err(|e| anyhow::anyhow!("approve confirm timeout: {:#}", e))?;
        eprintln!("[upgrade-dai] Approve confirmed.");
    }

    // Submit migrator call (EVM-014 retry-on-allowance-revert).
    // DAI uses a custom revert format "Dai/insufficient-allowance" — different
    // from the standard ERC-20 "transfer amount exceeds allowance". Match both.
    let result = match wallet_contract_call(chain.id, migrator, &calldata, None, Some(250_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            let is_allowance_lag = emsg.contains("exceeds allowance")
                || emsg.contains("Dai/insufficient-allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance");
            if is_allowance_lag {
                eprintln!("[upgrade-dai] EVM-014 allowance-lag retry, sleeping 5s…");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, migrator, &calldata, None, Some(250_000), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(
                    &format!("Upgrade submission failed: {:#}", emsg),
                    "UPGRADE_SUBMIT_FAILED",
                    "Inspect onchainos output. Common causes: insufficient gas, RPC issue.",
                );
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    // TX-001: confirm on-chain status before reporting success.
    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[upgrade-dai] Submit tx: {} — waiting for on-chain confirmation…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(
                    &format!("Tx {} broadcast but on-chain execution failed: {:#}", h, e),
                    "TX_REVERTED",
                    "On-chain revert. Inspect on Etherscan. No funds moved beyond gas.",
                );
            }
            eprintln!("[upgrade-dai] On-chain confirmed (status 0x1).");
        }
        None => return print_err(
            "Upgrade broadcast but onchainos did not return a tx hash",
            "TX_HASH_MISSING",
            "Cannot verify on-chain status. Check `onchainos wallet history`.",
        ),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "upgrade-dai",
        "chain": chain.key,
        "amount_dai": fmt_token_amount(amount_atomic, STABLE_DECIMALS),
        "amount_dai_raw": amount_atomic.to_string(),
        "amount_usds": fmt_token_amount(amount_atomic, STABLE_DECIMALS),
        "amount_usds_raw": amount_atomic.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "USDS now in your wallet on Ethereum. Use `spark-savings-plugin deposit --chain ETH --amount <X> --confirm` to start earning SSR.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
