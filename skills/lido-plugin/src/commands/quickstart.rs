use serde_json::json;

const ABOUT: &str = "Lido is the leading liquid staking protocol on Ethereum — stake ETH to \
    receive stETH (rebasing yield token) or wstETH (wrapped, compatible with DeFi). $35B+ TVL.";

const RPC_URL: &str = "https://ethereum.publicnode.com";

// stETH balance threshold for "active" classification: 0.001 stETH
const MIN_STETH_ACTIVE: u128 = 1_000_000_000_000_000; // 0.001 × 1e18

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

pub async fn run() -> anyhow::Result<()> {
    let wallet = crate::onchainos::resolve_wallet(1).await
        .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {e}"))?;

    if wallet.is_empty() {
        anyhow::bail!("No wallet found. Run: onchainos wallet login your@email.com");
    }

    eprintln!(
        "Checking assets for {}... on Ethereum...",
        &wallet[..10.min(wallet.len())]
    );

    // Fetch balances in parallel
    let (eth_wei, steth_raw, wsteth_raw) = tokio::join!(
        eth_balance_wei(&wallet, RPC_URL),
        async {
            let calldata = crate::rpc::calldata_single_address(
                crate::config::SEL_BALANCE_OF,
                &wallet,
            );
            match crate::onchainos::eth_call(1, crate::config::STETH_ADDRESS, &calldata).await {
                Ok(val) => {
                    match crate::rpc::extract_return_data(&val) {
                        Ok(hex) => crate::rpc::decode_uint256(&hex).unwrap_or(0),
                        Err(_) => 0,
                    }
                }
                Err(_) => 0,
            }
        },
        async {
            let calldata = crate::rpc::calldata_single_address(
                crate::config::SEL_BALANCE_OF,
                &wallet,
            );
            match crate::onchainos::eth_call(1, crate::config::WSTETH_ADDRESS, &calldata).await {
                Ok(val) => {
                    match crate::rpc::extract_return_data(&val) {
                        Ok(hex) => crate::rpc::decode_uint256(&hex).unwrap_or(0),
                        Err(_) => 0,
                    }
                }
                Err(_) => 0,
            }
        },
    );

    let eth_balance = eth_wei as f64 / 1e18;
    let steth_balance = steth_raw as f64 / 1e18;
    let wsteth_balance = wsteth_raw as f64 / 1e18;

    let has_staked = steth_raw >= MIN_STETH_ACTIVE || wsteth_raw >= MIN_STETH_ACTIVE;
    let has_gas = eth_wei >= MIN_ETH_READY_WEI;

    let (status, suggestion, onboarding_steps, next_command): (&str, &str, Vec<String>, String) =
        if has_staked {
            (
                "active",
                "You have an active Lido position. Check your stETH balance and accruing rewards.",
                vec![],
                format!("lido-plugin balance"),
            )
        } else if has_gas {
            (
                "ready",
                "Your wallet has ETH. Stake to receive stETH and start earning yield.",
                vec![
                    "1. Check the current staking APR:".to_string(),
                    "   lido-plugin get-apy".to_string(),
                    "2. Preview stake (no tx sent):".to_string(),
                    format!("   lido-plugin stake --amount-eth {:.4}", (eth_balance * 0.5).max(0.01)),
                    "3. Execute stake:".to_string(),
                    format!(
                        "   lido-plugin --confirm stake --amount-eth {:.4}",
                        (eth_balance * 0.5).max(0.01)
                    ),
                    "4. Check your stETH balance:".to_string(),
                    "   lido-plugin balance".to_string(),
                ],
                "lido-plugin get-apy".to_string(),
            )
        } else if !has_gas && (steth_raw > 0 || wsteth_raw > 0) {
            // Has some tokens but below active threshold, no gas
            (
                "needs_gas",
                "You have stETH/wstETH but need more ETH for gas. Send ETH to your wallet.",
                vec![
                    "1. Send at least 0.005 ETH (gas) to:".to_string(),
                    format!("   {}", wallet),
                    "2. Run quickstart again:".to_string(),
                    "   lido-plugin quickstart".to_string(),
                ],
                "lido-plugin quickstart".to_string(),
            )
        } else {
            (
                "no_funds",
                "No ETH found. Send ETH to your wallet on Ethereum mainnet to start staking.",
                vec![
                    "1. Send ETH to your wallet on Ethereum mainnet:".to_string(),
                    format!("   {}", wallet),
                    "   Minimum recommended: 0.01 ETH (covers gas + meaningful stake)".to_string(),
                    "2. Run quickstart again:".to_string(),
                    "   lido-plugin quickstart".to_string(),
                    "3. Check the current APR:".to_string(),
                    "   lido-plugin get-apy".to_string(),
                ],
                "lido-plugin quickstart".to_string(),
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": "Ethereum",
        "assets": {
            "eth_balance": format!("{:.6}", eth_balance),
            "steth_balance": format!("{:.6}", steth_balance),
            "wsteth_balance": format!("{:.6}", wsteth_balance),
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
