use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_approve_calldata, cl_factory, common_tick_spacings,
    resolve_token, rpc_url, swap_router, token_symbol, unix_now, CHAIN_ID, pad_address,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{cl_get_pool, format_amount, get_allowance, get_decimals, parse_human_amount};
use super::quote::get_quote;

#[derive(Args)]
pub struct SwapArgs {
    /// Input token (symbol or address, e.g. WETH, USDC)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or address)
    #[arg(long)]
    pub token_out: String,
    /// Amount of token_in to swap (human-readable, e.g. "0.01" for 0.01 WETH)
    #[arg(long)]
    pub amount_in: String,
    /// Slippage tolerance in percent (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Specific tick spacing to use (default: auto-select best)
    #[arg(long)]
    pub tick_spacing: Option<i32>,
    /// Deadline in minutes from now (default: 20)
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Broadcast the swap. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata but do not call onchainos (testing only)
    #[arg(long)]
    pub dry_run: bool,
}

/// SwapRouter.exactInputSingle(ExactInputSingleParams) → uint256 amountOut
/// Selector: keccak("exactInputSingle((address,address,int24,address,uint256,uint256,uint256,uint160))") = 0xa026383e
///
/// ExactInputSingleParams struct (ABI order):
///   address tokenIn       (32 bytes)
///   address tokenOut      (32 bytes)
///   int24   tickSpacing   (32 bytes)
///   address recipient     (32 bytes)
///   uint256 deadline      (32 bytes)
///   uint256 amountIn      (32 bytes)
///   uint256 amountOutMin  (32 bytes)
///   uint160 sqrtPriceLimitX96 (32 bytes)
fn build_exact_input_single(
    token_in: &str,
    token_out: &str,
    tick_spacing: i32,
    recipient: &str,
    deadline: u64,
    amount_in: u128,
    amount_out_min: u128,
) -> String {
    let ta  = pad_address(token_in);
    let tb  = pad_address(token_out);
    let ts  = format!("{:0>64x}", tick_spacing as u64);
    let rec = pad_address(recipient);
    let dl  = format!("{:0>64x}", deadline);
    let ain = format!("{:0>64x}", amount_in);
    let aom = format!("{:0>64x}", amount_out_min);
    let lim = format!("{:0>64x}", 0u64); // no sqrt price limit
    format!("0xa026383e{}{}{}{}{}{}{}{}", ta, tb, ts, rec, dl, ain, aom, lim)
}

pub async fn run(args: SwapArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let factory = cl_factory();
    let router = swap_router();
    let token_in = resolve_token(&args.token_in);
    let token_out = resolve_token(&args.token_out);

    let sym_in  = if token_symbol(&token_in) != "UNKNOWN"  { token_symbol(&token_in).to_string()  } else { args.token_in.clone() };
    let sym_out = if token_symbol(&token_out) != "UNKNOWN" { token_symbol(&token_out).to_string() } else { args.token_out.clone() };

    let dec_in  = get_decimals(&token_in, rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, rpc).await.unwrap_or(6);
    let amount_in_raw = parse_human_amount(&args.amount_in, dec_in)?;

    if amount_in_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // ── 1. Find best pool / quote ─────────────────────────────────────────────
    let zero = "0x0000000000000000000000000000000000000000";
    let tick_spacings: Vec<i32> = match args.tick_spacing {
        Some(ts) => vec![ts],
        None => common_tick_spacings().to_vec(),
    };

    let mut best_out: u128 = 0;
    let mut best_ts = 0i32;

    for ts in &tick_spacings {
        let pool = cl_get_pool(factory, &token_in, &token_out, *ts, rpc).await?;
        if pool == zero { continue; }
        match get_quote(&token_in, &token_out, amount_in_raw, *ts, rpc).await {
            Ok(out) if out > best_out => { best_out = out; best_ts = *ts; }
            _ => {}
        }
    }

    if best_out == 0 {
        anyhow::bail!(
            "No quote available for {} {} → {}. No Slipstream CL pool found with sufficient liquidity.",
            args.amount_in, sym_in, sym_out
        );
    }

    let slippage_factor = 1.0 - (args.slippage / 100.0);
    let amount_out_min = (best_out as f64 * slippage_factor) as u128;

    // ── 2. Resolve recipient ──────────────────────────────────────────────────
    let recipient = if args.dry_run {
        zero.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_exact_input_single(
        &token_in, &token_out, best_ts, &recipient, deadline, amount_in_raw, amount_out_min,
    );

    // ── 3. Preview ────────────────────────────────────────────────────────────
    let preview = serde_json::json!({
        "preview": true,
        "action": "swap",
        "token_in": sym_in,
        "token_out": sym_out,
        "amount_in": args.amount_in,
        "expected_out": format_amount(best_out, dec_out),
        "minimum_out": format_amount(amount_out_min, dec_out),
        "slippage": format!("{}%", args.slippage),
        "tick_spacing": best_ts,
        "recipient": recipient,
        "router": router,
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast this swap.");
        return Ok(());
    }

    // ── 4. Approve if needed ──────────────────────────────────────────────────
    if !args.dry_run {
        let allowance = get_allowance(&token_in, &recipient, router, rpc).await?;
        if allowance < amount_in_raw {
            eprintln!("[aerodrome-slipstream-plugin] Approving {} {} for SwapRouter...", sym_in, args.amount_in);
            let approve_data = build_approve_calldata(router, amount_in_raw);
            let approve_result = wallet_contract_call(CHAIN_ID, &token_in, &approve_data, true, false, Some(&recipient)).await?;
            let approve_hash = extract_tx_hash(&approve_result);
            eprintln!("[aerodrome-slipstream-plugin] Approve tx: {}", approve_hash);
            sleep(Duration::from_secs(5)).await;
        }
    }

    // ── 5. Execute swap ───────────────────────────────────────────────────────
    let result = wallet_contract_call(CHAIN_ID, router, &calldata, true, args.dry_run, Some(&recipient)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "swap",
        "token_in": sym_in,
        "token_out": sym_out,
        "amount_in": args.amount_in,
        "minimum_out": format_amount(amount_out_min, dec_out),
        "tick_spacing": best_ts,
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
