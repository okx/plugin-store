// commands/quickstart.rs — Curve Finance wallet-state onboarding
use crate::config;
use crate::onchainos;
use crate::rpc;
use serde_json::{json, Value};

const ABOUT: &str = "Curve Finance is the leading stablecoin and LST DEX — swap between \
    stablecoins with minimal slippage, provide liquidity to earn trading fees and CRV rewards. \
    $3B+ TVL.";

// Canonical stablecoin addresses on Ethereum (chain 1)
const USDC_ETHEREUM: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT_ETHEREUM: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

// USDC on other chains
const USDC_ARBITRUM: &str = "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8";
const USDC_BASE: &str    = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const USDC_POLYGON: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
const USDC_BSC: &str     = "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d";

// USDT on other chains
const USDT_ARBITRUM: &str = "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9";
const USDT_BSC: &str     = "0x55d398326f99059fF775485246999027B3197955";
const USDT_POLYGON: &str = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F";

// Minimum native gas needed
const MIN_GAS_ETHEREUM_WEI: u128 = 3_000_000_000_000_000; // 0.003 ETH
const MIN_GAS_L2_WEI: u128      = 100_000_000_000_000;     // 0.0001 ETH (L2)

// Minimum meaningful stablecoin balance (1 USDC/USDT = 1_000_000 raw)
const MIN_STABLE_RAW: u128 = 1_000_000; // 1 USDC/USDT (6 decimals)

async fn eth_balance_wei(wallet: &str, rpc_url: &str) -> u128 {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [wallet, "latest"],
        "id": 1
    });
    match client.post(rpc_url).json(&body).send().await {
        Ok(resp) => {
            match resp.json::<serde_json::Value>().await {
                Ok(val) => val["result"].as_str()
                    .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                    .unwrap_or(0),
                Err(_) => 0,
            }
        }
        Err(_) => 0,
    }
}

pub async fn run(chain_id: u64, wallet: Option<&str>) -> anyhow::Result<Value> {
    let rpc_url = config::rpc_url(chain_id);

    let chain_display = match chain_id {
        1     => "Ethereum",
        56    => "BSC",
        137   => "Polygon",
        8453  => "Base",
        42161 => "Arbitrum",
        _     => "Ethereum",
    };

    let resolved = if let Some(addr) = wallet {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)
            .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {e}"))?
    };

    eprintln!(
        "Checking assets for {}... on {}...",
        &resolved[..10.min(resolved.len())],
        chain_display
    );

    // Choose stablecoin addresses for this chain
    let (usdc_addr, usdt_addr) = match chain_id {
        1     => (USDC_ETHEREUM, USDT_ETHEREUM),
        42161 => (USDC_ARBITRUM, USDT_ARBITRUM),
        8453  => (USDC_BASE,     USDC_BASE),  // no native USDT on Base; use USDC twice
        137   => (USDC_POLYGON,  USDT_POLYGON),
        56    => (USDC_BSC,      USDT_BSC),
        _     => (USDC_ETHEREUM, USDT_ETHEREUM),
    };

    let min_gas_wei = if chain_id == 1 { MIN_GAS_ETHEREUM_WEI } else { MIN_GAS_L2_WEI };

    let (eth_wei, usdc_raw, usdt_raw) = tokio::join!(
        eth_balance_wei(&resolved, rpc_url),
        rpc::balance_of(usdc_addr, &resolved, rpc_url),
        rpc::balance_of(usdt_addr, &resolved, rpc_url),
    );

    let usdc_raw = usdc_raw.unwrap_or(0);
    let usdt_raw = usdt_raw.unwrap_or(0);

    let eth_balance  = eth_wei  as f64 / 1e18;
    let usdc_balance = usdc_raw as f64 / 1_000_000.0;
    let usdt_balance = usdt_raw as f64 / 1_000_000.0;

    let has_gas    = eth_wei >= min_gas_wei;
    let has_tokens = usdc_raw >= MIN_STABLE_RAW || usdt_raw >= MIN_STABLE_RAW;

    let chain_flag = if chain_id != 1 {
        format!("--chain {} ", chain_id)
    } else {
        String::new()
    };
    let wallet_flag = format!("--wallet {}", resolved);

    let (status, suggestion, next_command, onboarding_steps): (&str, &str, String, Vec<String>) =
        if has_gas && has_tokens {
            let (token, amount) = if usdc_balance >= 1.0 {
                ("USDC", format!("{:.2}", (usdc_balance * 0.9).max(1.0).min(usdc_balance)))
            } else {
                ("USDT", format!("{:.2}", (usdt_balance * 0.9).max(1.0).min(usdt_balance)))
            };
            (
                "ready",
                "Your wallet is funded. Use Curve to swap stablecoins with minimal slippage.",
                format!("curve {}get-pools", chain_flag),
                vec![
                    "1. Browse available pools:".to_string(),
                    format!("   curve {}get-pools", chain_flag),
                    "2. Get a swap quote:".to_string(),
                    format!("   curve {}quote --token-in {} --token-out DAI --amount {}", chain_flag, token, amount),
                    "3. Preview swap (no --confirm = safe):".to_string(),
                    format!("   curve {}swap --token-in {} --token-out DAI --amount {} {}", chain_flag, token, amount, wallet_flag),
                    "4. Execute swap (add --confirm):".to_string(),
                    format!("   curve {}--confirm swap --token-in {} --token-out DAI --amount {} {}", chain_flag, token, amount, wallet_flag),
                ],
            )
        } else if has_tokens && !has_gas {
            (
                "needs_gas",
                "You have stablecoins but need ETH for gas fees. Send ETH to your wallet.",
                format!("curve {}quickstart {}", chain_flag, wallet_flag),
                vec![
                    format!("1. Send at least {:.4} ETH (gas) to:", min_gas_wei as f64 / 1e18),
                    format!("   {}", resolved),
                    "2. Run quickstart again:".to_string(),
                    format!("   curve {}quickstart {}", chain_flag, wallet_flag),
                ],
            )
        } else if has_gas && !has_tokens {
            (
                "needs_funds",
                "You have ETH for gas but no USDC/USDT to swap. Transfer stablecoins to your wallet.",
                format!("curve {}get-pools", chain_flag),
                vec![
                    "1. Send USDC or USDT to your wallet:".to_string(),
                    format!("   {}", resolved),
                    "2. Run quickstart again after funding:".to_string(),
                    format!("   curve {}quickstart {}", chain_flag, wallet_flag),
                    "3. Browse available pools:".to_string(),
                    format!("   curve {}get-pools", chain_flag),
                ],
            )
        } else {
            (
                "no_funds",
                "No ETH or tokens found. Send ETH (for gas) and stablecoins to get started.",
                format!("curve {}get-pools", chain_flag),
                vec![
                    "1. Send ETH (for gas) and USDC or USDT to your wallet:".to_string(),
                    format!("   {}", resolved),
                    format!("   Minimum gas: {:.4} ETH", min_gas_wei as f64 / 1e18),
                    "2. Run quickstart again:".to_string(),
                    format!("   curve {}quickstart {}", chain_flag, wallet_flag),
                    "3. Browse available pools and rates:".to_string(),
                    format!("   curve {}get-pools", chain_flag),
                ],
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": resolved,
        "chain": chain_display,
        "chainId": chain_id,
        "assets": {
            "eth_balance": format!("{:.6}", eth_balance),
            "usdc_balance": format!("{:.2}", usdc_balance),
            "usdt_balance": format!("{:.2}", usdt_balance),
        },
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = json!(onboarding_steps);
    }

    Ok(out)
}
