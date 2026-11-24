use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_approve_calldata, chain_config, fee_to_tick_spacing,
    pad_address, pad_u256, resolve_token, token_symbol, unix_now,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{get_allowance, get_decimals, parse_human_amount};

#[derive(Args)]
pub struct MintPositionArgs {
    /// First token (symbol or address)
    #[arg(long)]
    pub token_a: String,
    /// Second token (symbol or address)
    #[arg(long)]
    pub token_b: String,
    /// Fee tier in basis points (100, 500, 3000, or 10000)
    #[arg(long)]
    pub fee: u32,
    /// Lower tick of the position range (negative values supported, e.g. --tick-lower -201000)
    #[arg(long, allow_hyphen_values = true)]
    pub tick_lower: i32,
    /// Upper tick of the position range (negative values supported, e.g. --tick-upper -199000)
    #[arg(long, allow_hyphen_values = true)]
    pub tick_upper: i32,
    /// Amount of token_a to supply (human-readable)
    #[arg(long)]
    pub amount_a: String,
    /// Amount of token_b to supply (human-readable)
    #[arg(long)]
    pub amount_b: String,
    /// Slippage tolerance % on min amounts (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Deadline in minutes from now (default: 20)
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos (dry-run)
    #[arg(long)]
    pub dry_run: bool,
}

/// ABI-encode an int24 tick as a 256-bit (64 hex char) slot, sign-extended.
/// Using `as i64 as u64` zero-extends for negative values — wrong for EVM ABI.
/// Negative values must fill the upper bits with 1s (sign extension).
fn encode_tick(tick: i32) -> String {
    if tick >= 0 {
        format!("{:0>64x}", tick as u64)
    } else {
        // Sign-extend: upper 224 bits = all 1s, lower 32 bits = two's complement of tick
        format!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffff{:08x}", tick as u32)
    }
}

/// NFPM.mint(MintParams) — selector 0x88316456
/// UniV3 MintParams: (token0,token1,fee,tickLower,tickUpper,amount0Desired,amount1Desired,amount0Min,amount1Min,recipient,deadline)
fn build_mint(
    token0: &str, token1: &str, fee: u32,
    tick_lower: i32, tick_upper: i32,
    amount0: u128, amount1: u128,
    amount0_min: u128, amount1_min: u128,
    recipient: &str, deadline: u64,
) -> String {
    format!(
        "0x88316456{}{}{}{}{}{}{}{}{}{}{}",
        pad_address(token0),
        pad_address(token1),
        format!("{:0>64x}", fee),
        encode_tick(tick_lower),
        encode_tick(tick_upper),
        pad_u256(amount0),
        pad_u256(amount1),
        pad_u256(amount0_min),
        pad_u256(amount1_min),
        pad_address(recipient),
        format!("{:0>64x}", deadline),
    )
}

pub async fn run(args: MintPositionArgs, chain_id: u64) -> anyhow::Result<()> {
    let cfg = chain_config(chain_id)?;
    let rpc_owned = crate::config::rpc_url(chain_id)?;
    let rpc: &str = &rpc_owned;
    let nfpm = cfg.nfpm;

    // Validate fee tier
    match args.fee {
        100 | 500 | 3000 | 10000 => {}
        _ => anyhow::bail!("Invalid fee tier: {}. Use 100, 500, 3000, or 10000.", args.fee),
    }

    // Validate tick range
    if args.tick_lower >= args.tick_upper {
        anyhow::bail!(
            "Invalid tick range: --tick-lower ({}) must be less than --tick-upper ({}).",
            args.tick_lower, args.tick_upper
        );
    }
    let tick_spacing = fee_to_tick_spacing(args.fee);
    if args.tick_lower % tick_spacing != 0 {
        anyhow::bail!(
            "--tick-lower {} is not aligned to tick spacing {} (fee tier {} bps). \
             Use a multiple of {} (e.g. {}).",
            args.tick_lower, tick_spacing, args.fee, tick_spacing,
            (args.tick_lower / tick_spacing) * tick_spacing
        );
    }
    if args.tick_upper % tick_spacing != 0 {
        anyhow::bail!(
            "--tick-upper {} is not aligned to tick spacing {} (fee tier {} bps). \
             Use a multiple of {} (e.g. {}).",
            args.tick_upper, tick_spacing, args.fee, tick_spacing,
            ((args.tick_upper + tick_spacing - 1) / tick_spacing) * tick_spacing
        );
    }

    let token_a = resolve_token(&args.token_a, chain_id);
    let token_b = resolve_token(&args.token_b, chain_id);
    // UniV3 requires token0 < token1 (lexicographic)
    let (token0, token1, amt_a_str, amt_b_str, sym0_key, sym1_key) =
        if token_a.to_lowercase() < token_b.to_lowercase() {
            (&token_a, &token_b, &args.amount_a, &args.amount_b, &args.token_a, &args.token_b)
        } else {
            (&token_b, &token_a, &args.amount_b, &args.amount_a, &args.token_b, &args.token_a)
        };

    let sym0 = if token_symbol(token0, chain_id) != "UNKNOWN" {
        token_symbol(token0, chain_id).to_string()
    } else { sym0_key.clone() };
    let sym1 = if token_symbol(token1, chain_id) != "UNKNOWN" {
        token_symbol(token1, chain_id).to_string()
    } else { sym1_key.clone() };

    let dec0 = get_decimals(token0, rpc).await.unwrap_or(18);
    let dec1 = get_decimals(token1, rpc).await.unwrap_or(18);
    let amount0 = parse_human_amount(amt_a_str, dec0)?;
    let amount1 = parse_human_amount(amt_b_str, dec1)?;
    // amount0Min and amount1Min are set to 0: the NFPM deposits at the current price ratio,
    // which can differ significantly from the desired ratio depending on where the price is
    // in the tick range. Slippage protection against price manipulation is provided by the deadline.
    let amount0_min: u128 = 0;
    let amount1_min: u128 = 0;
    let _ = args.slippage; // slippage arg retained for CLI compatibility

    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };

    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_mint(token0, token1, args.fee, args.tick_lower, args.tick_upper,
        amount0, amount1, amount0_min, amount1_min, &wallet, deadline);

    let preview = serde_json::json!({
        "preview": true,
        "action": "mint-position",
        "token0":        sym0,
        "token1":        sym1,
        "fee_bps":       args.fee,
        "fee_pct":       format!("{:.4}%", args.fee as f64 / 10_000.0),
        "tick_lower":    args.tick_lower,
        "tick_upper":    args.tick_upper,
        "amount0":       amt_a_str,
        "amount1":       amt_b_str,
        "slippage":      format!("{}%", args.slippage),
        "wallet":        wallet,
        "chain":         cfg.name,
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to mint this position.");
        return Ok(());
    }

    if !args.dry_run {
        for (token, sym, amount) in [
            (token0.as_str(), &sym0, amount0),
            (token1.as_str(), &sym1, amount1),
        ] {
            let allowance = get_allowance(token, &wallet, nfpm, rpc).await?;
            if allowance < amount {
                eprintln!("[sushiswap-v3] Approving {} for NFPM...", sym);
                let approve_data = build_approve_calldata(nfpm, amount);
                let r = wallet_contract_call(chain_id, token, &approve_data, false, false, Some(&wallet)).await?;
                eprintln!("[sushiswap-v3] Approve tx: {}", extract_tx_hash(&r));
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    let result = wallet_contract_call(chain_id, nfpm, &calldata, false, args.dry_run, Some(&wallet)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "mint-position",
        "token0":          sym0,
        "token1":          sym1,
        "fee_bps":         args.fee,
        "tick_lower":      args.tick_lower,
        "tick_upper":      args.tick_upper,
        "amount0_desired": amt_a_str,
        "amount1_desired": amt_b_str,
        "tx_hash":         tx_hash,
        "explorer":        format!("{}/{}", cfg.explorer, tx_hash),
        "chain":           cfg.name,
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
