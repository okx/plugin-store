use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Get user's sUSDS (and optionally sDAI) balance on the given chain.
pub async fn run(chain_id: u64, from: Option<&str>, dry_run: bool) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let wallet = match from {
        Some(addr) => addr.to_string(),
        None => onchainos::resolve_wallet(chain_id, dry_run)
            .context("Failed to resolve wallet address")?,
    };

    // Read sUSDS balance: balanceOf(address) = 0x70a08231
    let susds_bal_hex = rpc::eth_call(
        cfg.rpc_url,
        cfg.susds,
        &format!("0x70a08231{}", rpc::encode_address(&wallet)),
    )
    .await
    .context("Failed to read sUSDS balance")?;

    let susds_shares = rpc::decode_u256(&susds_bal_hex).unwrap_or(0);
    let susds_human = rpc::from_minimal(susds_shares, 18);

    // Convert sUSDS shares to USDS equivalent
    // On L2: read SSR oracle getConversionRate()
    // On Ethereum: use sUSDS.convertToAssets(shares)
    let usds_equivalent = if cfg.use_psm3 {
        // L2: conversion_rate from oracle (1e27 format) = chi accumulator
        // usds_value = shares * chi / 1e27
        if let Some(oracle) = cfg.ssr_oracle {
            match rpc::eth_call(cfg.rpc_url, oracle, "0xf36089ec").await {
                Ok(rate_hex) => {
                    let chi = rpc::decode_u256(&rate_hex).unwrap_or(1_000_000_000_000_000_000_000_000_000);
                    // chi is in 1e27, shares in 1e18
                    // usds_amount = shares * chi / 1e27 (in 1e18 units)
                    let usds_minimal = (susds_shares as u128)
                        .checked_mul(chi)
                        .map(|v| v / 1_000_000_000_000_000_000_000_000_000u128)
                        .unwrap_or(susds_shares);
                    rpc::from_minimal(usds_minimal, 18)
                }
                Err(_) => susds_human,
            }
        } else {
            susds_human
        }
    } else {
        // Ethereum: convertToAssets(uint256 shares) = 0x07a2d13a
        let data = format!(
            "0x07a2d13a{}",
            rpc::encode_u256(susds_shares)
        );
        match rpc::eth_call(cfg.rpc_url, cfg.susds, &data).await {
            Ok(result_hex) => {
                let assets = rpc::decode_u256(&result_hex).unwrap_or(susds_shares);
                rpc::from_minimal(assets, 18)
            }
            Err(_) => susds_human,
        }
    };

    // Also read USDS balance
    let usds_bal_hex = rpc::eth_call(
        cfg.rpc_url,
        cfg.usds,
        &format!("0x70a08231{}", rpc::encode_address(&wallet)),
    )
    .await
    .unwrap_or_default();
    let usds_bal = rpc::decode_u256(&usds_bal_hex).unwrap_or(0);
    let usds_human = rpc::from_minimal(usds_bal, 18);

    let mut result = json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "wallet": wallet,
        "sUSDS": {
            "balance": format!("{:.6}", susds_human),
            "balanceMinimal": susds_shares.to_string(),
            "usdEquivalent": format!("{:.6}", usds_equivalent),
            "token": cfg.susds
        },
        "USDS": {
            "balance": format!("{:.6}", usds_human),
            "balanceMinimal": usds_bal.to_string(),
            "token": cfg.usds
        }
    });

    // On Ethereum, also show sDAI balance
    if let Some(sdai_addr) = cfg.sdai {
        let sdai_bal_hex = rpc::eth_call(
            cfg.rpc_url,
            sdai_addr,
            &format!("0x70a08231{}", rpc::encode_address(&wallet)),
        )
        .await
        .unwrap_or_default();
        let sdai_bal = rpc::decode_u256(&sdai_bal_hex).unwrap_or(0);
        let sdai_human = rpc::from_minimal(sdai_bal, 18);

        // convertToAssets for sDAI
        let dai_equivalent = if sdai_bal > 0 {
            let data = format!("0x07a2d13a{}", rpc::encode_u256(sdai_bal));
            match rpc::eth_call(cfg.rpc_url, sdai_addr, &data).await {
                Ok(hex) => {
                    let assets = rpc::decode_u256(&hex).unwrap_or(sdai_bal);
                    rpc::from_minimal(assets, 18)
                }
                Err(_) => sdai_human,
            }
        } else {
            0.0
        };

        result["sDAI"] = json!({
            "balance": format!("{:.6}", sdai_human),
            "balanceMinimal": sdai_bal.to_string(),
            "daiEquivalent": format!("{:.6}", dai_equivalent),
            "token": sdai_addr
        });
    }

    Ok(result)
}
