use anyhow::Result;
use serde_json::Value;

use crate::api;
use crate::onchainos;

const ABOUT: &str = "Pendle Finance is a yield-trading protocol that splits yield-bearing tokens \
    into Principal Tokens (PT — fixed yield) and Yield Tokens (YT — floating yield). This skill \
    lets you browse markets, trade PT/YT, provide liquidity, and mint/redeem PT+YT pairs across \
    Ethereum, Arbitrum, BSC, and Base.";

// USDC (or equivalent stablecoin) per supported chain — the default trading asset.
fn usdc_address(chain_id: u64) -> Option<(&'static str, u32, &'static str)> {
    // (address, decimals, symbol)
    match chain_id {
        1     => Some(("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6,  "USDC")),
        42161 => Some(("0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6,  "USDC")),
        8453  => Some(("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", 6,  "USDC")),
        56    => Some(("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d", 18, "USDC")),
        _     => None,
    }
}

fn gas_symbol(chain_id: u64) -> &'static str {
    match chain_id {
        56 => "BNB",
        _  => "ETH",
    }
}

// Minimum thresholds: 0.0005 ETH covers a Pendle approve+swap on Arbitrum/Base; BSC uses BNB.
const MIN_GAS_WEI: u128  = 500_000_000_000_000; // 0.0005 native token (18 dec)
const MIN_USDC_USD: f64  = 5.0;                 // $5 minimum trade size

pub async fn run(
    user: Option<&str>,
    chain_id: u64,
    api_key: Option<&str>,
) -> Result<Value> {
    let wallet = match user {
        Some(addr) => {
            onchainos::validate_evm_address(addr)?;
            addr.to_string()
        }
        None => {
            let resolved = onchainos::resolve_wallet(chain_id)?;
            if resolved.is_empty() {
                anyhow::bail!(
                    "Cannot resolve wallet address. Pass --user or ensure onchainos is logged in."
                );
            }
            resolved
        }
    };

    eprintln!(
        "Checking assets for {}...",
        &wallet[..std::cmp::min(10, wallet.len())]
    );

    let (stable_addr, stable_decimals, stable_symbol) = usdc_address(chain_id)
        .unwrap_or(("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6, "USDC"));

    // Three-way parallel fetch: native gas, stablecoin balance, Pendle positions.
    let (gas_result, stable_result, positions_result) = tokio::join!(
        native_balance(chain_id, &wallet),
        onchainos::erc20_balance_of(chain_id, stable_addr, &wallet),
        api::get_positions(&wallet, Some(0.01), api_key),
    );

    let gas_wei = gas_result.unwrap_or(0);
    let stable_raw = stable_result.unwrap_or(0);
    let positions_count = positions_result
        .as_ref()
        .ok()
        .and_then(count_positions)
        .unwrap_or(0);

    let gas = gas_wei as f64 / 1e18;
    let stable = stable_raw as f64 / 10f64.powi(stable_decimals as i32);

    let (status, suggestion, onboarding_steps, next_command) = build_suggestion(
        &wallet,
        chain_id,
        gas_wei,
        stable,
        positions_count,
        stable_symbol,
        stable_addr,
    );

    let mut out = serde_json::json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": chain_id,
        "assets": {
            "gas_symbol":        gas_symbol(chain_id),
            "gas_balance":       format!("{:.6}", gas),
            "stable_symbol":     stable_symbol,
            "stable_balance":    format!("{:.4}", stable),
            "active_positions":  positions_count,
        },
        "status":       status,
        "suggestion":   suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = serde_json::json!(onboarding_steps);
    }

    Ok(out)
}

/// Query native token balance via eth_getBalance on the chain's public RPC.
/// Returns 0 on any RPC error — quickstart is best-effort read-only guidance.
async fn native_balance(chain_id: u64, wallet: &str) -> Result<u128> {
    let rpc_url = onchainos::default_rpc_url(chain_id);
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [wallet, "latest"],
        "id": 1
    });
    let resp: Value = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    let hex = resp["result"].as_str().unwrap_or("0x0");
    let clean = hex.trim_start_matches("0x");
    if clean.is_empty() {
        return Ok(0);
    }
    let truncated = if clean.len() > 32 {
        &clean[clean.len() - 32..]
    } else {
        clean
    };
    Ok(u128::from_str_radix(truncated, 16).unwrap_or(0))
}

/// Count Pendle positions from the API response. The dashboard API shape has varied
/// across versions — probe several common paths and return 0 if none match.
fn count_positions(resp: &Value) -> Option<usize> {
    for key in &["openPositions", "positions", "data", "results"] {
        if let Some(arr) = resp[*key].as_array() {
            return Some(arr.len());
        }
    }
    if let Some(n) = resp["totalOpen"].as_u64() {
        return Some(n as usize);
    }
    if let Some(n) = resp["total"].as_u64() {
        return Some(n as usize);
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn build_suggestion(
    wallet: &str,
    chain_id: u64,
    gas_wei: u128,
    stable: f64,
    positions_count: usize,
    stable_symbol: &'static str,
    stable_addr: &'static str,
) -> (&'static str, String, Vec<String>, String) {
    let gas = gas_symbol(chain_id);

    // Case 1: active — user already has Pendle positions
    if positions_count > 0 {
        return (
            "active",
            format!(
                "You have {} active Pendle position(s). Review them below.",
                positions_count
            ),
            vec![],
            format!("pendle-plugin --chain {} get-positions", chain_id),
        );
    }

    // Case 2: ready — has gas + trading asset
    if gas_wei >= MIN_GAS_WEI && stable >= MIN_USDC_USD {
        return (
            "ready",
            "Your wallet is funded. Browse active Pendle markets to find a yield opportunity."
                .to_string(),
            vec![
                "1. Browse the top active markets (high TVL, high APY):".to_string(),
                format!("   pendle-plugin --chain {} list-markets --active-only --limit 10", chain_id),
                "2. Or search by asset (e.g. weETH, wstETH):".to_string(),
                format!("   pendle-plugin --chain {} list-markets --search weETH --active-only", chain_id),
                "3. Preview buying PT for fixed yield (no --confirm = preview only):".to_string(),
                format!(
                    "   pendle-plugin --chain {} buy-pt --token-in {} --amount-in 5000000 --pt-address <PT_ADDR>",
                    chain_id, stable_addr
                ),
                "4. Re-run with --confirm to execute.".to_string(),
            ],
            format!(
                "pendle-plugin --chain {} list-markets --active-only --limit 10",
                chain_id
            ),
        );
    }

    // Case 3: has stable but no gas
    if stable >= MIN_USDC_USD {
        return (
            "needs_gas",
            format!(
                "You have {} but need {} for gas. Send at least 0.0005 {} to your wallet.",
                stable_symbol, gas, gas
            ),
            vec![
                format!("1. Send at least 0.0005 {} for gas to your wallet:", gas),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm:".to_string(),
                format!("   pendle-plugin --chain {} quickstart", chain_id),
            ],
            format!("pendle-plugin --chain {} quickstart", chain_id),
        );
    }

    // Case 4: has gas but no stable
    if gas_wei >= MIN_GAS_WEI {
        return (
            "needs_funds",
            format!(
                "You have {} for gas but need a trading asset. Send at least $5 {} to your wallet.",
                gas, stable_symbol
            ),
            vec![
                format!("1. Send at least 5 {} to your wallet:", stable_symbol),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm:".to_string(),
                format!("   pendle-plugin --chain {} quickstart", chain_id),
                "3. Then browse markets:".to_string(),
                format!(
                    "   pendle-plugin --chain {} list-markets --active-only --limit 10",
                    chain_id
                ),
            ],
            format!("pendle-plugin --chain {} quickstart", chain_id),
        );
    }

    // Case 5: no funds
    (
        "no_funds",
        format!(
            "No {} or {} found. Send both to your wallet to get started.",
            gas, stable_symbol
        ),
        vec![
            format!(
                "1. Send {} (at least 0.0005) and {} (at least 5) to your wallet:",
                gas, stable_symbol
            ),
            format!("   {}", wallet),
            "2. Run quickstart again to confirm:".to_string(),
            format!("   pendle-plugin --chain {} quickstart", chain_id),
            "3. Browse markets to find a yield opportunity:".to_string(),
            format!(
                "   pendle-plugin --chain {} list-markets --active-only --limit 10",
                chain_id
            ),
        ],
        format!("pendle-plugin --chain {} quickstart", chain_id),
    )
}
