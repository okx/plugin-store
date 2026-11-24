use clap::Args;
use crate::config::{
    build_approve_calldata, factory, pad_address, pad_bool, pad_u256,
    resolve_token_validated, router, rpc_url, token_symbol, unix_now, CHAIN_ID,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    amm_get_pool, format_amount, get_allowance, get_balance_of,
    get_decimals, get_total_supply, parse_human_amount, pool_get_reserves, pool_token0,
    wait_for_allowance,
};

#[derive(Args)]
pub struct RemoveLiquidityArgs {
    /// First token of the pool (symbol or address)
    #[arg(long)]
    pub token_a: String,
    /// Second token of the pool (symbol or address)
    #[arg(long)]
    pub token_b: String,
    /// Exact LP token amount to burn (human-readable), OR use --percent
    #[arg(long)]
    pub liquidity: Option<String>,
    /// Percentage of your LP position to remove (1–100)
    #[arg(long)]
    pub percent: Option<f64>,
    /// Remove from stable pool (default: volatile)
    #[arg(long)]
    pub stable: bool,
    /// Slippage tolerance in percent (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Deadline in minutes from now (default: 20)
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Broadcast on-chain
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata only, do not call onchainos
    #[arg(long)]
    pub dry_run: bool,
}

/// Router.removeLiquidity(tokenA, tokenB, stable, liquidity, amountAMin, amountBMin, to, deadline)
/// Selector: 0x0dede6c4
fn build_remove_liquidity_calldata(
    token_a: &str, token_b: &str, stable: bool,
    liquidity: u128, amount_a_min: u128, amount_b_min: u128,
    to: &str, deadline: u64,
) -> String {
    format!(
        "0x0dede6c4{}{}{}{}{}{}{}{}",
        pad_address(token_a),
        pad_address(token_b),
        pad_bool(stable),
        pad_u256(liquidity),
        pad_u256(amount_a_min),
        pad_u256(amount_b_min),
        pad_address(to),
        pad_u256(deadline as u128),
    )
}

pub async fn run(args: RemoveLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let rtr = router();
    let token_a = resolve_token_validated(&args.token_a)?;
    let token_b = resolve_token_validated(&args.token_b)?;

    let sym_a = resolve_symbol(&token_a, &args.token_a);
    let sym_b = resolve_symbol(&token_b, &args.token_b);

    let dec_a = get_decimals(&token_a, rpc).await.unwrap_or(18);
    let dec_b = get_decimals(&token_b, rpc).await.unwrap_or(18);

    let zero = "0x0000000000000000000000000000000000000000";
    let pool_type = if args.stable { "stable" } else { "volatile" };
    let pool = amm_get_pool(fac, &token_a, &token_b, args.stable, rpc).await?;

    if pool == zero {
        anyhow::bail!("No Aerodrome AMM {} pool found for {}/{}.", pool_type, sym_a, sym_b);
    }

    let recipient = if args.dry_run {
        zero.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    // ── Resolve LP amount ──────────────────────────────────────────────────────
    // In dry-run mode, look up the real wallet's LP balance (not the zero address)
    // so --percent can compute a non-zero amount for calldata preview.
    let balance_addr = if args.dry_run {
        resolve_wallet(CHAIN_ID).unwrap_or_else(|_| zero.to_string())
    } else {
        recipient.clone()
    };
    let lp_balance = get_balance_of(&pool, &balance_addr, rpc).await.unwrap_or(0);

    // In dry-run mode with --percent and zero LP balance, use 1 LP token as a
    // calldata-generation mock so the command produces useful output.
    const DRY_RUN_MOCK_LP: u128 = 1_000_000_000_000_000_000; // 1.0 LP token

    let liquidity_raw = if let Some(ref liq_str) = args.liquidity {
        parse_human_amount(liq_str, 18)?
    } else if let Some(pct) = args.percent {
        if pct <= 0.0 || pct > 100.0 {
            anyhow::bail!("--percent must be between 1 and 100");
        }
        let effective_balance = if args.dry_run && lp_balance == 0 {
            DRY_RUN_MOCK_LP
        } else {
            lp_balance
        };
        (effective_balance as f64 * pct / 100.0) as u128
    } else {
        anyhow::bail!("Specify either --liquidity <amount> or --percent <1-100>");
    };

    if liquidity_raw == 0 {
        anyhow::bail!("LP amount resolved to 0. Check your balance with `aerodrome-amm positions`.");
    }
    if !args.dry_run && liquidity_raw > lp_balance {
        anyhow::bail!(
            "Insufficient LP balance. Requested {}, have {}.",
            format_amount(liquidity_raw, 18), format_amount(lp_balance, 18)
        );
    }

    // ── Compute expected underlying amounts ────────────────────────────────────
    let total_supply = get_total_supply(&pool, rpc).await.unwrap_or(1);
    let t0           = pool_token0(&pool, rpc).await.unwrap_or_default();
    let (r0, r1)     = pool_get_reserves(&pool, rpc).await.unwrap_or((0, 0));

    let share    = liquidity_raw as f64 / total_supply as f64;
    let (exp_a, exp_b) = if t0.to_lowercase() == token_a.to_lowercase() {
        ((r0 as f64 * share) as u128, (r1 as f64 * share) as u128)
    } else {
        ((r1 as f64 * share) as u128, (r0 as f64 * share) as u128)
    };

    let slip  = 1.0 - (args.slippage / 100.0);
    let min_a = (exp_a as f64 * slip) as u128;
    let min_b = (exp_b as f64 * slip) as u128;

    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_remove_liquidity_calldata(
        &token_a, &token_b, args.stable,
        liquidity_raw, min_a, min_b, &recipient, deadline,
    );

    // ── Preview ────────────────────────────────────────────────────────────────
    let preview = serde_json::json!({
        "preview": true,
        "action": "remove_liquidity",
        "pool": pool,
        "pool_type": pool_type,
        "token_a": sym_a,
        "token_b": sym_b,
        "lp_to_burn": format_amount(liquidity_raw, 18),
        "expected_a": format_amount(exp_a, dec_a),
        "expected_b": format_amount(exp_b, dec_b),
        "minimum_a": format_amount(min_a, dec_a),
        "minimum_b": format_amount(min_b, dec_b),
        "slippage": format!("{}%", args.slippage),
        "wallet": recipient,
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast.");
        return Ok(());
    }

    // ── Approve LP token (pool address) to Router ──────────────────────────────
    if !args.dry_run {
        let allowance = get_allowance(&pool, &recipient, rtr, rpc).await?;
        if allowance < liquidity_raw {
            eprintln!("[aerodrome-amm] Approving LP token for Router...");
            let approve_data   = build_approve_calldata(rtr, liquidity_raw);
            let approve_result = wallet_contract_call(CHAIN_ID, &pool, &approve_data, false, false, Some(&recipient)).await?;
            let approve_hash   = extract_tx_hash(&approve_result);
            eprintln!("[aerodrome-amm] Approve tx: {}. Waiting for allowance to confirm on-chain...", approve_hash);
            wait_for_allowance(&pool, &recipient, rtr, liquidity_raw, rpc).await?;
        }
    }

    // ── Execute ────────────────────────────────────────────────────────────────
    let result  = wallet_contract_call(CHAIN_ID, rtr, &calldata, false, args.dry_run, Some(&recipient)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "remove_liquidity",
        "pool": pool,
        "pool_type": pool_type,
        "token_a": sym_a,
        "token_b": sym_b,
        "lp_burned": format_amount(liquidity_raw, 18),
        "minimum_a": format_amount(min_a, dec_a),
        "minimum_b": format_amount(min_b, dec_b),
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
        out["calldata"] = serde_json::json!(calldata);
        if args.percent.is_some() && lp_balance == 0 {
            out["note"] = serde_json::json!(
                "Amounts estimated using a 1 LP token mock (no active position). \
                 Run with your actual LP balance for accurate output estimates."
            );
        }
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn resolve_symbol(addr: &str, fallback: &str) -> String {
    let s = token_symbol(addr);
    if s != "UNKNOWN" { s.to_string() } else { fallback.to_string() }
}
