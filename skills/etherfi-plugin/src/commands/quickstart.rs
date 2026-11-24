use serde_json::json;

const ABOUT: &str = "ether.fi is a decentralized liquid staking and restaking protocol — stake ETH \
    to receive eETH (liquid staking token with native restaking yield) or wrap to weETH for DeFi \
    compatibility. $10B+ TVL.";

// Minimum ETH for gas to be considered "ready": 0.005 ETH
const MIN_ETH_READY_WEI: u128 = 5_000_000_000_000_000; // 0.005 × 1e18

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

pub async fn run(from: Option<&str>) -> anyhow::Result<()> {
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        crate::onchainos::resolve_wallet(crate::config::CHAIN_ID)
            .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {e}"))?
    };

    if wallet.is_empty() {
        anyhow::bail!("No wallet found. Run: onchainos wallet login your@email.com");
    }

    eprintln!(
        "Checking assets for {}... on Ethereum...",
        &wallet[..10.min(wallet.len())]
    );

    let rpc_url = crate::config::rpc_url();

    // Fetch balances in parallel
    let (eth_wei, eeth_raw, weeth_raw) = tokio::join!(
        eth_balance_wei(&wallet, rpc_url),
        crate::rpc::get_balance(crate::config::eeth_address(), &wallet, rpc_url),
        crate::rpc::get_balance(crate::config::weeth_address(), &wallet, rpc_url),
    );

    let eth_wei = eth_wei;
    let eeth_raw = eeth_raw.unwrap_or(0);
    let weeth_raw = weeth_raw.unwrap_or(0);

    let eth_balance = eth_wei as f64 / 1e18;
    let eeth_balance = eeth_raw as f64 / 1e18;
    let weeth_balance = weeth_raw as f64 / 1e18;

    let has_staked = eeth_raw > 0 || weeth_raw > 0;
    let has_gas = eth_wei >= MIN_ETH_READY_WEI;

    let (status, suggestion, onboarding_steps, next_command): (&str, &str, Vec<String>, String) =
        if has_staked {
            (
                "active",
                "You have an active ether.fi position. Check your eETH/weETH balances and restaking yield.",
                vec![],
                "etherfi-plugin positions".to_string(),
            )
        } else if has_gas {
            (
                "ready",
                "Your wallet has ETH. Stake to receive eETH and start earning restaking yield.",
                vec![
                    "1. View current positions and APY:".to_string(),
                    "   etherfi-plugin positions".to_string(),
                    "2. Preview stake (no tx sent):".to_string(),
                    format!("   etherfi-plugin stake --amount {:.4}", (eth_balance * 0.5).max(0.001).min(eth_balance - 0.003)),
                    "3. Execute stake:".to_string(),
                    format!(
                        "   etherfi-plugin --confirm stake --amount {:.4}",
                        (eth_balance * 0.5).max(0.001).min(eth_balance - 0.003)
                    ),
                    "4. Optionally wrap eETH → weETH for auto-compounding:".to_string(),
                    "   etherfi-plugin --confirm wrap --amount <eETH_AMOUNT>".to_string(),
                ],
                "etherfi-plugin positions".to_string(),
            )
        } else if !has_gas && has_staked {
            (
                "needs_gas",
                "You have eETH/weETH but need ETH for gas fees. Send ETH to your wallet.",
                vec![
                    "1. Send at least 0.005 ETH (gas) to:".to_string(),
                    format!("   {}", wallet),
                    "2. Run quickstart again:".to_string(),
                    "   etherfi-plugin quickstart".to_string(),
                ],
                "etherfi-plugin quickstart".to_string(),
            )
        } else {
            (
                "no_funds",
                "No ETH found. Send ETH to your wallet on Ethereum mainnet to start restaking.",
                vec![
                    "1. Send ETH to your wallet on Ethereum mainnet:".to_string(),
                    format!("   {}", wallet),
                    "   Minimum: 0.001 ETH (protocol minimum) + gas (~0.00005 ETH/tx)".to_string(),
                    "2. Run quickstart again:".to_string(),
                    "   etherfi-plugin quickstart".to_string(),
                    "3. View current positions and APY:".to_string(),
                    "   etherfi-plugin positions".to_string(),
                ],
                "etherfi-plugin quickstart".to_string(),
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": "Ethereum",
        "assets": {
            "eth_balance": format!("{:.6}", eth_balance),
            "eeth_balance": format!("{:.6}", eeth_balance),
            "weeth_balance": format!("{:.6}", weeth_balance),
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
