use serde_json::json;

const ABOUT: &str = "Velodrome V2 is the leading AMM DEX on Optimism — provide liquidity to volatile \
    or stable pools to earn trading fees and VELO rewards. $500M+ TVL.";

// USDC on Optimism (6 decimals)
const USDC_ADDRESS: &str = "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85";
// VELO on Optimism (18 decimals)
const VELO_ADDRESS: &str = "0x9560e827aF36c94D2Ac33a39bCE1Fe78631088Db";

// Minimum ETH needed for at least one swap tx on Optimism (~0.0001 ETH)
const MIN_ETH_READY_WEI: u128 = 100_000_000_000_000; // 0.0001 × 1e18
const MIN_ETH_GOOD_WEI: u128 = 10_000_000_000_000_000; // 0.01 × 1e18

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

pub async fn run() -> anyhow::Result<()> {
    let wallet = crate::onchainos::resolve_wallet(10)
        .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {e}"))?;

    if wallet.is_empty() {
        anyhow::bail!("No wallet found. Run: onchainos wallet login your@email.com");
    }

    eprintln!(
        "Checking assets for {}... on Optimism...",
        &wallet[..10.min(wallet.len())]
    );

    let rpc_url = crate::config::rpc_url();

    // Fetch balances in parallel
    let (eth_wei, usdc_raw, velo_raw) = tokio::join!(
        eth_balance_wei(&wallet, rpc_url),
        crate::rpc::get_balance(USDC_ADDRESS, &wallet, rpc_url),
        crate::rpc::get_balance(VELO_ADDRESS, &wallet, rpc_url),
    );

    let eth_wei = eth_wei;
    let usdc_raw = usdc_raw.unwrap_or(0);
    let velo_raw = velo_raw.unwrap_or(0);

    let eth_balance = eth_wei as f64 / 1e18;
    let usdc_balance = usdc_raw as f64 / 1_000_000.0;
    let velo_balance = velo_raw as f64 / 1e18;

    // active: has VELO (suggests earned rewards or LP participation)
    let is_active = velo_raw > 0;
    // ready: enough ETH for gas AND (has USDC or has significant ETH)
    let has_gas = eth_wei >= MIN_ETH_READY_WEI;
    let has_tokens = usdc_raw > 0 || eth_wei >= MIN_ETH_GOOD_WEI;
    let is_ready = has_gas && has_tokens;

    let (status, suggestion, onboarding_steps, next_command): (&str, &str, Vec<String>, String) =
        if is_active {
            (
                "active",
                "You have VELO tokens — likely an active LP or rewards earner. Check your LP positions and pending rewards.",
                vec![],
                "velodrome-v2-plugin positions".to_string(),
            )
        } else if is_ready {
            let example_token = if usdc_raw > 0 { "USDC" } else { "WETH" };
            let example_amount = if usdc_raw > 0 {
                format!("{:.2}", (usdc_balance * 0.5).max(1.0).min(usdc_balance))
            } else {
                format!("{:.4}", (eth_balance * 0.3).max(0.001).min(eth_balance - 0.001))
            };
            (
                "ready",
                "Your wallet is funded. Swap tokens or provide liquidity to earn fees and VELO rewards.",
                vec![
                    "1. Get a swap quote (no gas required):".to_string(),
                    format!("   velodrome-v2-plugin quote --token-in {} --token-out WETH --amount-in {}", example_token, example_amount),
                    "2. Preview swap (no tx sent):".to_string(),
                    format!("   velodrome-v2-plugin swap --token-in {} --token-out WETH --amount-in {} --slippage 0.5", example_token, example_amount),
                    "3. Execute swap:".to_string(),
                    format!("   velodrome-v2-plugin --confirm swap --token-in {} --token-out WETH --amount-in {} --slippage 0.5", example_token, example_amount),
                    "4. Browse pools for LP opportunities:".to_string(),
                    "   velodrome-v2-plugin pools --token-a WETH --token-b USDC".to_string(),
                ],
                format!("velodrome-v2-plugin quote --token-in {} --token-out WETH --amount-in {}", example_token, example_amount),
            )
        } else if has_gas && !has_tokens {
            (
                "needs_funds",
                "You have ETH for gas but need tokens to swap or provide liquidity.",
                vec![
                    "1. Bridge or transfer tokens to your Optimism wallet:".to_string(),
                    format!("   {}", wallet),
                    "   Recommended: USDC, WETH, or any Optimism token".to_string(),
                    "2. Run quickstart again:".to_string(),
                    "   velodrome-v2-plugin quickstart".to_string(),
                    "3. Browse available pools:".to_string(),
                    "   velodrome-v2-plugin pools --token-a WETH --token-b USDC".to_string(),
                ],
                "velodrome-v2-plugin pools --token-a WETH --token-b USDC".to_string(),
            )
        } else if !has_gas && has_tokens {
            (
                "needs_gas",
                "You have tokens but need ETH on Optimism for gas fees (very cheap, ~$0.01/tx).",
                vec![
                    "1. Send at least 0.0001 ETH (gas) to your Optimism wallet:".to_string(),
                    format!("   {}", wallet),
                    "2. Run quickstart again:".to_string(),
                    "   velodrome-v2-plugin quickstart".to_string(),
                ],
                "velodrome-v2-plugin quickstart".to_string(),
            )
        } else {
            (
                "no_funds",
                "Wallet is empty on Optimism. Bridge ETH and tokens to get started.",
                vec![
                    "1. Bridge ETH and tokens to Optimism:".to_string(),
                    format!("   Your Optimism address: {}", wallet),
                    "   Use the official Optimism bridge: https://app.optimism.io/bridge".to_string(),
                    "   Minimum: 0.001 ETH (gas) + tokens to swap or LP with".to_string(),
                    "2. Run quickstart again:".to_string(),
                    "   velodrome-v2-plugin quickstart".to_string(),
                    "3. Browse available pools:".to_string(),
                    "   velodrome-v2-plugin pools --token-a WETH --token-b USDC".to_string(),
                ],
                "velodrome-v2-plugin quickstart".to_string(),
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": "Optimism",
        "assets": {
            "eth_balance": format!("{:.6}", eth_balance),
            "usdc_balance": format!("{:.2}", usdc_balance),
            "velo_balance": format!("{:.4}", velo_balance),
        },
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = json!(onboarding_steps);
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
