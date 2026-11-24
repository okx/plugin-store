// commands/quickstart.rs — PancakeSwap V2 wallet-state onboarding
use crate::config;
use crate::onchainos;
use serde_json::{json, Value};

const ABOUT: &str = "PancakeSwap V2 is the leading DEX on BSC — swap tokens and provide \
    liquidity to earn trading fees and CAKE rewards. $2B+ TVL.";

// USDT on BSC, USDC on Base
const USDT_BSC: &str  = "0x55d398326f99059fF775485246999027B3197955";
const USDC_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

// Minimum native gas needed
const MIN_GAS_BSC_WEI: u128  = 1_000_000_000_000_000; // 0.001 BNB
const MIN_GAS_L2_WEI: u128   = 100_000_000_000_000;    // 0.0001 ETH

// Minimum meaningful token balance (1 USDT/USDC = 1_000_000 raw, 6 decimals)
const MIN_TOKEN_RAW: u128 = 1_000_000;

async fn native_balance_wei(wallet: &str, rpc_url: &str) -> u128 {
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

async fn erc20_balance_of(token: &str, owner: &str, rpc_url: &str) -> u128 {
    let owner_clean = owner.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);
    let data = format!("0x70a08231{}", owner_padded);

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": token, "data": data }, "latest"],
        "id": 1
    });
    match client.post(rpc_url).json(&body).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(val) => val["result"].as_str()
                .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

pub async fn run(chain_id: u64) -> anyhow::Result<Value> {
    let cfg = config::chain_config(chain_id)?;

    let (chain_display, native_symbol, token_addr, token_symbol) = match chain_id {
        56   => ("BSC",  "BNB", USDT_BSC,  "USDT"),
        8453 => ("Base", "ETH", USDC_BASE, "USDC"),
        _    => ("BSC",  "BNB", USDT_BSC,  "USDT"),
    };

    let min_gas_wei = if chain_id == 56 { MIN_GAS_BSC_WEI } else { MIN_GAS_L2_WEI };

    let wallet = onchainos::resolve_wallet(chain_id)
        .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {e}"))?;

    eprintln!(
        "Checking assets for {}... on {}...",
        &wallet[..10.min(wallet.len())],
        chain_display
    );

    let (native_wei, token_raw) = tokio::join!(
        native_balance_wei(&wallet, cfg.rpc_url),
        erc20_balance_of(token_addr, &wallet, cfg.rpc_url),
    );

    let native_balance = native_wei  as f64 / 1e18;
    let token_balance  = token_raw   as f64 / 1_000_000.0;

    let has_gas    = native_wei >= min_gas_wei;
    let has_tokens = token_raw  >= MIN_TOKEN_RAW;

    let chain_flag = if chain_id != 56 {
        format!("--chain {} ", chain_id)
    } else {
        String::new()
    };

    let (status, suggestion, next_command, onboarding_steps): (&str, &str, String, Vec<String>) =
        if has_gas && has_tokens {
            let example_amount = format!("{:.2}", (token_balance * 0.9).max(1.0).min(token_balance));
            (
                "ready",
                "Your wallet is funded. Swap tokens or provide liquidity on PancakeSwap V2.",
                format!("pancakeswap-v2 {}quote --token-in {} --token-out {} --amount-in {}", chain_flag, token_symbol, native_symbol, example_amount),
                vec![
                    "1. Get a swap quote:".to_string(),
                    format!("   pancakeswap-v2 {}quote --token-in {} --token-out {} --amount-in {}", chain_flag, token_symbol, native_symbol, example_amount),
                    "2. Preview swap (no --confirm = safe):".to_string(),
                    format!("   pancakeswap-v2 {}swap --token-in {} --token-out {} --amount-in {}", chain_flag, token_symbol, native_symbol, example_amount),
                    "3. Execute swap (add --confirm):".to_string(),
                    format!("   pancakeswap-v2 {}--confirm swap --token-in {} --token-out {} --amount-in {}", chain_flag, token_symbol, native_symbol, example_amount),
                ],
            )
        } else if has_gas && !has_tokens {
            (
                "needs_funds",
                &format!("You have {} for gas but no {}. Transfer tokens to your wallet.", native_symbol, token_symbol),
                format!("pancakeswap-v2 {}quote --token-in {} --token-out {} --amount-in 1", chain_flag, native_symbol, token_symbol),
                vec![
                    format!("1. Send {} or other tokens to your wallet:", token_symbol),
                    format!("   {}", wallet),
                    "2. Run quickstart again after funding:".to_string(),
                    format!("   pancakeswap-v2 {}quickstart", chain_flag),
                    format!("3. Or swap your {} for {}:", native_symbol, token_symbol),
                    format!("   pancakeswap-v2 {}quote --token-in {} --token-out {} --amount-in 1", chain_flag, native_symbol, token_symbol),
                ],
            )
        } else if has_tokens && !has_gas {
            (
                "needs_gas",
                &format!("You have tokens but need {} for gas fees. Send {} to your wallet.", native_symbol, native_symbol),
                format!("pancakeswap-v2 {}quickstart", chain_flag),
                vec![
                    format!("1. Send at least {:.4} {} (gas) to:", min_gas_wei as f64 / 1e18, native_symbol),
                    format!("   {}", wallet),
                    "2. Run quickstart again:".to_string(),
                    format!("   pancakeswap-v2 {}quickstart", chain_flag),
                ],
            )
        } else {
            (
                "no_funds",
                &format!("Wallet empty. Send {} (gas) and {} to get started.", native_symbol, token_symbol),
                format!("pancakeswap-v2 {}quickstart", chain_flag),
                vec![
                    format!("1. Send {} (for gas) and {} to your wallet:", native_symbol, token_symbol),
                    format!("   {}", wallet),
                    format!("   Minimum gas: {:.4} {}", min_gas_wei as f64 / 1e18, native_symbol),
                    "2. Run quickstart again:".to_string(),
                    format!("   pancakeswap-v2 {}quickstart", chain_flag),
                ],
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": chain_display,
        "chainId": chain_id,
        "assets": {
            "native_balance": format!("{:.6}", native_balance),
            "native_symbol": native_symbol,
            "token_balance": format!("{:.2}", token_balance),
            "token_symbol": token_symbol,
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
