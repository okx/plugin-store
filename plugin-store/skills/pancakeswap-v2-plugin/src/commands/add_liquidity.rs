// commands/add_liquidity.rs — addLiquidity / addLiquidityETH
use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::onchainos::{self, erc20_approve};
use crate::rpc;

/// Estimate LP tokens minted using V2 formula:
///   lp = min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1)
/// Returns "N/A" if reserves or total supply cannot be fetched (e.g. new pair).
async fn estimate_lp_received(
    pair_addr: &str,
    token0: &str,
    token1: &str,
    amount0: u128,
    amount1: u128,
    rpc_url: &str,
) -> String {
    let ts = match rpc::erc20_total_supply(pair_addr, rpc_url).await {
        Ok(v) => v,
        Err(_) => return "N/A".to_string(),
    };
    if ts == 0 {
        // Brand-new pair — first deposit mints sqrt(amount0 * amount1) - MINIMUM_LIQUIDITY
        // Use integer approximation via Newton's method
        let product = (amount0 as u128).saturating_mul(amount1 as u128);
        let sqrt = isqrt_u128(product).saturating_sub(1000);
        return format_lp_human(sqrt);
    }
    let (r0, r1, _) = match rpc::pair_get_reserves(pair_addr, rpc_url).await {
        Ok(v) => v,
        Err(_) => return "N/A".to_string(),
    };
    // Determine which reserve matches which token
    let (res0, res1) = if token0.to_lowercase() < token1.to_lowercase() {
        (r0, r1)
    } else {
        (r1, r0)
    };
    if res0 == 0 || res1 == 0 {
        return "N/A".to_string();
    }
    let lp0 = (amount0 as u128).saturating_mul(ts) / res0;
    let lp1 = (amount1 as u128).saturating_mul(ts) / res1;
    let lp = lp0.min(lp1);
    format_lp_human(lp)
}

fn isqrt_u128(n: u128) -> u128 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

fn format_lp_human(lp: u128) -> String {
    // LP tokens have 18 decimals
    let whole = lp / 1_000_000_000_000_000_000u128;
    let frac  = (lp % 1_000_000_000_000_000_000u128) / 1_000_000_000_000u128;
    format!("{}.{:06}", whole, frac)
}

pub struct AddLiquidityArgs {
    pub chain_id: u64,
    pub token_a: String,
    pub token_b: String,
    pub amount_a: String,        // human-readable decimal, e.g. "10" or "0.5"
    pub amount_b: String,        // human-readable decimal
    pub slippage_bps: u64,
    pub deadline_secs: u64,
    pub from: Option<String>,
    pub rpc_url: Option<String>,
    pub dry_run: bool,
    pub confirm: bool,
}

pub async fn run(args: AddLiquidityArgs) -> Result<serde_json::Value> {
    let cfg = chain_config(args.chain_id)?;
    let rpc = args.rpc_url.as_deref().unwrap_or(cfg.rpc_url);

    let native_a = is_native(&args.token_a);
    let native_b = is_native(&args.token_b);

    if native_a && native_b {
        anyhow::bail!("Cannot add liquidity with two native tokens.");
    }

    // Resolve wallet
    let wallet = if let Some(ref f) = args.from {
        f.clone()
    } else if args.dry_run {
        // Use a recognisable placeholder so dry-run output is clearly non-live
        "0xDRYRUN0000000000000000000000000000000000".to_string()
    } else {
        let w = onchainos::resolve_wallet(args.chain_id).unwrap_or_default();
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
        }
        w
    };

    let deadline = rpc::current_timestamp(rpc).await.unwrap_or(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ) + args.deadline_secs;

    // Resolve token decimals and parse human-readable amounts
    let token_a_for_dec = if native_a { cfg.weth.to_string() } else { resolve_token_address(&args.token_a, args.chain_id)? };
    let token_b_for_dec = if native_b { cfg.weth.to_string() } else { resolve_token_address(&args.token_b, args.chain_id)? };
    let decimals_a = rpc::erc20_decimals(&token_a_for_dec, rpc).await.unwrap_or(18);
    let decimals_b = rpc::erc20_decimals(&token_b_for_dec, rpc).await.unwrap_or(18);
    let amount_a = rpc::parse_human_amount(&args.amount_a, decimals_a)?;
    let amount_b = rpc::parse_human_amount(&args.amount_b, decimals_b)?;
    if amount_a == 0 && amount_b == 0 {
        anyhow::bail!("Both amounts are zero — provide at least one non-zero amount.");
    }

    let mut steps = vec![];
    // Track resolved addresses for LP estimation
    let mut token_a_addr_for_lp = String::new();
    let mut token_b_addr_for_lp = String::new();

    if native_a || native_b {
        // addLiquidityETH variant
        let (token_sym, token_amount, eth_amount) = if native_b {
            (&args.token_a, amount_a, amount_b)
        } else {
            (&args.token_b, amount_b, amount_a)
        };
        let token_addr = resolve_token_address(token_sym, args.chain_id)?;
        let token_min = token_amount * (10000 - args.slippage_bps) as u128 / 10000;
        let eth_min = eth_amount * (10000 - args.slippage_bps) as u128 / 10000;
        token_a_addr_for_lp = token_addr.clone();
        token_b_addr_for_lp = cfg.weth.to_string();

        // Approve token if needed
        let allowance = rpc::erc20_allowance(&token_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allowance < token_amount {
            let r = erc20_approve(
                args.chain_id, &token_addr, cfg.router02, token_amount,
                args.from.as_deref(), args.dry_run, args.confirm,
            ).await?;
            steps.push(json!({"step":"approve_token","txHash": onchainos::extract_tx_hash(&r)}));
            if !args.dry_run && args.confirm { sleep(Duration::from_secs(5)).await; }
        }

        let calldata = build_add_liquidity_eth(&token_addr, token_amount, token_min, eth_min, &wallet, deadline);
        let result = onchainos::wallet_contract_call(
            args.chain_id, cfg.router02, &calldata,
            args.from.as_deref(), Some(eth_amount), args.dry_run, args.confirm,
        ).await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        if !args.dry_run && args.confirm {
            onchainos::wait_and_check_receipt(&tx_hash, rpc).await?;
        }
        steps.push(json!({
            "step": "addLiquidityETH",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    } else {
        // addLiquidity variant (token + token)
        let token_a_addr = resolve_token_address(&args.token_a, args.chain_id)?;
        let token_b_addr = resolve_token_address(&args.token_b, args.chain_id)?;
        token_a_addr_for_lp = token_a_addr.clone();
        token_b_addr_for_lp = token_b_addr.clone();
        let amount_a_min = amount_a * (10000 - args.slippage_bps) as u128 / 10000;
        let amount_b_min = amount_b * (10000 - args.slippage_bps) as u128 / 10000;

        // Approve tokenA if needed
        let allow_a = rpc::erc20_allowance(&token_a_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allow_a < amount_a {
            let r = erc20_approve(
                args.chain_id, &token_a_addr, cfg.router02, amount_a,
                args.from.as_deref(), args.dry_run, args.confirm,
            ).await?;
            steps.push(json!({"step":"approve_tokenA","txHash": onchainos::extract_tx_hash(&r)}));
            if !args.dry_run && args.confirm { sleep(Duration::from_secs(5)).await; }
        }

        // Approve tokenB if needed
        let allow_b = rpc::erc20_allowance(&token_b_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allow_b < amount_b {
            let r = erc20_approve(
                args.chain_id, &token_b_addr, cfg.router02, amount_b,
                args.from.as_deref(), args.dry_run, args.confirm,
            ).await?;
            steps.push(json!({"step":"approve_tokenB","txHash": onchainos::extract_tx_hash(&r)}));
            if !args.dry_run && args.confirm { sleep(Duration::from_secs(5)).await; }
        }

        let calldata = build_add_liquidity(
            &token_a_addr, &token_b_addr,
            amount_a, amount_b,
            amount_a_min, amount_b_min,
            &wallet, deadline,
        );
        let result = onchainos::wallet_contract_call(
            args.chain_id, cfg.router02, &calldata,
            args.from.as_deref(), None, args.dry_run, args.confirm,
        ).await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        if !args.dry_run && args.confirm {
            onchainos::wait_and_check_receipt(&tx_hash, rpc).await?;
        }
        steps.push(json!({
            "step": "addLiquidity",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    }

    // Estimate LP tokens received from on-chain pair state
    let lp_received = if args.dry_run {
        "estimated (dry-run)".to_string()
    } else {
        let pair_addr = rpc::factory_get_pair(
            cfg.factory, &token_a_addr_for_lp, &token_b_addr_for_lp, rpc
        ).await.unwrap_or_default();
        if pair_addr.len() > 2 && pair_addr != "0x0000000000000000000000000000000000000000" {
            estimate_lp_received(&pair_addr, &token_a_addr_for_lp, &token_b_addr_for_lp, amount_a, amount_b, rpc).await
        } else {
            "unknown (pair not yet created or lookup failed)".to_string()
        }
    };

    Ok(json!({
        "ok": true,
        "steps": steps,
        "data": {
            "tokenA": args.token_a,
            "tokenB": args.token_b,
            "amountA": amount_a.to_string(),
            "amountB": amount_b.to_string(),
            "lpReceived": lp_received,
            "wallet": wallet,
            "chain": args.chain_id
        }
    }))
}

/// Build calldata for addLiquidity
/// Selector: 0xe8e33700
fn build_add_liquidity(
    token_a: &str,
    token_b: &str,
    amount_a_desired: u128,
    amount_b_desired: u128,
    amount_a_min: u128,
    amount_b_min: u128,
    to: &str,
    deadline: u64,
) -> String {
    format!(
        "0xe8e33700{}{}{}{}{}{}{}{}",
        pad_addr(token_a),
        pad_addr(token_b),
        format!("{:064x}", amount_a_desired),
        format!("{:064x}", amount_b_desired),
        format!("{:064x}", amount_a_min),
        format!("{:064x}", amount_b_min),
        pad_addr(to),
        format!("{:064x}", deadline),
    )
}

/// Build calldata for addLiquidityETH
/// Selector: 0xf305d719
fn build_add_liquidity_eth(
    token: &str,
    amount_token_desired: u128,
    amount_token_min: u128,
    amount_eth_min: u128,
    to: &str,
    deadline: u64,
) -> String {
    format!(
        "0xf305d719{}{}{}{}{}{}",
        pad_addr(token),
        format!("{:064x}", amount_token_desired),
        format!("{:064x}", amount_token_min),
        format!("{:064x}", amount_eth_min),
        pad_addr(to),
        format!("{:064x}", deadline),
    )
}

fn pad_addr(addr: &str) -> String {
    format!("{:0>64}", addr.trim_start_matches("0x").trim_start_matches("0X"))
}
