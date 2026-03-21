//! Thin wrapper around the `onchainos` CLI binary (used as an SDK).
//!
//! We shell out to the already-installed `onchainos` CLI for all wallet
//! operations. The CLI handles: auth / token refresh / TEE signing / broadcast.
//!
//! We NEVER read onchainos internal files directly — only call its CLI commands.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::process::Command;
use std::sync::OnceLock;

/// Cached EVM address from `onchainos wallet balance`.
/// Once resolved, the address won't change within a process lifetime.
static CACHED_EVM_ADDRESS: OnceLock<String> = OnceLock::new();

/// Cached Solana address from `onchainos wallet balance`.
static CACHED_SOL_ADDRESS: OnceLock<String> = OnceLock::new();

/// Chain mapping for `wallet` commands (wallet balance, wallet contract-call).
///
/// onchainos wallet uses internal chain names (from `onchainos wallet chains`):
///   Ethereum = "eth", Base = "base_eth", Arbitrum = "arb_eth", Polygon = "matic", Solana = "sol"
pub fn chain_flag(chain_name: &str) -> &str {
    match chain_name.to_lowercase().as_str() {
        "base" | "base_eth" | "8453" => "base_eth",
        "ethereum" | "eth" | "1" => "eth",
        "arbitrum" | "arb" | "arb_eth" | "42161" => "arb_eth",
        "polygon" | "matic" | "137" => "matic",
        "solana" | "sol" | "501" => "sol",
        _ => chain_name,
    }
}

/// Chain name for non-wallet commands (swap, token, market, memepump, signal, portfolio, gateway).
///
/// These commands accept human-readable names: "base", "solana", "ethereum", "arbitrum", etc.
fn api_chain(chain_name: &str) -> &str {
    match chain_name.to_lowercase().as_str() {
        "base" | "base_eth" | "8453" => "base",
        "ethereum" | "eth" | "1" => "ethereum",
        "arbitrum" | "arb" | "arb_eth" | "42161" => "arbitrum",
        "polygon" | "matic" | "137" => "polygon",
        "solana" | "sol" | "501" => "solana",
        _ => chain_name,
    }
}

// ── Internal helpers ────────────────────────────────────────────────

/// Run an onchainos CLI command and return parsed JSON output.
fn run_onchainos(args: &[&str]) -> Result<Value> {
    let output = Command::new("onchainos")
        .args(args)
        .output()
        .context("failed to execute onchainos CLI — is it installed and in PATH?")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
            if let Some(err) = json["error"].as_str() {
                bail!("onchainos error: {}", err);
            }
        }
        bail!(
            "onchainos failed (exit {}): {}{}",
            output.status,
            stdout,
            stderr
        );
    }

    let json: Value =
        serde_json::from_str(&stdout).context("failed to parse onchainos JSON output")?;

    if json["ok"].as_bool() != Some(true) {
        let err = json["error"].as_str().unwrap_or("unknown error");
        bail!("onchainos error: {}", err);
    }

    Ok(json["data"].clone())
}

// ── Public API ──────────────────────────────────────────────────────

/// Check if onchainos wallet CLI is available and the user is logged in.
///
/// Uses `onchainos wallet status` — lightweight, no network call for balances.
pub fn is_available() -> bool {
    match run_onchainos(&["wallet", "status"]) {
        Ok(data) => data["loggedIn"].as_bool() == Some(true),
        Err(_) => false,
    }
}

/// Get the EVM address for the current onchainos wallet account.
///
/// Calls `onchainos wallet balance` (which returns `evmAddress` at the top level)
/// and caches the result for the process lifetime.
pub fn get_evm_address() -> Result<String> {
    // Return cached value if available
    if let Some(addr) = CACHED_EVM_ADDRESS.get() {
        return Ok(addr.clone());
    }

    let data = run_onchainos(&["wallet", "balance"])?;

    // Response: { "evmAddress": "0x...", "solAddress": "...", "details": [...], ... }
    let addr = data["evmAddress"]
        .as_str()
        .filter(|a| !a.is_empty() && a.starts_with("0x"))
        .context("onchainos wallet balance: missing or invalid evmAddress")?;

    let _ = CACHED_EVM_ADDRESS.set(addr.to_string());
    Ok(addr.to_string())
}

/// Get the Solana address for the current onchainos wallet account.
///
/// Calls `onchainos wallet balance` (which returns `solAddress` at the top level)
/// and caches the result for the process lifetime.
pub fn get_sol_address() -> Result<String> {
    // Return cached value if available
    if let Some(addr) = CACHED_SOL_ADDRESS.get() {
        return Ok(addr.clone());
    }

    let data = run_onchainos(&["wallet", "balance"])?;

    // Response: { "evmAddress": "0x...", "solAddress": "...", "details": [...], ... }
    let addr = data["solAddress"]
        .as_str()
        .filter(|a| !a.is_empty())
        .context("onchainos wallet balance: missing or invalid solAddress")?;

    let _ = CACHED_SOL_ADDRESS.set(addr.to_string());
    Ok(addr.to_string())
}

/// Query token balance via `onchainos wallet balance --chain <chain>`.
///
/// Returns a list of `{ symbol, balance, tokenAddress, usdValue }` for the given chain.
/// Returns `Err` if the onchainos balance query fails (caller should fall back to RPC).
pub fn get_token_balances(chain: &str) -> Result<Vec<TokenBalance>> {
    let chain = chain_flag(chain);
    let data = run_onchainos(&["wallet", "balance", "--chain", chain])?;

    let mut balances = Vec::new();

    // Response: { "totalValueUsd": "...", "details": [ { "tokenAssets": [ ... ] } ] }
    let details = data["details"]
        .as_array()
        .or_else(|| data.as_array())
        .context("onchainos balance: unexpected response format")?;

    for group in details {
        let assets = group["tokenAssets"]
            .as_array()
            .or_else(|| group["assets"].as_array());
        if let Some(assets) = assets {
            for asset in assets {
                let symbol = asset["symbol"].as_str().unwrap_or("").to_string();
                let balance_str = asset["balance"].as_str().unwrap_or("0");
                let balance: f64 = balance_str.parse().unwrap_or(0.0);
                let token_address = asset["tokenAddress"].as_str().unwrap_or("").to_string();
                let usd_value_str = asset["usdValue"]
                    .as_str()
                    .or_else(|| asset["usdValue"].as_f64().map(|_| ""))
                    .unwrap_or("0");
                let usd_value: f64 = usd_value_str.parse().unwrap_or(0.0);

                if balance > 0.0 {
                    balances.push(TokenBalance {
                        symbol,
                        balance,
                        token_address,
                        usd_value,
                    });
                }
            }
        }
    }

    Ok(balances)
}

/// A token balance entry from onchainos wallet balance.
pub struct TokenBalance {
    pub symbol: String,
    pub balance: f64,
    pub token_address: String,
    pub usd_value: f64,
}

/// Get gas price via `onchainos gateway gas --chain <chain>`.
///
/// Returns the "normal" gas price in wei. Falls back to Err if onchainos fails.
pub fn get_gas_price(chain: &str) -> Result<u128> {
    let chain = chain_flag(chain);
    let data = run_onchainos(&["gateway", "gas", "--chain", chain])?;

    // Response: [ { "normal": "11510000", "supportEip1559": true, ... } ]
    let entry = if data.is_array() {
        data.as_array()
            .and_then(|a| a.first())
            .context("onchainos gateway gas: empty response")?
            .clone()
    } else {
        data
    };

    // "normal" = baseFee + proposePriorityFee (good default for cost estimation)
    let normal_str = entry["normal"]
        .as_str()
        .or_else(|| entry["normal"].as_u64().map(|_| ""))
        .context("onchainos gateway gas: missing 'normal' field")?;

    if normal_str.is_empty() {
        // numeric value
        entry["normal"]
            .as_u64()
            .map(|n| n as u128)
            .context("onchainos gateway gas: cannot parse 'normal'")
    } else {
        normal_str
            .parse::<u128>()
            .context("onchainos gateway gas: invalid 'normal' value")
    }
}

/// Execute `onchainos wallet contract-call` and return the txHash.
///
/// * `chain`      — onchainos chain name (e.g. "base_eth", "eth")
/// * `to`         — contract address (checksummed hex)
/// * `input_data` — ABI-encoded calldata (hex with 0x prefix)
/// * `value`      — native token value to send (e.g. "0")
pub async fn contract_call(
    chain: &str,
    to: &str,
    input_data: &str,
    value: &str,
) -> Result<String> {
    let data = run_onchainos(&[
        "wallet",
        "contract-call",
        "--to",
        to,
        "--chain",
        chain,
        "--input-data",
        input_data,
        "--value",
        value,
        "--gas-limit",
        "500000",
    ])?;

    let tx_hash = data["txHash"]
        .as_str()
        .context("onchainos contract-call response missing txHash")?
        .to_string();

    Ok(tx_hash)
}

// ── Token wrappers ────────────────────────────────────────────────

/// onchainos token trending --chains <chains> --sort-by <sort_by> --time-frame <time_frame>
pub fn token_trending(chains: &str, sort_by: &str, time_frame: &str) -> Result<Value> {
    run_onchainos(&[
        "token",
        "trending",
        "--chains",
        api_chain(chains),
        "--sort-by",
        sort_by,
        "--time-frame",
        time_frame,
    ])
}

/// onchainos token advanced-info --address <addr> --chain <chain>
pub fn token_advanced_info(address: &str, chain: &str) -> Result<Value> {
    run_onchainos(&[
        "token",
        "advanced-info",
        "--address",
        address,
        "--chain",
        api_chain(chain),
    ])
}

/// onchainos token price-info --address <addr> --chain <chain>
pub fn token_price_info(address: &str, chain: &str) -> Result<Value> {
    run_onchainos(&[
        "token",
        "price-info",
        "--address",
        address,
        "--chain",
        api_chain(chain),
    ])
}

/// onchainos token holders --address <addr> --chain <chain> [--tag-filter <tag>]
pub fn token_holders(address: &str, chain: &str, tag_filter: Option<&str>) -> Result<Value> {
    let mut args = vec![
        "token",
        "holders",
        "--address",
        address,
        "--chain",
        api_chain(chain),
    ];
    if let Some(tag) = tag_filter {
        args.push("--tag-filter");
        args.push(tag);
    }
    run_onchainos(&args)
}

// ── Portfolio wrappers ────────────────────────────────────────────

/// onchainos portfolio all-balances --address <addr> --chains <chains>
pub fn portfolio_all_balances(address: &str, chains: &str) -> Result<Value> {
    run_onchainos(&[
        "portfolio",
        "all-balances",
        "--address",
        address,
        "--chains",
        chains,
    ])
}

// ── Signal wrappers ──────────────────────────────────────────────

/// onchainos signal list --chain <chain> [--wallet-type <wt>] [--min-address-count <n>]
/// [--min-market-cap-usd <n>] [--min-liquidity-usd <n>]
pub fn signal_list(
    chain: &str,
    wallet_type: Option<&str>,
    min_address_count: Option<&str>,
    min_market_cap_usd: Option<&str>,
    min_liquidity_usd: Option<&str>,
) -> Result<Value> {
    let mut args = vec!["signal", "list", "--chain", api_chain(chain)];
    if let Some(wt) = wallet_type {
        args.push("--wallet-type");
        args.push(wt);
    }
    if let Some(n) = min_address_count {
        args.push("--min-address-count");
        args.push(n);
    }
    if let Some(n) = min_market_cap_usd {
        args.push("--min-market-cap-usd");
        args.push(n);
    }
    if let Some(n) = min_liquidity_usd {
        args.push("--min-liquidity-usd");
        args.push(n);
    }
    run_onchainos(&args)
}

// ── Market wrappers ──────────────────────────────────────────────

/// onchainos market kline --address <addr> --chain <chain> --bar <bar> --limit <limit>
pub fn market_kline(address: &str, chain: &str, bar: &str, limit: &str) -> Result<Value> {
    run_onchainos(&[
        "market",
        "kline",
        "--address",
        address,
        "--chain",
        api_chain(chain),
        "--bar",
        bar,
        "--limit",
        limit,
    ])
}

// ── Memepump wrappers ───────────────────────────────────────────

/// onchainos memepump token-dev-info --address <addr> --chain <chain>
pub fn memepump_dev_info(address: &str, chain: &str) -> Result<Value> {
    run_onchainos(&[
        "memepump",
        "token-dev-info",
        "--address",
        address,
        "--chain",
        api_chain(chain),
    ])
}

/// onchainos memepump token-bundle-info --address <addr> --chain <chain>
pub fn memepump_bundle_info(address: &str, chain: &str) -> Result<Value> {
    run_onchainos(&[
        "memepump",
        "token-bundle-info",
        "--address",
        address,
        "--chain",
        api_chain(chain),
    ])
}

/// onchainos memepump tokens --chain <chain> --stage <stage> [+ dynamic filter flags]
///
/// `filters` is a list of (flag_name, value) pairs like ("--min-market-cap", "50000").
pub fn memepump_tokens(chain: &str, stage: &str, filters: &[(&str, &str)]) -> Result<Value> {
    let mut args = vec![
        "memepump",
        "tokens",
        "--chain",
        api_chain(chain),
        "--stage",
        stage,
    ];
    for (flag, value) in filters {
        args.push(flag);
        args.push(value);
    }
    run_onchainos(&args)
}

// ── Swap wrappers ───────────────────────────────────────────────

/// onchainos swap quote --from <from> --to <to> --amount <amt> --chain <chain> [--slippage <s>]
pub fn swap_quote(
    from: &str,
    to: &str,
    amount: &str,
    chain: &str,
    slippage: Option<&str>,
) -> Result<Value> {
    let mut args = vec![
        "swap",
        "quote",
        "--from",
        from,
        "--to",
        to,
        "--amount",
        amount,
        "--chain",
        api_chain(chain),
    ];
    if let Some(s) = slippage {
        args.push("--slippage");
        args.push(s);
    }
    run_onchainos(&args)
}

/// onchainos swap swap --from <from> --to <to> --amount <amt> --chain <chain> --wallet <wallet> [--slippage <s>]
pub fn swap_swap(
    from: &str,
    to: &str,
    amount: &str,
    chain: &str,
    wallet: &str,
    slippage: Option<&str>,
) -> Result<Value> {
    let mut args = vec![
        "swap",
        "swap",
        "--from",
        from,
        "--to",
        to,
        "--amount",
        amount,
        "--chain",
        api_chain(chain),
        "--wallet",
        wallet,
    ];
    if let Some(s) = slippage {
        args.push("--slippage");
        args.push(s);
    }
    run_onchainos(&args)
}

/// onchainos swap approve --token <addr> --amount <amt> --chain <chain>
pub fn swap_approve(token: &str, amount: &str, chain: &str) -> Result<Value> {
    run_onchainos(&[
        "swap",
        "approve",
        "--token",
        token,
        "--amount",
        amount,
        "--chain",
        api_chain(chain),
    ])
}

// ── Gateway wrappers ────────────────────────────────────────────

/// onchainos gateway orders --address <addr> --chain <chain> --order-id <id>
pub fn gateway_orders(address: &str, chain: &str, order_id: &str) -> Result<Value> {
    run_onchainos(&[
        "gateway",
        "orders",
        "--address",
        address,
        "--chain",
        api_chain(chain),
        "--order-id",
        order_id,
    ])
}

// ── Solana-specific helpers ─────────────────────────────────────

/// Sign and broadcast a Solana transaction via onchainos wallet.
///
/// onchainos wallet contract-call --to <program_id> --chain solana --unsigned-tx <base58_tx>
pub async fn contract_call_solana(to: &str, unsigned_tx_b58: &str) -> Result<String> {
    let data = run_onchainos(&[
        "wallet",
        "contract-call",
        "--to",
        to,
        "--chain",
        chain_flag("solana"), // wallet command uses "sol"
        "--unsigned-tx",
        unsigned_tx_b58,
    ])?;

    let tx_hash = data["txHash"]
        .as_str()
        .context("onchainos contract-call (solana) response missing txHash")?
        .to_string();

    Ok(tx_hash)
}

/// Execute a Solana swap: get unsigned tx via `onchainos swap swap`,
/// then sign and broadcast via `onchainos wallet contract-call`.
///
/// Returns `(tx_hash, swap_data)` where `swap_data` contains `routerResult` etc.
pub async fn execute_solana_swap(
    from: &str,
    to: &str,
    amount: &str,
    wallet: &str,
    slippage: &str,
) -> Result<(String, Value)> {
    // 1. Get swap tx data
    let raw = swap_swap(from, to, amount, "solana", wallet, Some(slippage))?;
    let swap_data = if raw.is_array() {
        raw.as_array()
            .and_then(|a| a.first())
            .cloned()
            .context("empty swap response")?
    } else {
        raw
    };

    let unsigned_tx_b58 = swap_data["tx"]["data"]
        .as_str()
        .context("swap response missing tx.data (unsigned transaction)")?;
    let program_id = swap_data["tx"]["to"]
        .as_str()
        .context("swap response missing tx.to (program id)")?;

    // 2. Sign and broadcast via onchainos wallet
    let tx_hash = contract_call_solana(program_id, unsigned_tx_b58).await?;
    Ok((tx_hash, swap_data))
}
