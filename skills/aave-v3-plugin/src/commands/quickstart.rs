use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

const ABOUT: &str = "Aave V3 is a leading decentralized liquidity protocol — supply assets to \
    earn yield, borrow against collateral with variable rates, and manage positions \
    across Ethereum, Base, Arbitrum, and Polygon. $20B+ TVL.";

// Canonical USDC addresses per chain
const USDC_ETHEREUM: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDC_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const USDC_ARBITRUM: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
const USDC_POLYGON: &str = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359";

// Minimum native token needed for approve + supply tx
const MIN_GAS_ETHEREUM_WEI: u128 = 3_000_000_000_000_000; // 0.003 ETH
const MIN_GAS_L2_WEI: u128 = 100_000_000_000_000;          // 0.0001 ETH (L2 cheap)

// Minimum meaningful token balances to be considered "funded"
const MIN_USDC_RAW: u128 = 1_000_000;           // 1 USDC (6 decimals)
const MIN_WETH_RAW: u128 = 500_000_000_000_000;  // 0.0005 WETH (18 decimals)

pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::wallet_address(chain_id)
            .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {e}"))?
    };

    eprintln!(
        "Checking assets for {}... on {}...",
        &wallet[..10.min(wallet.len())],
        cfg.name
    );

    let usdc_addr = match chain_id {
        1     => USDC_ETHEREUM,
        8453  => USDC_BASE,
        42161 => USDC_ARBITRUM,
        137   => USDC_POLYGON,
        _     => USDC_ETHEREUM,
    };
    let weth_addr = cfg.weth_address;
    let min_gas_wei = if chain_id == 1 { MIN_GAS_ETHEREUM_WEI } else { MIN_GAS_L2_WEI };

    // Resolve Pool address (needed for on-chain account data query)
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .unwrap_or_default();

    // Fetch wallet balances and Aave position data in parallel
    let (eth_res, usdc_res, weth_res, account_res) = tokio::join!(
        rpc::get_eth_balance(&wallet, cfg.rpc_url),
        rpc::get_erc20_balance(usdc_addr, &wallet, cfg.rpc_url),
        rpc::get_erc20_balance(weth_addr, &wallet, cfg.rpc_url),
        async {
            if pool_addr.is_empty() {
                return Err(anyhow::anyhow!("pool address unavailable"));
            }
            rpc::get_user_account_data(&pool_addr, &wallet, cfg.rpc_url).await
        },
    );

    let eth_wei   = eth_res.unwrap_or(0);
    let usdc_raw  = usdc_res.unwrap_or(0);
    let weth_raw  = weth_res.unwrap_or(0);

    let eth_balance  = eth_wei  as f64 / 1e18;
    let usdc_balance = usdc_raw as f64 / 1_000_000.0;
    let weth_balance = weth_raw as f64 / 1e18;

    // Has active Aave positions if collateral or debt is non-zero
    let has_positions = account_res.as_ref()
        .map(|a| a.total_collateral_base > 0 || a.total_debt_base > 0)
        .unwrap_or(false);

    let has_tokens = usdc_raw >= MIN_USDC_RAW || weth_raw >= MIN_WETH_RAW;
    let has_gas    = eth_wei  >= min_gas_wei;

    let chain_flag = if chain_id != 8453 {
        format!("--chain {} ", chain_id)
    } else {
        String::new()
    };
    let from_flag = format!("--from {}", &wallet);

    let (status, suggestion, onboarding_steps, next_command): (&str, &str, Vec<String>, String) =
        if has_positions {
            (
                "active",
                "You have open Aave V3 positions. Review your health factor and manage them.",
                vec![],
                format!("aave-v3-plugin {}positions {}", chain_flag, from_flag),
            )
        } else if has_gas && has_tokens {
            let (asset, example_amount) = if usdc_balance >= 1.0 {
                ("USDC", format!("{:.2}", (usdc_balance * 0.9).max(1.0).min(usdc_balance)))
            } else {
                ("WETH", format!("{:.4}", (weth_balance * 0.9).max(0.0005).min(weth_balance)))
            };
            (
                "ready",
                "Your wallet is funded. Supply assets to Aave V3 to start earning yield.",
                vec![
                    "1. Check current reserve rates:".to_string(),
                    format!("   aave-v3-plugin {}reserves", chain_flag),
                    "2. Supply assets to earn interest:".to_string(),
                    format!(
                        "   aave-v3-plugin {}--confirm supply --asset {} --amount {} {}",
                        chain_flag, asset, example_amount, from_flag
                    ),
                    "3. View your positions after supplying:".to_string(),
                    format!("   aave-v3-plugin {}positions {}", chain_flag, from_flag),
                ],
                format!("aave-v3-plugin {}reserves", chain_flag),
            )
        } else if has_tokens && !has_gas {
            (
                "needs_gas",
                "You have tokens but need ETH for gas fees. Send ETH to your wallet.",
                vec![
                    format!("1. Send at least {:.4} ETH (gas) to:", min_gas_wei as f64 / 1e18),
                    format!("   {}", wallet),
                    "2. Run quickstart again to confirm:".to_string(),
                    format!("   aave-v3-plugin {}quickstart {}", chain_flag, from_flag),
                ],
                format!("aave-v3-plugin {}quickstart {}", chain_flag, from_flag),
            )
        } else if has_gas && !has_tokens {
            (
                "needs_funds",
                "You have ETH for gas but no USDC or WETH to supply. Transfer tokens to your wallet.",
                vec![
                    "1. Send USDC or WETH to your wallet:".to_string(),
                    format!("   {}", wallet),
                    "2. Run quickstart again after funding:".to_string(),
                    format!("   aave-v3-plugin {}quickstart {}", chain_flag, from_flag),
                    "3. Browse available reserves:".to_string(),
                    format!("   aave-v3-plugin {}reserves", chain_flag),
                ],
                format!("aave-v3-plugin {}reserves", chain_flag),
            )
        } else {
            (
                "no_funds",
                "No ETH or tokens found. Send ETH (for gas) and USDC/WETH to get started.",
                vec![
                    "1. Send ETH (for gas) and USDC or WETH to your wallet:".to_string(),
                    format!("   {}", wallet),
                    format!("   Minimum gas: {:.4} ETH", min_gas_wei as f64 / 1e18),
                    "2. Run quickstart again:".to_string(),
                    format!("   aave-v3-plugin {}quickstart {}", chain_flag, from_flag),
                    "3. Browse available reserves and rates:".to_string(),
                    format!("   aave-v3-plugin {}reserves", chain_flag),
                ],
                format!("aave-v3-plugin {}quickstart {}", chain_flag, from_flag),
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": cfg.name,
        "chainId": chain_id,
        "assets": {
            "eth_balance": format!("{:.6}", eth_balance),
            "usdc_balance": format!("{:.2}", usdc_balance),
            "weth_balance": format!("{:.6}", weth_balance),
        },
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = json!(onboarding_steps);
    }

    // Include Aave position summary if user has active positions
    if let Ok(account_data) = &account_res {
        if account_data.total_collateral_base > 0 || account_data.total_debt_base > 0 {
            let hf_display = if account_data.health_factor >= u128::MAX / 2 {
                "no_debt".to_string()
            } else {
                format!("{:.4}", account_data.health_factor_f64())
            };
            let hf_status = if account_data.health_factor >= u128::MAX / 2 {
                "no_debt"
            } else {
                account_data.health_factor_status()
            };
            out["positions"] = json!({
                "healthFactor": hf_display,
                "healthFactorStatus": hf_status,
                "totalCollateralUSD": format!("{:.2}", account_data.total_collateral_usd()),
                "totalDebtUSD": format!("{:.2}", account_data.total_debt_usd()),
                "availableBorrowsUSD": format!("{:.2}", account_data.available_borrows_usd()),
            });
        }
    }

    Ok(out)
}
