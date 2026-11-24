use clap::Args;
use crate::config::{
    build_approve_calldata, factory, pad_address, pad_bool, pad_u256,
    resolve_token_validated, router, rpc_url, token_symbol, unix_now, CHAIN_ID,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    amm_get_pool, format_amount, get_allowance, get_balance_of,
    get_decimals, parse_human_amount, router_quote_add_liquidity, wait_for_allowance,
};

#[derive(Args)]
pub struct AddLiquidityArgs {
    /// First token (symbol or address)
    #[arg(long)]
    pub token_a: String,
    /// Second token (symbol or address)
    #[arg(long)]
    pub token_b: String,
    /// Desired amount of token_a (human-readable)
    #[arg(long)]
    pub amount_a: String,
    /// Desired amount of token_b (human-readable)
    #[arg(long)]
    pub amount_b: String,
    /// Use stable pool (default: volatile)
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

/// Router.addLiquidity(tokenA, tokenB, stable, amountADesired, amountBDesired,
///                     amountAMin, amountBMin, to, deadline)
/// Selector: 0x5a47ddc3
fn build_add_liquidity_calldata(
    token_a: &str, token_b: &str, stable: bool,
    amount_a: u128, amount_b: u128,
    amount_a_min: u128, amount_b_min: u128,
    to: &str, deadline: u64,
) -> String {
    format!(
        "0x5a47ddc3{}{}{}{}{}{}{}{}{}",
        pad_address(token_a),
        pad_address(token_b),
        pad_bool(stable),
        pad_u256(amount_a),
        pad_u256(amount_b),
        pad_u256(amount_a_min),
        pad_u256(amount_b_min),
        pad_address(to),
        pad_u256(deadline as u128),
    )
}

pub async fn run(args: AddLiquidityArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let rtr = router();
    let token_a = resolve_token_validated(&args.token_a)?;
    let token_b = resolve_token_validated(&args.token_b)?;

    let sym_a = resolve_symbol(&token_a, &args.token_a);
    let sym_b = resolve_symbol(&token_b, &args.token_b);

    let dec_a = get_decimals(&token_a, rpc).await.unwrap_or(18);
    let dec_b = get_decimals(&token_b, rpc).await.unwrap_or(18);
    let amount_a_raw = parse_human_amount(&args.amount_a, dec_a)?;
    let amount_b_raw = parse_human_amount(&args.amount_b, dec_b)?;

    if amount_a_raw == 0 || amount_b_raw == 0 {
        anyhow::bail!("Both amounts must be greater than 0");
    }

    // ── 1. Verify pool exists ──────────────────────────────────────────────────
    let zero = "0x0000000000000000000000000000000000000000";
    let pool = amm_get_pool(fac, &token_a, &token_b, args.stable, rpc).await?;
    let pool_type = if args.stable { "stable" } else { "volatile" };

    if pool == zero {
        anyhow::bail!(
            "No Aerodrome AMM {} pool exists for {}/{}. \
             Create it first via the Aerodrome UI, or use --stable/remove --stable to switch pool type.",
            pool_type, sym_a, sym_b
        );
    }

    // ── 2. Quote how much will actually be used ────────────────────────────────
    let (used_a, used_b, lp_out) = router_quote_add_liquidity(
        rtr, fac, &token_a, &token_b, args.stable,
        amount_a_raw, amount_b_raw, rpc,
    ).await?;

    let slip  = 1.0 - (args.slippage / 100.0);
    let min_a = (used_a as f64 * slip) as u128;
    let min_b = (used_b as f64 * slip) as u128;

    // ── 3. Check wallet balances ───────────────────────────────────────────────
    let recipient = if args.dry_run {
        zero.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    if !args.dry_run {
        let bal_a = get_balance_of(&token_a, &recipient, rpc).await.unwrap_or(0);
        let bal_b = get_balance_of(&token_b, &recipient, rpc).await.unwrap_or(0);
        if bal_a < used_a {
            anyhow::bail!(
                "Insufficient {} balance. Need {}, have {}.",
                sym_a, format_amount(used_a, dec_a), format_amount(bal_a, dec_a)
            );
        }
        if bal_b < used_b {
            anyhow::bail!(
                "Insufficient {} balance. Need {}, have {}.",
                sym_b, format_amount(used_b, dec_b), format_amount(bal_b, dec_b)
            );
        }
    }

    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_add_liquidity_calldata(
        &token_a, &token_b, args.stable,
        amount_a_raw, amount_b_raw, min_a, min_b,
        &recipient, deadline,
    );

    // ── 4. Preview ─────────────────────────────────────────────────────────────
    let preview = serde_json::json!({
        "preview": true,
        "action": "add_liquidity",
        "pool": pool,
        "pool_type": pool_type,
        "token_a": sym_a,
        "token_b": sym_b,
        "amount_a_desired": args.amount_a,
        "amount_b_desired": args.amount_b,
        "amount_a_used": format_amount(used_a, dec_a),
        "amount_b_used": format_amount(used_b, dec_b),
        "lp_tokens_expected": format_amount(lp_out, 18),
        "slippage": format!("{}%", args.slippage),
        "wallet": recipient,
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast.");
        return Ok(());
    }

    // ── 5. Approve token_a and token_b if needed ───────────────────────────────
    if !args.dry_run {
        for (token, sym, amount) in [(&token_a, &sym_a, amount_a_raw), (&token_b, &sym_b, amount_b_raw)] {
            let allowance = get_allowance(token, &recipient, rtr, rpc).await?;
            if allowance < amount {
                eprintln!("[aerodrome-amm] Approving {} {} for Router...", sym, format_amount(amount, if token == &token_a { dec_a } else { dec_b }));
                let approve_data   = build_approve_calldata(rtr, amount);
                let approve_result = wallet_contract_call(CHAIN_ID, token, &approve_data, false, false, Some(&recipient)).await?;
                let approve_hash   = extract_tx_hash(&approve_result);
                eprintln!("[aerodrome-amm] Approve tx: {}. Waiting for allowance to confirm on-chain...", approve_hash);
                wait_for_allowance(token, &recipient, rtr, amount, rpc).await?;
            }
        }
    }

    // ── 6. Execute ─────────────────────────────────────────────────────────────
    let result  = wallet_contract_call(CHAIN_ID, rtr, &calldata, false, args.dry_run, Some(&recipient)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "add_liquidity",
        "pool": pool,
        "pool_type": pool_type,
        "token_a": sym_a,
        "token_b": sym_b,
        "amount_a_used": format_amount(used_a, dec_a),
        "amount_b_used": format_amount(used_b, dec_b),
        "lp_tokens_expected": format_amount(lp_out, 18),
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
        out["calldata"] = serde_json::json!(calldata);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn resolve_symbol(addr: &str, fallback: &str) -> String {
    let s = token_symbol(addr);
    if s != "UNKNOWN" { s.to_string() } else { fallback.to_string() }
}
