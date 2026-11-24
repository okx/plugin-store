use clap::Args;
use crate::config::{
    build_approve_calldata, factory, pad_address, pad_bool, pad_u256,
    resolve_token_validated, router, rpc_url, token_symbol, unix_now, CHAIN_ID,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    amm_get_pool, format_amount, get_allowance, get_balance_of, get_decimals,
    parse_human_amount, router_get_amounts_out, wait_for_allowance,
};

#[derive(Args)]
pub struct SwapArgs {
    /// Input token (symbol or address, e.g. WETH, USDC)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or address)
    #[arg(long)]
    pub token_out: String,
    /// Amount of token_in (human-readable, e.g. "0.01")
    #[arg(long)]
    pub amount_in: String,
    /// Slippage tolerance in percent (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Force stable pool (default: auto-select best output)
    #[arg(long)]
    pub stable: bool,
    /// Deadline in minutes from now (default: 20)
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Broadcast the swap on-chain
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata only, do not call onchainos
    #[arg(long)]
    pub dry_run: bool,
}

/// Router.swapExactTokensForTokens(amountIn, amountOutMin, Route[], to, deadline)
/// Selector: 0xcac88ea9
///
/// ABI encoding (1 route):
///   4   selector
///   32  amountIn
///   32  amountOutMin
///   32  offset to routes = 0xa0 (160)
///   32  to
///   32  deadline
///   32  routes.length = 1
///   32  routes[0].from
///   32  routes[0].to
///   32  routes[0].stable
///   32  routes[0].factory
fn build_swap_calldata(
    amount_in: u128,
    amount_out_min: u128,
    token_from: &str,
    token_to: &str,
    stable: bool,
    factory: &str,
    recipient: &str,
    deadline: u64,
) -> String {
    format!(
        "0xcac88ea9{}{}{}{}{}{}{}{}{}{}",
        pad_u256(amount_in),
        pad_u256(amount_out_min),
        "00000000000000000000000000000000000000000000000000000000000000a0", // offset = 160
        pad_address(recipient),
        pad_u256(deadline as u128),
        "0000000000000000000000000000000000000000000000000000000000000001", // routes.length
        pad_address(token_from),
        pad_address(token_to),
        pad_bool(stable),
        pad_address(factory),
    )
}

pub async fn run(args: SwapArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let rtr = router();
    let token_in  = resolve_token_validated(&args.token_in)?;
    let token_out = resolve_token_validated(&args.token_out)?;

    let sym_in  = resolve_symbol(&token_in, &args.token_in);
    let sym_out = resolve_symbol(&token_out, &args.token_out);

    let dec_in  = get_decimals(&token_in, rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, rpc).await.unwrap_or(18);
    let amount_in_raw = parse_human_amount(&args.amount_in, dec_in)?;

    if amount_in_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // ── 1. Find best pool ──────────────────────────────────────────────────────
    let zero = "0x0000000000000000000000000000000000000000";
    let pool_types: &[bool] = if args.stable { &[true] } else { &[false, true] };

    let mut best_out: u128 = 0;
    let mut best_stable = false;

    for &is_stable in pool_types {
        let pool = amm_get_pool(fac, &token_in, &token_out, is_stable, rpc).await?;
        if pool == zero { continue; }
        match router_get_amounts_out(rtr, fac, amount_in_raw, &token_in, &token_out, is_stable, rpc).await {
            Ok(out) if out > best_out => {
                best_out = out;
                best_stable = is_stable;
            }
            _ => {}
        }
    }

    if best_out == 0 {
        anyhow::bail!(
            "No Aerodrome AMM pool found for {} → {} on Base. \
             Use `aerodrome-amm pools --token-a {} --token-b {}` to check available pools.",
            sym_in, sym_out, args.token_in, args.token_out
        );
    }

    let slippage_factor = 1.0 - (args.slippage / 100.0);
    let amount_out_min = (best_out as f64 * slippage_factor) as u128;

    // ── 2. Resolve recipient ───────────────────────────────────────────────────
    let recipient = if args.dry_run {
        zero.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_swap_calldata(
        amount_in_raw, amount_out_min,
        &token_in, &token_out,
        best_stable, fac, &recipient, deadline,
    );

    // ── 3. Preview ─────────────────────────────────────────────────────────────
    let preview = serde_json::json!({
        "preview": true,
        "action": "swap",
        "token_in": sym_in,
        "token_out": sym_out,
        "amount_in": args.amount_in,
        "expected_out": format_amount(best_out, dec_out),
        "minimum_out": format_amount(amount_out_min, dec_out),
        "slippage": format!("{}%", args.slippage),
        "pool_type": if best_stable { "stable" } else { "volatile" },
        "router": rtr,
        "wallet": recipient,
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast this swap.");
        return Ok(());
    }

    // ── 4. Check wallet balance ────────────────────────────────────────────────
    if !args.dry_run {
        let balance = get_balance_of(&token_in, &recipient, rpc).await.unwrap_or(0);
        if balance < amount_in_raw {
            anyhow::bail!(
                "Insufficient {} balance. Need {}, have {}.",
                sym_in,
                format_amount(amount_in_raw, dec_in),
                format_amount(balance, dec_in)
            );
        }
    }

    // ── 5. Approve token_in if needed ──────────────────────────────────────────
    if !args.dry_run {
        let allowance = get_allowance(&token_in, &recipient, rtr, rpc).await?;
        if allowance < amount_in_raw {
            eprintln!("[aerodrome-amm] Approving {} {} for Router...", sym_in, args.amount_in);
            let approve_data   = build_approve_calldata(rtr, amount_in_raw);
            let approve_result = wallet_contract_call(CHAIN_ID, &token_in, &approve_data, false, false, Some(&recipient)).await?;
            let approve_hash   = extract_tx_hash(&approve_result);
            eprintln!("[aerodrome-amm] Approve tx: {}. Waiting for allowance to confirm on-chain...", approve_hash);
            wait_for_allowance(&token_in, &recipient, rtr, amount_in_raw, rpc).await?;
        }
    }

    // ── 5. Execute swap ────────────────────────────────────────────────────────
    let result  = wallet_contract_call(CHAIN_ID, rtr, &calldata, false, args.dry_run, Some(&recipient)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "swap",
        "token_in": sym_in,
        "token_out": sym_out,
        "amount_in": args.amount_in,
        "minimum_out": format_amount(amount_out_min, dec_out),
        "pool_type": if best_stable { "stable" } else { "volatile" },
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
