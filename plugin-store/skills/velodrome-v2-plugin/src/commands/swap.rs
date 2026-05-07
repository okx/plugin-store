use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{
    build_approve_calldata, build_swap_calldata, factory_address,
    resolve_token_address, router_address, rpc_url, unix_now,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_get_pool, get_allowance, get_erc20_decimals, parse_human_amount, router_get_amounts_out};

const CHAIN_ID: u64 = 10;

#[derive(Args)]
pub struct SwapArgs {
    /// Input token (symbol or hex address, e.g. USDC, WETH, 0x...)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or hex address)
    #[arg(long)]
    pub token_out: String,
    /// Amount in (human-readable decimal, e.g. 0.1 for 0.1 WETH, 1.5 for 1.5 USDC)
    #[arg(long)]
    pub amount_in: String,
    /// Slippage tolerance in percent (e.g. 0.5 = 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Use stable pool (false = volatile, true = stable). If omitted, auto-selects best.
    #[arg(long)]
    pub stable: Option<bool>,
    /// Transaction deadline in minutes from now
    #[arg(long, default_value = "20")]
    pub deadline_minutes: u64,
    /// Dry run -- build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: SwapArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let token_in = resolve_token_address(&args.token_in);
    let token_out = resolve_token_address(&args.token_out);
    let factory = factory_address();
    let router = router_address();

    // --- 0. Parse amount_in ---
    let decimals_in = get_erc20_decimals(&token_in, rpc).await?;
    let amount_in = parse_human_amount(&args.amount_in, decimals_in)?;

    // --- 1. Find best pool (volatile or stable) ---
    let stable_options: Vec<bool> = match args.stable {
        Some(s) => vec![s],
        None => vec![false, true],
    };

    let mut best_amount_out: u128 = 0;
    let mut best_stable: bool = false;

    for stable in stable_options {
        let pool_addr = factory_get_pool(&token_in, &token_out, stable, factory, rpc).await?;
        if pool_addr == "0x0000000000000000000000000000000000000000" {
            continue;
        }
        match router_get_amounts_out(router, amount_in, &token_in, &token_out, stable, factory, rpc).await {
            Ok(amount_out) if amount_out > best_amount_out => {
                best_amount_out = amount_out;
                best_stable = stable;
            }
            _ => {}
        }
    }

    if best_amount_out == 0 {
        anyhow::bail!("No valid pool or quote found. Check token addresses and pool type.");
    }

    let slippage_factor = 1.0 - (args.slippage / 100.0);
    let amount_out_min = (best_amount_out as f64 * slippage_factor) as u128;

    // Preview gate — emit structured preview and exit before any wallet/on-chain calls
    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Add --confirm to broadcast",
            "data": {
                "action": "swap",
                "token_in": token_in,
                "token_out": token_out,
                "amount_in": amount_in,
                "stable": best_stable,
                "estimated_amount_out": best_amount_out,
                "amount_out_min": amount_out_min
            }
        }))?);
        return Ok(());
    }

    // --- 2. Resolve recipient ---
    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    // --- 3. Check allowance and approve if needed ---
    if !args.dry_run {
        let allowance = get_allowance(&token_in, &recipient, router, rpc).await?;
        if allowance < amount_in {
            println!("Approving {} for Router...", token_in);
            let approve_data = build_approve_calldata(router, amount_in);
            let approve_result =
                wallet_contract_call(CHAIN_ID, &token_in, &approve_data, args.confirm, false).await?;
            println!("Approve tx: {}", extract_tx_hash(&approve_result));
            // Wait 3s for approve nonce to clear before swap
            sleep(Duration::from_secs(3)).await;
        }
    }

    // --- 4. Build swapExactTokensForTokens calldata ---
    let deadline = unix_now() + args.deadline_minutes * 60;
    let calldata = build_swap_calldata(
        amount_in,
        amount_out_min,
        &token_in,
        &token_out,
        best_stable,
        factory,
        &recipient,
        deadline,
    );

    let result = wallet_contract_call(CHAIN_ID, router, &calldata, args.confirm, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"tokenIn\":\"{}\",\"tokenOut\":\"{}\",\"amountIn\":{},\"stable\":{},\"amountOutMin\":{}}}",
        tx_hash, token_in, token_out, amount_in, best_stable, amount_out_min
    );

    Ok(())
}
