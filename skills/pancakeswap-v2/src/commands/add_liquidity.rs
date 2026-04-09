// commands/add_liquidity.rs — addLiquidity / addLiquidityETH
use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::onchainos::{self, erc20_approve};
use crate::rpc;

pub struct AddLiquidityArgs {
    pub chain_id: u64,
    pub token_a: String,
    pub token_b: String,
    pub amount_a: u128,          // desired amount of token_a (minimal units)
    pub amount_b: u128,          // desired amount of token_b or ETH (minimal units)
    pub slippage_bps: u64,
    pub deadline_secs: u64,
    pub from: Option<String>,
    pub rpc_url: Option<String>,
    pub dry_run: bool,
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
    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        let w = args.from.clone()
            .unwrap_or_else(|| onchainos::resolve_wallet(args.chain_id).unwrap_or_default());
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

    let mut steps = vec![];

    if native_a || native_b {
        // addLiquidityETH variant
        let (token_sym, token_amount, eth_amount) = if native_b {
            (&args.token_a, args.amount_a, args.amount_b)
        } else {
            (&args.token_b, args.amount_b, args.amount_a)
        };
        let token_addr = resolve_token_address(token_sym, args.chain_id);
        let token_min = token_amount * (10000 - args.slippage_bps) as u128 / 10000;
        let eth_min = eth_amount * (10000 - args.slippage_bps) as u128 / 10000;

        // Approve token if needed
        let allowance = rpc::erc20_allowance(&token_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allowance < token_amount {
            let r = erc20_approve(
                args.chain_id, &token_addr, cfg.router02, u128::MAX,
                args.from.as_deref(), args.dry_run,
            ).await?;
            steps.push(json!({"step":"approve_token","txHash": onchainos::extract_tx_hash(&r)}));
            if !args.dry_run { sleep(Duration::from_secs(5)).await; }
        }

        let calldata = build_add_liquidity_eth(&token_addr, token_amount, token_min, eth_min, &wallet, deadline);
        let result = onchainos::wallet_contract_call(
            args.chain_id, cfg.router02, &calldata,
            args.from.as_deref(), Some(eth_amount as u64), args.dry_run,
        ).await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        steps.push(json!({
            "step": "addLiquidityETH",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    } else {
        // addLiquidity variant (token + token)
        let token_a_addr = resolve_token_address(&args.token_a, args.chain_id);
        let token_b_addr = resolve_token_address(&args.token_b, args.chain_id);
        let amount_a_min = args.amount_a * (10000 - args.slippage_bps) as u128 / 10000;
        let amount_b_min = args.amount_b * (10000 - args.slippage_bps) as u128 / 10000;

        // Approve tokenA if needed
        let allow_a = rpc::erc20_allowance(&token_a_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allow_a < args.amount_a {
            let r = erc20_approve(
                args.chain_id, &token_a_addr, cfg.router02, u128::MAX,
                args.from.as_deref(), args.dry_run,
            ).await?;
            steps.push(json!({"step":"approve_tokenA","txHash": onchainos::extract_tx_hash(&r)}));
            if !args.dry_run { sleep(Duration::from_secs(5)).await; }
        }

        // Approve tokenB if needed
        let allow_b = rpc::erc20_allowance(&token_b_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allow_b < args.amount_b {
            let r = erc20_approve(
                args.chain_id, &token_b_addr, cfg.router02, u128::MAX,
                args.from.as_deref(), args.dry_run,
            ).await?;
            steps.push(json!({"step":"approve_tokenB","txHash": onchainos::extract_tx_hash(&r)}));
            if !args.dry_run { sleep(Duration::from_secs(5)).await; }
        }

        let calldata = build_add_liquidity(
            &token_a_addr, &token_b_addr,
            args.amount_a, args.amount_b,
            amount_a_min, amount_b_min,
            &wallet, deadline,
        );
        let result = onchainos::wallet_contract_call(
            args.chain_id, cfg.router02, &calldata,
            args.from.as_deref(), None, args.dry_run,
        ).await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        steps.push(json!({
            "step": "addLiquidity",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    }

    Ok(json!({
        "ok": true,
        "steps": steps,
        "data": {
            "tokenA": args.token_a,
            "tokenB": args.token_b,
            "amountA": args.amount_a.to_string(),
            "amountB": args.amount_b.to_string(),
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
