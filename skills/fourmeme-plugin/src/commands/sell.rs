/// `fourmeme-plugin sell --token 0x... --amount <tokens>` — burn tokens for BNB.
///
/// Flow:
///   1. trySell preview → confirms `tokenManager` (V1/V2) and BNB-out estimate
///   2. ERC-20 approve `amount` to the resolved tokenManager (force-skipped if
///      allowance already covers it)
///   3. GAS-001 BNB-for-gas pre-check
///   4. Send `sellToken(token, amount)` to the resolved manager
///   5. Wait for receipt via direct RPC

use anyhow::{Context, Result};
use clap::Args;

use crate::config::{chain_name, is_supported_chain, TOKEN_DECIMALS};
use crate::rpc::{eth_call, eth_get_balance_wei, estimate_native_gas_cost_wei, wei_to_bnb};

const GAS_LIMIT_SELL: u64 = 250_000;

#[derive(Args)]
pub struct SellArgs {
    #[arg(long)]
    pub token: String,

    /// Whole tokens to sell (e.g. "1000000")
    #[arg(long)]
    pub amount: Option<String>,

    /// Sell entire wallet balance
    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Pass --confirm to actually submit the on-chain tx. Default is preview-only
    /// (prints the planned tx without spending gas) so accidental invocation is safe.
    #[arg(long, default_value_t = false)]
    pub confirm: bool,
}

pub async fn run(args: SellArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("sell"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: SellArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    if args.all && args.amount.is_some() {
        anyhow::bail!("Pass either --amount or --all, not both.");
    }
    let token = args.token.to_lowercase();

    let info = super::fetch_token_info(args.chain, &token).await?;
    if info.liquidity_added {
        anyhow::bail!("Token has graduated to PancakeSwap. Sell via pancakeswap plugin.");
    }
    let sym = super::erc20_symbol(args.chain, &token).await;
    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    let bal = super::erc20_balance(args.chain, &token, &wallet).await?;
    if bal == 0 {
        anyhow::bail!("Wallet holds 0 {}. Nothing to sell.", sym);
    }

    let amount_raw = if args.all {
        bal
    } else {
        let s = args.amount.as_ref()
            .ok_or_else(|| anyhow::anyhow!("--amount or --all is required"))?;
        let parsed = super::parse_human_amount(s, TOKEN_DECIMALS)?;
        if parsed > bal {
            anyhow::bail!(
                "Sell amount {} > wallet balance {}. Use --all to sell everything.",
                super::fmt_decimal(parsed, TOKEN_DECIMALS),
                super::fmt_decimal(bal, TOKEN_DECIMALS)
            );
        }
        parsed
    };

    let q = super::fetch_try_sell(args.chain, &token, amount_raw).await
        .context("trySell preview failed")?;

    if !args.confirm {
        let resp = serde_json::json!({
            "ok": true,
            "preview_only": true,
            "data": {
                "action": "sell",
                "chain": chain_name(args.chain),
                "chain_id": args.chain,
                "wallet": wallet,
                "token": token,
                "symbol": sym,
                "token_manager": q.token_manager,
                "input": {
                    "amount":     super::fmt_decimal(amount_raw, TOKEN_DECIMALS),
                    "amount_raw": amount_raw.to_string(),
                },
                "preview": {
                    "estimated_funds_bnb": format!("{:.8}", q.funds as f64 / 1e18),
                    "estimated_fee_bnb":   format!("{:.8}", q.fee as f64 / 1e18),
                },
                "tx_plan": [
                    format!("token.approve({}, {}) — pre-tx, --force", q.token_manager, amount_raw),
                    format!("TokenManager.sellToken({}, {})", token, amount_raw),
                ],
                "note": "preview only (--confirm omitted): no transactions submitted.",
            }
        });
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    // Pre-flight gas (BNB) — sell only spends gas, no msg.value
    let need_gas = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_SELL).await?
        .saturating_mul(2); // approve + sell
    let have = eth_get_balance_wei(args.chain, &wallet).await?;
    if have < need_gas {
        anyhow::bail!(
            "Insufficient BNB for gas: have {:.6}, need ~{:.6} for approve + sell.",
            wei_to_bnb(have), wei_to_bnb(need_gas),
        );
    }

    // Step 1: Approve (only if existing allowance is short)
    let allowance_data = format!(
        "0x{}{}{}",
        crate::calldata::SEL_ALLOWANCE,
        crate::rpc::pad_address(&wallet),
        crate::rpc::pad_address(&q.token_manager),
    );
    let allowance_hex = eth_call(args.chain, &token, &allowance_data).await?;
    let current_allowance = crate::rpc::parse_uint256_to_u128(&allowance_hex);
    if current_allowance < amount_raw {
        eprintln!("[fourmeme] approving {} to {}...", sym, q.token_manager);
        let approve_data = crate::calldata::build_approve_max(&q.token_manager);
        let approve_resp = crate::onchainos::wallet_contract_call(
            args.chain, &token, &approve_data,
            Some(&wallet), None, true,
        ).await?;
        let approve_hash = crate::onchainos::extract_tx_hash(&approve_resp)?;
        eprintln!("[fourmeme] approve tx: {} (waiting...)", approve_hash);
        crate::onchainos::wait_for_tx_receipt(&approve_hash, args.chain, 120).await?;
    } else {
        eprintln!("[fourmeme] allowance already covers {} — skipping approve.", sym);
    }

    // Step 2: Sell
    let sell_data = crate::calldata::build_sell_token(&token, amount_raw);
    eprintln!("[fourmeme] selling {} {}...", super::fmt_decimal(amount_raw, TOKEN_DECIMALS), sym);
    let sell_resp = crate::onchainos::wallet_contract_call(
        args.chain, &q.token_manager, &sell_data,
        Some(&wallet), None, false,
    ).await?;
    let sell_hash = crate::onchainos::extract_tx_hash(&sell_resp)?;
    eprintln!("[fourmeme] sell tx: {} (waiting...)", sell_hash);
    crate::onchainos::wait_for_tx_receipt(&sell_hash, args.chain, 120).await?;

    // EVM-012: post-tx balance reads are display-only (the sell already
    // confirmed). Keep the soft fallback but expose query errors so the
    // displayed deltas can be marked as best-effort when RPC blips.
    let (post_bal, post_bal_query_error) = match super::erc20_balance(args.chain, &token, &wallet).await {
        Ok(v) => (v, None::<String>),
        Err(e) => (0u128, Some(format!("{:#}", e))),
    };
    let (post_bnb, post_bnb_query_error) = match eth_get_balance_wei(args.chain, &wallet).await {
        Ok(v) => (v, None::<String>),
        Err(e) => (0u128, Some(format!("{:#}", e))),
    };

    let out = serde_json::json!({
        "ok": true,
        "data": {
            "action": "sell",
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "wallet": wallet,
            "token": token,
            "symbol": sym,
            "token_manager": q.token_manager,
            "sold":     super::fmt_decimal(amount_raw, TOKEN_DECIMALS),
            "sold_raw": amount_raw.to_string(),
            "preview_funds_bnb": format!("{:.8}", q.funds as f64 / 1e18),
            "post_trade_token_balance":     super::fmt_decimal(post_bal, TOKEN_DECIMALS),
            "post_trade_token_balance_raw": post_bal.to_string(),
            "post_trade_token_balance_query_error": post_bal_query_error,
            "post_trade_bnb_balance": format!("{:.6}", wei_to_bnb(post_bnb)),
            "post_trade_bnb_balance_query_error":   post_bnb_query_error,
            "sell_tx": sell_hash,
            "on_chain_status": "0x1",
        }
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
