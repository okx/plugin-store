// commands/swap.rs — swap tokens via PancakeSwap V2 Router02
use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::config::{chain_config, resolve_token_address, is_native};
use crate::onchainos::{self, erc20_approve};
use crate::rpc;
use super::quote::determine_path;

pub struct SwapArgs {
    pub chain_id: u64,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: String,       // human-readable decimal, e.g. "1.5" or "100"
    pub slippage_bps: u64,       // e.g. 50 = 0.5%
    pub deadline_secs: u64,
    pub from: Option<String>,
    pub rpc_url: Option<String>,
    pub dry_run: bool,
    pub confirm: bool,
}

pub async fn run(args: SwapArgs) -> Result<serde_json::Value> {
    let cfg = chain_config(args.chain_id)?;
    let rpc = args.rpc_url.as_deref().unwrap_or(cfg.rpc_url);

    let token_in_sym = args.token_in.clone();
    let token_out_sym = args.token_out.clone();
    let native_in = is_native(&token_in_sym);
    let native_out = is_native(&token_out_sym);

    let token_in_addr = if native_in {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&token_in_sym, args.chain_id)?
    };
    let token_out_addr = if native_out {
        cfg.weth.to_string()
    } else {
        resolve_token_address(&token_out_sym, args.chain_id)?
    };

    if token_in_addr == token_out_addr {
        anyhow::bail!("tokenIn and tokenOut must be different tokens.");
    }

    // Resolve tokenIn decimals and parse human-readable amount
    let decimals_in = rpc::erc20_decimals(&token_in_addr, rpc).await.unwrap_or(18);
    let amount_in = rpc::parse_human_amount(&args.amount_in, decimals_in)?;
    if amount_in == 0 {
        anyhow::bail!("Amount must be greater than 0.");
    }

    // Resolve wallet
    let wallet = if args.dry_run {
        args.from.clone().unwrap_or_else(|| "0xDRYRUN0000000000000000000000000000000000".to_string())
    } else {
        let w = args.from.clone()
            .unwrap_or_else(|| onchainos::resolve_wallet(args.chain_id).unwrap_or_default());
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
        }
        w
    };

    // Determine path
    let path = determine_path(&token_in_addr, &token_out_addr, cfg.factory, cfg.weth, rpc).await?;

    // Quote for amountOutMin
    let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    let amounts = rpc::router_get_amounts_out(cfg.router02, amount_in, &path_refs, rpc).await?;
    let amount_out_expected = *amounts.last().unwrap_or(&0);
    if amount_out_expected == 0 {
        anyhow::bail!("Swap would yield 0 output tokens — input amount is too small or no liquidity for this pair.");
    }
    let amount_out_min = amount_out_expected * (10000 - args.slippage_bps) as u128 / 10000;

    // Deadline
    let now = rpc::current_timestamp(rpc).await.unwrap_or(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    let deadline = now + args.deadline_secs;

    let mut results = json!({
        "ok": true,
        "steps": []
    });
    let steps = results["steps"].as_array_mut().unwrap();

    if native_in {
        // Variant B: Native ETH/BNB → Token (swapExactETHForTokens)
        let calldata = build_swap_exact_eth_for_tokens(
            amount_out_min,
            &path,
            &wallet,
            deadline,
        );
        let result = onchainos::wallet_contract_call(
            args.chain_id,
            cfg.router02,
            &calldata,
            args.from.as_deref(),
            Some(amount_in),
            args.dry_run,
            args.confirm,
        )
        .await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        if !args.dry_run {
            onchainos::wait_and_check_receipt(&tx_hash, rpc).await?;
        }
        steps.push(json!({
            "step": "swapExactETHForTokens",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    } else if native_out {
        // Variant C: Token → Native ETH/BNB (swapExactTokensForETH)
        // Check and approve if needed
        let allowance = rpc::erc20_allowance(&token_in_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allowance < amount_in {
            let approve_result = erc20_approve(
                args.chain_id,
                &token_in_addr,
                cfg.router02,
                amount_in,
                args.from.as_deref(),
                args.dry_run,
                args.confirm,
            )
            .await?;
            steps.push(json!({
                "step": "approve",
                "txHash": onchainos::extract_tx_hash(&approve_result)
            }));
            if !args.dry_run && args.confirm {
                sleep(Duration::from_secs(3)).await;
            }
        }

        let calldata = build_swap_exact_tokens_for_eth(
            amount_in,
            amount_out_min,
            &path,
            &wallet,
            deadline,
        );
        let result = onchainos::wallet_contract_call(
            args.chain_id,
            cfg.router02,
            &calldata,
            args.from.as_deref(),
            None,
            args.dry_run,
            args.confirm,
        )
        .await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        if !args.dry_run && args.confirm {
            onchainos::wait_and_check_receipt(&tx_hash, rpc).await?;
        }
        steps.push(json!({
            "step": "swapExactTokensForETH",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    } else {
        // Variant A: Token → Token (swapExactTokensForTokens)
        // Check and approve if needed
        let allowance = rpc::erc20_allowance(&token_in_addr, &wallet, cfg.router02, rpc).await.unwrap_or(0);
        if allowance < amount_in {
            let approve_result = erc20_approve(
                args.chain_id,
                &token_in_addr,
                cfg.router02,
                amount_in,
                args.from.as_deref(),
                args.dry_run,
                args.confirm,
            )
            .await?;
            steps.push(json!({
                "step": "approve",
                "txHash": onchainos::extract_tx_hash(&approve_result)
            }));
            if !args.dry_run && args.confirm {
                sleep(Duration::from_secs(3)).await;
            }
        }

        let calldata = build_swap_exact_tokens_for_tokens(
            amount_in,
            amount_out_min,
            &path,
            &wallet,
            deadline,
        );
        let result = onchainos::wallet_contract_call(
            args.chain_id,
            cfg.router02,
            &calldata,
            args.from.as_deref(),
            None,
            args.dry_run,
            args.confirm,
        )
        .await?;
        let tx_hash = onchainos::extract_tx_hash(&result).to_string();
        if !args.dry_run && args.confirm {
            onchainos::wait_and_check_receipt(&tx_hash, rpc).await?;
        }
        steps.push(json!({
            "step": "swapExactTokensForTokens",
            "txHash": tx_hash,
            "explorer": format!("{}/tx/{}", cfg.explorer, tx_hash)
        }));
    }

    results["data"] = json!({
        "tokenIn": token_in_addr,
        "tokenOut": token_out_addr,
        "symbolIn": args.token_in,
        "symbolOut": args.token_out,
        "amountIn": amount_in.to_string(),
        "amountOutMin": amount_out_min.to_string(),
        "amountOutExpected": amount_out_expected.to_string(),
        "path": path,
        "wallet": wallet,
        "chain": args.chain_id
    });

    Ok(results)
}

/// Build calldata for swapExactTokensForTokens
/// Selector: 0x38ed1739
fn build_swap_exact_tokens_for_tokens(
    amount_in: u128,
    amount_out_min: u128,
    path: &[String],
    to: &str,
    deadline: u64,
) -> String {
    let selector = "38ed1739";
    let amount_in_hex = format!("{:064x}", amount_in);
    let amount_out_min_hex = format!("{:064x}", amount_out_min);
    // offset to path: 5 * 32 = 160 = 0xa0
    let path_offset = format!("{:064x}", 0xa0u64);
    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").trim_start_matches("0X"));
    let deadline_hex = format!("{:064x}", deadline);
    let path_len = format!("{:064x}", path.len());
    let mut path_bytes = String::new();
    for addr in path {
        path_bytes.push_str(&format!(
            "{:0>64}",
            addr.trim_start_matches("0x").trim_start_matches("0X")
        ));
    }
    format!(
        "0x{}{}{}{}{}{}{}{}",
        selector,
        amount_in_hex,
        amount_out_min_hex,
        path_offset,
        to_padded,
        deadline_hex,
        path_len,
        path_bytes
    )
}

/// Build calldata for swapExactETHForTokens
/// Selector: 0x7ff36ab5
fn build_swap_exact_eth_for_tokens(
    amount_out_min: u128,
    path: &[String],
    to: &str,
    deadline: u64,
) -> String {
    let selector = "7ff36ab5";
    let amount_out_min_hex = format!("{:064x}", amount_out_min);
    // offset to path: 4 * 32 = 128 = 0x80
    let path_offset = format!("{:064x}", 0x80u64);
    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").trim_start_matches("0X"));
    let deadline_hex = format!("{:064x}", deadline);
    let path_len = format!("{:064x}", path.len());
    let mut path_bytes = String::new();
    for addr in path {
        path_bytes.push_str(&format!(
            "{:0>64}",
            addr.trim_start_matches("0x").trim_start_matches("0X")
        ));
    }
    format!(
        "0x{}{}{}{}{}{}{}",
        selector,
        amount_out_min_hex,
        path_offset,
        to_padded,
        deadline_hex,
        path_len,
        path_bytes
    )
}

/// Build calldata for swapExactTokensForETH
/// Selector: 0x18cbafe5
fn build_swap_exact_tokens_for_eth(
    amount_in: u128,
    amount_out_min: u128,
    path: &[String],
    to: &str,
    deadline: u64,
) -> String {
    let selector = "18cbafe5";
    let amount_in_hex = format!("{:064x}", amount_in);
    let amount_out_min_hex = format!("{:064x}", amount_out_min);
    let path_offset = format!("{:064x}", 0xa0u64);
    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").trim_start_matches("0X"));
    let deadline_hex = format!("{:064x}", deadline);
    let path_len = format!("{:064x}", path.len());
    let mut path_bytes = String::new();
    for addr in path {
        path_bytes.push_str(&format!(
            "{:0>64}",
            addr.trim_start_matches("0x").trim_start_matches("0X")
        ));
    }
    format!(
        "0x{}{}{}{}{}{}{}{}",
        selector,
        amount_in_hex,
        amount_out_min_hex,
        path_offset,
        to_padded,
        deadline_hex,
        path_len,
        path_bytes
    )
}
