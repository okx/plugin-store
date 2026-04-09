// commands/remove_liquidity.rs — removeLiquidity / removeLiquidityETH
use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::onchainos::{self, erc20_approve};
use crate::rpc;

pub struct RemoveLiquidityArgs {
    pub chain_id: u64,
    pub token_a: String,
    pub token_b: String,
    pub liquidity: Option<u128>,  // LP token amount; None = use full balance
    pub slippage_bps: u64,
    pub deadline_secs: u64,
    pub from: Option<String>,
    pub rpc_url: Option<String>,
    pub dry_run: bool,
}

pub async fn run(args: RemoveLiquidityArgs) -> Result<serde_json::Value> {
    let cfg = chain_config(args.chain_id)?;
    let rpc = args.rpc_url.as_deref().unwrap_or(cfg.rpc_url);

    let native_b = is_native(&args.token_b);
    let native_a = is_native(&args.token_a);

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

    let token_a_addr = if native_a {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&args.token_a, args.chain_id)
    };
    let token_b_addr = if native_b {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&args.token_b, args.chain_id)
    };

    // Look up pair
    let pair_addr = rpc::factory_get_pair(cfg.factory, &token_a_addr, &token_b_addr, rpc).await?;
    if pair_addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("No V2 pair found for {} / {}.", args.token_a, args.token_b);
    }

    // Get LP balance
    let lp_balance = rpc::erc20_balance_of(&pair_addr, &wallet, rpc).await.unwrap_or(0);
    if !args.dry_run && lp_balance == 0 {
        anyhow::bail!("You have no LP tokens for this pair (pair: {}).", pair_addr);
    }

    let liquidity = args.liquidity.unwrap_or(lp_balance);
    if liquidity == 0 && !args.dry_run {
        anyhow::bail!("Liquidity amount is zero.");
    }

    // Compute expected withdrawal
    let (reserve0, reserve1, _) = rpc::pair_get_reserves(&pair_addr, rpc).await?;
    let token0 = rpc::pair_token0(&pair_addr, rpc).await?;
    let total_supply = rpc::erc20_total_supply(&pair_addr, rpc).await.unwrap_or(1);

    let (reserve_a, reserve_b) = if token0.to_lowercase() == token_a_addr.to_lowercase() {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    let liq_u128 = if liquidity == 0 { 1u128 } else { liquidity };
    let amount_a_expected = reserve_a * liq_u128 / total_supply;
    let amount_b_expected = reserve_b * liq_u128 / total_supply;
    let amount_a_min = amount_a_expected * (10000 - args.slippage_bps) as u128 / 10000;
    let amount_b_min = amount_b_expected * (10000 - args.slippage_bps) as u128 / 10000;

    let deadline = rpc::current_timestamp(rpc).await.unwrap_or(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ) + args.deadline_secs;

    let mut steps = vec![];

    // Approve LP tokens to Router02
    let lp_allowance = rpc::erc20_allowance(&pair_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
    if lp_allowance < liquidity {
        let r = erc20_approve(
            args.chain_id, &pair_addr, cfg.router02, liquidity,
            args.from.as_deref(), args.dry_run,
        ).await?;
        steps.push(json!({"step":"approve_lp","txHash": onchainos::extract_tx_hash(&r)}));
        if !args.dry_run { sleep(Duration::from_secs(5)).await; }
    }

    if native_a || native_b {
        // removeLiquidityETH
        let (token_addr, tok_a_min, eth_min) = if native_b {
            (&token_a_addr, amount_a_min, amount_b_min)
        } else {
            (&token_b_addr, amount_b_min, amount_a_min)
        };
        let calldata = build_remove_liquidity_eth(
            token_addr, liquidity, tok_a_min, eth_min, &wallet, deadline,
        );
        let result = onchainos::wallet_contract_call(
            args.chain_id, cfg.router02, &calldata,
            args.from.as_deref(), None, args.dry_run,
        ).await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        steps.push(json!({
            "step": "removeLiquidityETH",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    } else {
        // removeLiquidity
        let calldata = build_remove_liquidity(
            &token_a_addr, &token_b_addr,
            liquidity, amount_a_min, amount_b_min,
            &wallet, deadline,
        );
        let result = onchainos::wallet_contract_call(
            args.chain_id, cfg.router02, &calldata,
            args.from.as_deref(), None, args.dry_run,
        ).await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        steps.push(json!({
            "step": "removeLiquidity",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    }

    Ok(json!({
        "ok": true,
        "steps": steps,
        "data": {
            "pair": pair_addr,
            "lpBurned": liquidity.to_string(),
            "lpBalance": lp_balance.to_string(),
            "expectedTokenA": amount_a_expected.to_string(),
            "expectedTokenB": amount_b_expected.to_string(),
            "tokenA": token_a_addr,
            "tokenB": token_b_addr,
            "chain": args.chain_id
        }
    }))
}

/// Build calldata for removeLiquidity
/// Selector: 0xbaa2abde
fn build_remove_liquidity(
    token_a: &str,
    token_b: &str,
    liquidity: u128,
    amount_a_min: u128,
    amount_b_min: u128,
    to: &str,
    deadline: u64,
) -> String {
    format!(
        "0xbaa2abde{}{}{}{}{}{}{}",
        pad_addr(token_a),
        pad_addr(token_b),
        format!("{:064x}", liquidity),
        format!("{:064x}", amount_a_min),
        format!("{:064x}", amount_b_min),
        pad_addr(to),
        format!("{:064x}", deadline),
    )
}

/// Build calldata for removeLiquidityETH
/// Selector: 0x02751cec
fn build_remove_liquidity_eth(
    token: &str,
    liquidity: u128,
    amount_token_min: u128,
    amount_eth_min: u128,
    to: &str,
    deadline: u64,
) -> String {
    format!(
        "0x02751cec{}{}{}{}{}{}",
        pad_addr(token),
        format!("{:064x}", liquidity),
        format!("{:064x}", amount_token_min),
        format!("{:064x}", amount_eth_min),
        pad_addr(to),
        format!("{:064x}", deadline),
    )
}

fn pad_addr(addr: &str) -> String {
    format!("{:0>64}", addr.trim_start_matches("0x").trim_start_matches("0X"))
}
