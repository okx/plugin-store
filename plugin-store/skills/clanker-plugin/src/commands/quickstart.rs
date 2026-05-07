// commands/quickstart.rs — Clanker wallet-state onboarding
use crate::config;
use crate::onchainos;
use serde_json::{json, Value};

const ABOUT: &str = "Clanker is a permissionless token launcher on Base — deploy your own \
    ERC-20 token with a liquidity pool in seconds, then claim trading fees earned by your \
    token's pool.";

// Minimum ETH needed to deploy (covers factory gas on Base)
const MIN_DEPLOY_GAS_WEI: u128 = 1_000_000_000_000_000; // 0.001 ETH

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

pub async fn run(chain_id: u64) -> anyhow::Result<Value> {
    let rpc_url = config::rpc_url(chain_id);

    let chain_display = match chain_id {
        8453  => "Base",
        42161 => "Arbitrum",
        _     => "Base",
    };

    let wallet = onchainos::resolve_wallet(chain_id)
        .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {e}"))?;

    eprintln!(
        "Checking assets for {}... on {}...",
        &wallet[..10.min(wallet.len())],
        chain_display
    );

    let eth_wei = eth_balance_wei(&wallet, rpc_url).await;
    let eth_balance = eth_wei as f64 / 1e18;

    let has_eth = eth_wei > 0;
    let can_deploy = eth_wei >= MIN_DEPLOY_GAS_WEI;

    let chain_flag = if chain_id != 8453 {
        format!("--chain {} ", chain_id)
    } else {
        String::new()
    };

    let (status, suggestion, next_command, onboarding_steps): (&str, &str, String, Vec<String>) =
        if can_deploy {
            (
                "ready",
                "Your wallet has ETH to deploy a token on Base. Try listing recent launches or deploying your own token.",
                format!("clanker {}list-tokens --limit 5", chain_flag),
                vec![
                    "1. Browse recently deployed tokens for inspiration:".to_string(),
                    format!("   clanker {}list-tokens --limit 5", chain_flag),
                    "2. Preview your token deployment (safe — no tx sent):".to_string(),
                    format!("   clanker {}deploy-token --name \"MyToken\" --symbol \"MTK\" --from {}", chain_flag, wallet),
                    "3. Deploy your token (add --confirm):".to_string(),
                    format!("   clanker {}deploy-token --name \"MyToken\" --symbol \"MTK\" --from {} --confirm", chain_flag, wallet),
                    "4. After deployment, claim LP fees:".to_string(),
                    format!("   clanker {}claim-rewards --token-address <your-token> --from {} --confirm", chain_flag, wallet),
                ],
            )
        } else if has_eth && !can_deploy {
            (
                "needs_funds",
                "You have some ETH but may not have enough for deployment gas. Consider adding more ETH.",
                format!("clanker {}list-tokens --limit 5", chain_flag),
                vec![
                    format!("1. Send at least {:.4} ETH to your wallet for deployment gas:", MIN_DEPLOY_GAS_WEI as f64 / 1e18),
                    format!("   {}", wallet),
                    "2. Browse recent Clanker launches while you wait:".to_string(),
                    format!("   clanker {}list-tokens --limit 5", chain_flag),
                    "3. Run quickstart again after topping up:".to_string(),
                    format!("   clanker {}quickstart", chain_flag),
                ],
            )
        } else {
            (
                "no_funds",
                "No ETH found. Bridge ETH to Base to deploy a token.",
                format!("clanker {}list-tokens --limit 5", chain_flag),
                vec![
                    format!("1. Bridge or send ETH to your wallet on {}:", chain_display),
                    format!("   {}", wallet),
                    format!("   Minimum recommended: {:.4} ETH", MIN_DEPLOY_GAS_WEI as f64 / 1e18),
                    "2. Run quickstart again after funding:".to_string(),
                    format!("   clanker {}quickstart", chain_flag),
                    "3. While you wait, explore recent launches:".to_string(),
                    format!("   clanker {}list-tokens --limit 5", chain_flag),
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
            "eth_balance": format!("{:.6}", eth_balance),
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
