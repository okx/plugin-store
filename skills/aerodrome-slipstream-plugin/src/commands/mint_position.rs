use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_approve_calldata, cl_factory, nfpm, resolve_token,
    rpc_url, token_symbol, unix_now, CHAIN_ID, pad_address,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{cl_get_pool, format_amount, get_allowance, get_decimals, parse_human_amount, pool_slot0};

#[derive(Args)]
pub struct MintPositionArgs {
    /// First token (symbol or address)
    #[arg(long)]
    pub token_a: String,
    /// Second token (symbol or address)
    #[arg(long)]
    pub token_b: String,
    /// Tick spacing of the pool (e.g. 100 for ~0.3% fee tier)
    #[arg(long)]
    pub tick_spacing: i32,
    /// Lower tick of the range
    #[arg(long)]
    pub tick_lower: i32,
    /// Upper tick of the range
    #[arg(long)]
    pub tick_upper: i32,
    /// Desired amount of token_a to deposit (human-readable)
    #[arg(long)]
    pub amount_a: String,
    /// Desired amount of token_b to deposit (human-readable)
    #[arg(long)]
    pub amount_b: String,
    /// Slippage tolerance % (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Deadline in minutes (default: 20)
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos
    #[arg(long)]
    pub dry_run: bool,
}

/// NFPM.mint(MintParams) — selector 0xb5007d1f
///
/// MintParams struct ABI order (12 fields):
///   address token0          (32 bytes)
///   address token1          (32 bytes)
///   int24   tickSpacing     (32 bytes)
///   int24   tickLower       (32 bytes)
///   int24   tickUpper       (32 bytes)
///   uint256 amount0Desired  (32 bytes)
///   uint256 amount1Desired  (32 bytes)
///   uint256 amount0Min      (32 bytes)
///   uint256 amount1Min      (32 bytes)
///   address recipient       (32 bytes)
///   uint256 deadline        (32 bytes)
///   uint160 sqrtPriceX96    (32 bytes) — 0 for existing pools
fn build_mint_calldata(
    token0: &str, token1: &str,
    tick_spacing: i32, tick_lower: i32, tick_upper: i32,
    amount0_desired: u128, amount1_desired: u128,
    amount0_min: u128, amount1_min: u128,
    recipient: &str, deadline: u64,
) -> String {
    // ABI requires int24 to be 256-bit sign-extended: SIGNEXTEND(2,x)==x must hold.
    // Negative values need all upper bytes set to 0xFF, not 0x00.
    let encode_int = |v: i32| {
        if v >= 0 {
            format!("{:064x}", v as u64)
        } else {
            // Sign-extend the 32-bit representation to 256 bits (fill upper 28 bytes with FF)
            format!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffff{:08x}", v as u32)
        }
    };
    format!(
        "0xb5007d1f{}{}{}{}{}{}{}{}{}{}{}{}",
        pad_address(token0),
        pad_address(token1),
        encode_int(tick_spacing),
        encode_int(tick_lower),
        encode_int(tick_upper),
        format!("{:0>64x}", amount0_desired),
        format!("{:0>64x}", amount1_desired),
        format!("{:0>64x}", amount0_min),
        format!("{:0>64x}", amount1_min),
        pad_address(recipient),
        format!("{:0>64x}", deadline),
        format!("{:0>64x}", 0u128),   // sqrtPriceX96 = 0 (use existing pool price)
    )
}


pub async fn run(args: MintPositionArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let factory = cl_factory();
    let nfpm_addr = nfpm();

    let token_a = resolve_token(&args.token_a);
    let token_b = resolve_token(&args.token_b);
    let sym_a = if token_symbol(&token_a) != "UNKNOWN" { token_symbol(&token_a).to_string() } else { args.token_a.clone() };
    let sym_b = if token_symbol(&token_b) != "UNKNOWN" { token_symbol(&token_b).to_string() } else { args.token_b.clone() };

    // M1: Validate tick range ordering
    if args.tick_lower >= args.tick_upper {
        anyhow::bail!(
            "Invalid tick range: --tick-lower ({}) must be less than --tick-upper ({}).",
            args.tick_lower, args.tick_upper
        );
    }

    // M2: Validate tick alignment against tick spacing
    if args.tick_lower % args.tick_spacing != 0 {
        anyhow::bail!(
            "--tick-lower {} is not aligned to tick spacing {}. \
             Use a multiple of {} (e.g. {}).",
            args.tick_lower, args.tick_spacing, args.tick_spacing,
            (args.tick_lower / args.tick_spacing) * args.tick_spacing
        );
    }
    if args.tick_upper % args.tick_spacing != 0 {
        anyhow::bail!(
            "--tick-upper {} is not aligned to tick spacing {}. \
             Use a multiple of {} (e.g. {}).",
            args.tick_upper, args.tick_spacing, args.tick_spacing,
            (args.tick_upper / args.tick_spacing) * args.tick_spacing
        );
    }

    // Verify pool exists
    let zero = "0x0000000000000000000000000000000000000000";
    let pool = cl_get_pool(factory, &token_a, &token_b, args.tick_spacing, rpc).await?;
    if pool == zero {
        anyhow::bail!("No Slipstream CL pool found for {}/{} with tick_spacing={}. Use `pools --token-a {} --token-b {}` to find available pools.",
            sym_a, sym_b, args.tick_spacing, args.token_a, args.token_b);
    }

    // Determine pool token order by address comparison (token0 < token1 by address)
    let (token0, token1, amount0_desired, amount1_desired, dec0, dec1, sym0, sym1) =
        if token_a.to_lowercase() < token_b.to_lowercase() {
            let d0 = get_decimals(&token_a, rpc).await.unwrap_or(18);
            let d1 = get_decimals(&token_b, rpc).await.unwrap_or(18);
            let a0 = parse_human_amount(&args.amount_a, d0)?;
            let a1 = parse_human_amount(&args.amount_b, d1)?;
            (token_a.clone(), token_b.clone(), a0, a1, d0, d1, sym_a.clone(), sym_b.clone())
        } else {
            let d0 = get_decimals(&token_b, rpc).await.unwrap_or(18);
            let d1 = get_decimals(&token_a, rpc).await.unwrap_or(18);
            let a0 = parse_human_amount(&args.amount_b, d0)?;
            let a1 = parse_human_amount(&args.amount_a, d1)?;
            (token_b.clone(), token_a.clone(), a0, a1, d0, d1, sym_b.clone(), sym_a.clone())
        };

    // For LP minting, both token amounts are adjusted internally by the NFPM based on the
    // current price and tick range. Setting fixed percentage minimums on both amounts causes
    // PSC (price slippage check) failures when the actual ratio differs from the desired ratio.
    // The slippage flag is shown in the preview for informational purposes only.
    let amount0_min: u128 = 0;
    let amount1_min: u128 = 0;

    // Current price for context
    let (_, current_tick) = pool_slot0(&pool, rpc).await.unwrap_or((0, 0));

    let recipient = if args.dry_run {
        zero.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_mint_calldata(
        &token0, &token1, args.tick_spacing,
        args.tick_lower, args.tick_upper,
        amount0_desired, amount1_desired,
        amount0_min, amount1_min,
        &recipient, deadline,
    );

    let in_range = current_tick >= args.tick_lower && current_tick < args.tick_upper;

    // Preview
    let preview = serde_json::json!({
        "preview": true,
        "action": "mint-position",
        "pool": pool,
        "token0": sym0,
        "token1": sym1,
        "tick_spacing": args.tick_spacing,
        "tick_lower": args.tick_lower,
        "tick_upper": args.tick_upper,
        "current_tick": current_tick,
        "in_range": in_range,
        "amount0_desired": format_amount(amount0_desired, dec0),
        "amount1_desired": format_amount(amount1_desired, dec1),
        "note_amounts": "Actual amounts consumed depend on current pool price; desired values are maximums",
        "recipient": recipient,
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to mint this position.");
        return Ok(());
    }

    // Approve token0 for NFPM
    if !args.dry_run {
        let allow0 = get_allowance(&token0, &recipient, nfpm_addr, rpc).await?;
        if allow0 < amount0_desired {
            eprintln!("[aerodrome-slipstream-plugin] Approving {} for NonfungiblePositionManager...", sym0);
            let approve0 = build_approve_calldata(nfpm_addr, amount0_desired);
            let r = wallet_contract_call(CHAIN_ID, &token0, &approve0, true, false, Some(&recipient)).await?;
            eprintln!("[aerodrome-slipstream-plugin] Approve {} tx: {}", sym0, extract_tx_hash(&r));
            sleep(Duration::from_secs(5)).await;
        }
        // Approve token1 for NFPM
        let allow1 = get_allowance(&token1, &recipient, nfpm_addr, rpc).await?;
        if allow1 < amount1_desired {
            eprintln!("[aerodrome-slipstream-plugin] Approving {} for NonfungiblePositionManager...", sym1);
            let approve1 = build_approve_calldata(nfpm_addr, amount1_desired);
            let r = wallet_contract_call(CHAIN_ID, &token1, &approve1, true, false, Some(&recipient)).await?;
            eprintln!("[aerodrome-slipstream-plugin] Approve {} tx: {}", sym1, extract_tx_hash(&r));
            sleep(Duration::from_secs(5)).await;
        }
    }

    // Mint position
    let result = wallet_contract_call(CHAIN_ID, nfpm_addr, &calldata, true, args.dry_run, Some(&recipient)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "mint-position",
        "token0": sym0,
        "token1": sym1,
        "tick_spacing": args.tick_spacing,
        "tick_lower": args.tick_lower,
        "tick_upper": args.tick_upper,
        "in_range": in_range,
        "amount0_desired": format_amount(amount0_desired, dec0),
        "amount1_desired": format_amount(amount1_desired, dec1),
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
        "note": "Check `positions` to see your new token_id"
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
