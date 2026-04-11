use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Build a base Command for onchainos, explicitly adding ~/.local/bin to PATH.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command and return its stdout as a parsed JSON Value.
fn run_cmd(mut cmd: Command) -> anyhow::Result<Value> {
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos exited with status {}: stderr={} stdout={}",
            output.status.code().unwrap_or(-1),
            stderr.trim(),
            stdout.trim()
        );
    }
    serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse onchainos JSON output: {}", stdout.trim()))
}

#[allow(dead_code)]
/// Search for Aave investment products on a given chain.
/// Returns the parsed JSON value from onchainos; the product list is at data["data"]["list"].
/// Product fields: investmentId (u64), name (string), rate (string), tvl (string).
pub fn defi_search(platform: &str, chain_id: u64) -> anyhow::Result<Value> {
    let mut cmd = base_cmd();
    cmd.args([
        "defi",
        "search",
        "--platform",
        platform,
        "--chain",
        &chain_id.to_string(),
    ]);
    run_cmd(cmd)
}

#[allow(dead_code)]
/// Extract the product list array from defi_search output.
/// onchainos returns {"ok": true, "data": {"list": [...], "total": N}}.
pub fn defi_search_list(result: &Value) -> &[Value] {
    result
        .get("data")
        .and_then(|d| d.get("list"))
        .and_then(|l| l.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// Invest in a DeFi product (supply / deposit).
/// investment_id: string representation of the numeric investmentId.
/// token: token symbol or address.
/// amount_minimal: amount in minimal units (e.g. "10000" for 0.01 USDC with 6 decimals).
/// wallet_addr: the wallet address performing the investment.
/// Collect / claim rewards for a DeFi platform via platform-id.
/// platform_id: analysisPlatformId from defi positions (e.g. 10 for Aave V3).
/// reward_type: e.g. "REWARD_PLATFORM", "REWARD_INVESTMENT".
pub fn defi_collect(
    platform_id: u64,
    chain_id: u64,
    wallet_addr: &str,
    reward_type: &str,
) -> anyhow::Result<Value> {
    let chain_name = chain_id_to_name(chain_id);
    let mut cmd = base_cmd();
    cmd.args([
        "defi",
        "collect",
        "--platform-id",
        &platform_id.to_string(),
        "--address",
        wallet_addr,
        "--chain",
        chain_name,
        "--reward-type",
        reward_type,
    ]);
    run_cmd(cmd)
}

/// Get DeFi positions for a wallet address on a given chain.
/// Requires --address and --chains (comma-separated chain names).
pub fn defi_positions(chain_id: u64, wallet_addr: &str) -> anyhow::Result<Value> {
    // Map chain ID to onchainos chain name
    let chain_name = chain_id_to_name(chain_id);
    let mut cmd = base_cmd();
    cmd.args([
        "defi",
        "positions",
        "--address",
        wallet_addr,
        "--chains",
        chain_name,
    ]);
    run_cmd(cmd)
}

/// Resolve a token symbol or address to (contract_address, decimals).
/// For both symbol and address inputs, queries onchainos token search to get actual decimals.
/// Falls back to decimals=18 only if the token is not found in onchainos.
pub fn resolve_token(asset: &str, chain_id: u64) -> anyhow::Result<(String, u8)> {
    let is_address = asset.starts_with("0x") && asset.len() == 42;
    let chain_name = chain_id_to_name(chain_id);
    let mut cmd = base_cmd();
    cmd.args(["token", "search", "--query", asset, "--chain", chain_name]);
    let result = run_cmd(cmd)?;

    let tokens = result
        .as_array()
        .or_else(|| result.get("data").and_then(|d| d.as_array()))
        .ok_or_else(|| anyhow::anyhow!("No tokens found for '{}' on chain {}", asset, chain_id))?;

    let first = tokens.first().ok_or_else(|| {
        anyhow::anyhow!("No token match for '{}' on chain {}", asset, chain_id)
    })?;

    // For address input: use the original address directly (token search confirms it exists);
    // for symbol input: extract the contract address from search results.
    let addr = if is_address {
        asset.to_lowercase()
    } else {
        first["tokenContractAddress"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tokenContractAddress in token search result"))?
            .to_lowercase()
    };

    let decimals = first["decimal"]
        .as_str()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(18);

    Ok((addr, decimals))
}

/// Public alias for use in dry-run command string formatting.
pub fn chain_id_to_name_pub(chain_id: u64) -> &'static str {
    chain_id_to_name(chain_id)
}

/// Map numeric chain ID to onchainos chain name string.
fn chain_id_to_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "ethereum",
        137 => "polygon",
        42161 => "arbitrum",
        8453 => "base",
        56 => "bsc",
        _ => "ethereum",
    }
}

/// Submit a contract call via onchainos wallet contract-call.
///
/// If dry_run is true, prints the command that would be run and returns a mock
/// success JSON without actually executing it.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let mut args: Vec<String> = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
    ];
    if let Some(addr) = from {
        args.push("--from".to_string());
        args.push(addr.to_string());
    }
    if dry_run {
        args.push("--dry-run".to_string());
        let cmd_str = format!("onchainos {}", args.join(" "));
        eprintln!("[dry-run] would execute: {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "simulatedCommand": cmd_str
        }));
    }
    args.push("--force".to_string());
    let mut cmd = base_cmd();
    cmd.args(&args);
    run_cmd(cmd)
}

/// Approve an ERC-20 token spend via wallet contract-call (approve(spender, uint256.max)).
/// Uses unlimited approval (type(uint256).max) for simplicity.
pub fn dex_approve(
    chain_id: u64,
    token: &str,
    spender: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Encode approve(spender, uint256.max) calldata
    let calldata = crate::calldata::encode_erc20_approve(spender, u128::MAX)
        .map_err(|e| anyhow::anyhow!("Failed to encode approve calldata: {}", e))?;
    wallet_contract_call(chain_id, token, &calldata, None, dry_run)
}

/// Get wallet balance for the active wallet.
#[allow(dead_code)]
pub fn wallet_balance(chain_id: u64) -> anyhow::Result<Value> {
    let mut cmd = base_cmd();
    cmd.args([
        "wallet",
        "balance",
        "--chain",
        &chain_id.to_string(),
    ]);
    run_cmd(cmd)
}

/// Get the currently active EVM wallet address for the given chain.
pub fn wallet_address(chain_id: u64) -> anyhow::Result<String> {
    let mut cmd = base_cmd();
    cmd.args(["wallet", "addresses", "--chain", &chain_id.to_string()]);
    let result = run_cmd(cmd)?;
    result["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not resolve wallet address from onchainos wallet addresses"))
}
