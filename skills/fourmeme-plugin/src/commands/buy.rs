/// `fourmeme-plugin buy --token 0x... --funds 0.01` — submit a BNB buy.
///
/// Flow:
///   1. tryBuy preview → derive `estimated_amount` and the actual `tokenManager` (V1 vs V2)
///   2. Compute slippage floor: `min_amount = estimated × (1 - slippage_bps/10000)`
///   3. GAS-001 native gas pre-check (need msg.value + ~150k × gasPrice × 1.2)
///   4. Send `buyTokenAMAP(token, fundsWei, minAmount)` with msg.value = fundsWei
///   5. Wait for receipt via direct RPC

use anyhow::{Context, Result};
use clap::Args;

use crate::config::{chain_name, is_supported_chain, TOKEN_DECIMALS};
use crate::rpc::{eth_get_balance_wei, estimate_native_gas_cost_wei, wei_to_bnb};

const GAS_LIMIT_BUY: u64 = 250_000;

#[derive(Args)]
pub struct BuyArgs {
    #[arg(long)]
    pub token: String,

    /// BNB to spend (e.g. "0.01")
    #[arg(long)]
    pub funds: String,

    /// Slippage tolerance in basis points (default 100 = 1%)
    #[arg(long, default_value_t = 100)]
    pub slippage_bps: u64,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Pass --confirm to actually submit the on-chain tx. Default is preview-only
    /// (prints the planned tx without spending gas) so accidental invocation is safe.
    #[arg(long, default_value_t = false)]
    pub confirm: bool,
}

pub async fn run(args: BuyArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("buy"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: BuyArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let token = args.token.to_lowercase();
    let funds_wei = super::parse_human_amount(&args.funds, 18)?;

    let info = super::fetch_token_info(args.chain, &token).await?;
    if info.liquidity_added {
        anyhow::bail!(
            "Token has graduated to PancakeSwap. Buy via pancakeswap plugin."
        );
    }
    if !info.is_bnb_quoted() {
        anyhow::bail!(
            "Token uses non-native quote ({}). v0.1 supports BNB-quoted tokens only.",
            info.quote
        );
    }
    let sym = super::erc20_symbol(args.chain, &token).await;

    let q = super::fetch_try_buy(args.chain, &token, 0, funds_wei).await
        .context("tryBuy preview failed")?;
    if q.estimated_amount == 0 {
        anyhow::bail!(
            "tryBuy returned 0 token output for {} BNB. Likely the curve is at its cap or this token isn't a Four.meme V2 listing.",
            args.funds
        );
    }
    let min_amount = super::apply_slippage_floor(q.estimated_amount, args.slippage_bps);

    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    if !args.confirm {
        let resp = serde_json::json!({
            "ok": true,
            "preview_only": true,
            "data": {
                "action": "buy (BNB-quoted)",
                "chain": chain_name(args.chain),
                "chain_id": args.chain,
                "wallet": wallet,
                "token": token,
                "symbol": sym,
                "token_manager": q.token_manager,
                "input": {
                    "funds_bnb": args.funds, "funds_wei": funds_wei.to_string(),
                    "slippage_bps": args.slippage_bps,
                },
                "preview": {
                    "estimated_amount":     super::fmt_decimal(q.estimated_amount, TOKEN_DECIMALS),
                    "estimated_amount_raw": q.estimated_amount.to_string(),
                    "min_amount":     super::fmt_decimal(min_amount, TOKEN_DECIMALS),
                    "min_amount_raw": min_amount.to_string(),
                    "estimated_cost_bnb": format!("{:.8}", q.estimated_cost as f64 / 1e18),
                    "estimated_fee_bnb":  format!("{:.8}", q.estimated_fee as f64 / 1e18),
                },
                "tx_plan": [
                    format!(
                        "TokenManager.buyTokenAMAP({}, {}, {}) with msg.value = {} wei BNB",
                        token, funds_wei, min_amount, funds_wei
                    ),
                ],
                "note": "preview only (--confirm omitted): no transactions submitted.",
            }
        });
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    // Pre-flight: BNB balance must cover msg.value + gas
    let need_gas = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_BUY).await?;
    let need_total = funds_wei.saturating_add(need_gas);
    let have = eth_get_balance_wei(args.chain, &wallet).await?;
    if have < need_total {
        anyhow::bail!(
            "Insufficient BNB: have {:.6}, need ~{:.6} ({:.6} for trade + {:.6} for gas).",
            wei_to_bnb(have), wei_to_bnb(need_total),
            wei_to_bnb(funds_wei), wei_to_bnb(need_gas),
        );
    }

    let calldata = crate::calldata::build_buy_token_amap(&token, funds_wei, min_amount);
    eprintln!("[fourmeme] buying {} BNB worth of {} (min {} tokens)...",
        args.funds, sym, super::fmt_decimal(min_amount, TOKEN_DECIMALS));

    let resp = crate::onchainos::wallet_contract_call(
        args.chain,
        &q.token_manager, // route to the manager Helper3 reported (V1 vs V2)
        &calldata,
        Some(&wallet),
        Some(funds_wei),
        false, // user-facing tx, allow backend prompt
    ).await?;
    let tx_hash = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[fourmeme] buy tx: {} (waiting for confirmation...)", tx_hash);
    crate::onchainos::wait_for_tx_receipt(&tx_hash, args.chain, 120).await?;

    // Read post-trade balance to report what was actually filled.
    // EVM-012: post-tx read is display-only (the buy already confirmed).
    // Keep the soft fallback but expose the query error.
    let (post_bal, post_bal_query_error) = match super::erc20_balance(args.chain, &token, &wallet).await {
        Ok(v) => (v, None::<String>),
        Err(e) => (0u128, Some(format!("{:#}", e))),
    };

    let out = serde_json::json!({
        "ok": true,
        "data": {
            "action": "buy (BNB-quoted)",
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "wallet": wallet,
            "token": token,
            "symbol": sym,
            "token_manager": q.token_manager,
            "spent_bnb": args.funds,
            "spent_bnb_wei": funds_wei.to_string(),
            "preview_amount":      super::fmt_decimal(q.estimated_amount, TOKEN_DECIMALS),
            "min_amount_floor":    super::fmt_decimal(min_amount, TOKEN_DECIMALS),
            "post_trade_balance":     super::fmt_decimal(post_bal, TOKEN_DECIMALS),
            "post_trade_balance_raw": post_bal.to_string(),
            "post_trade_balance_query_error": post_bal_query_error,
            "buy_tx": tx_hash,
            "on_chain_status": "0x1",
            "tip": format!("Run `fourmeme-plugin sell --token {} --all` when you want to exit.", token),
        }
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
