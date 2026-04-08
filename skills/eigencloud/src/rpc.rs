use anyhow::Context;
use serde_json::{json, Value};

/// Perform an eth_call via JSON-RPC (Ethereum mainnet).
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            {"to": to, "data": data},
            "latest"
        ],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call JSON parse failed")?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Get ERC-20 balance of an address.
/// balanceOf(address) -> uint256 — selector 0x70a08231
pub async fn get_erc20_balance(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    use crate::config::pad_address;
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(token, &data, rpc_url).await?;
    Ok(parse_u128_from_hex(&hex))
}

/// Get ERC-20 allowance.
/// allowance(address owner, address spender) -> uint256 — selector 0xdd62ed3e
pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    use crate::config::pad_address;
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let hex = eth_call(token, &data, rpc_url).await?;
    Ok(parse_u128_from_hex(&hex))
}

/// StrategyManager.stakerDepositShares(address user, address strategy) -> uint256
/// Returns deposit shares held by staker in the given strategy.
/// Selector: 0xfe243a17
pub async fn get_staker_strategy_shares(
    strategy_manager: &str,
    staker: &str,
    strategy: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    use crate::config::pad_address;
    let data = format!("0xfe243a17{}{}", pad_address(staker), pad_address(strategy));
    let hex = eth_call(strategy_manager, &data, rpc_url).await?;
    Ok(parse_u128_from_hex(&hex))
}

/// StrategyManager.getDeposits(address staker) -> (address[] strategies, uint256[] shares)
/// Returns all strategies and corresponding deposit shares for a staker.
/// Selector: 0x94f649dd
/// ABI returns: [offset_strategies(32), offset_shares(32), strategies_len(32), ...strategies, shares_len(32), ...shares]
pub async fn get_deposits(
    strategy_manager: &str,
    staker: &str,
    rpc_url: &str,
) -> anyhow::Result<Vec<(String, u128)>> {
    use crate::config::pad_address;
    let data = format!("0x94f649dd{}", pad_address(staker));
    let hex = eth_call(strategy_manager, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok(vec![]);
    }

    // Parse offsets to arrays (in 32-byte words, each word = 64 hex chars)
    let strategies_offset = usize::from_str_radix(&clean[0..64], 16).unwrap_or(0) * 2; // byte offset -> hex offset
    let _shares_offset = usize::from_str_radix(&clean[64..128], 16).unwrap_or(0) * 2;

    if clean.len() < strategies_offset + 64 {
        return Ok(vec![]);
    }

    let strategies_len = usize::from_str_radix(&clean[strategies_offset..strategies_offset + 64], 16).unwrap_or(0);
    let shares_offset = usize::from_str_radix(&clean[64..128], 16).unwrap_or(0) * 2;
    if clean.len() < shares_offset + 64 {
        return Ok(vec![]);
    }
    let shares_len = usize::from_str_radix(&clean[shares_offset..shares_offset + 64], 16).unwrap_or(0);

    let count = strategies_len.min(shares_len);
    let mut result = Vec::with_capacity(count);

    for i in 0..count {
        let strat_start = strategies_offset + 64 + i * 64;
        let share_start = shares_offset + 64 + i * 64;

        if strat_start + 64 > clean.len() || share_start + 64 > clean.len() {
            break;
        }

        let strat_word = &clean[strat_start..strat_start + 64];
        let strat_addr = format!("0x{}", &strat_word[strat_word.len().saturating_sub(40)..]);

        let share_word = &clean[share_start..share_start + 64];
        let share_val = u128::from_str_radix(
            if share_word.len() > 32 { &share_word[share_word.len() - 32..] } else { share_word },
            16,
        ).unwrap_or(0);

        result.push((strat_addr, share_val));
    }

    Ok(result)
}

/// DelegationManager.delegatedTo(address staker) -> address operator
/// Returns the operator the staker is delegated to (zero if undelegated).
/// Selector: 0x65da1264
pub async fn get_delegated_operator(
    delegation_manager: &str,
    staker: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    use crate::config::pad_address;
    let data = format!("0x65da1264{}", pad_address(staker));
    let hex = eth_call(delegation_manager, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() >= 40 {
        Ok(format!("0x{}", &clean[clean.len() - 40..]))
    } else {
        Ok("0x0000000000000000000000000000000000000000".to_string())
    }
}

/// DelegationManager.isOperator(address operator) -> bool
/// Selector: 0x6d70f7ae
pub async fn is_operator(
    delegation_manager: &str,
    addr: &str,
    rpc_url: &str,
) -> anyhow::Result<bool> {
    use crate::config::pad_address;
    let data = format!("0x6d70f7ae{}", pad_address(addr));
    let hex = eth_call(delegation_manager, &data, rpc_url).await?;
    Ok(parse_u128_from_hex(&hex) != 0)
}

/// Strategy.sharesToUnderlying(uint256 shares) -> uint256
/// Converts strategy shares to underlying token amount.
/// Selector: 0xf3e73875
pub async fn shares_to_underlying(
    strategy: &str,
    shares: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    use crate::config::pad_u256;
    let data = format!("0xf3e73875{}", pad_u256(shares));
    let hex = eth_call(strategy, &data, rpc_url).await?;
    Ok(parse_u128_from_hex(&hex))
}

/// Strategy.totalShares() -> uint256
/// Total shares deposited in this strategy (proxy for TVL).
/// Selector: 0x3a98ef39
pub async fn get_total_shares(strategy: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(strategy, "0x3a98ef39", rpc_url).await?;
    Ok(parse_u128_from_hex(&hex))
}

/// Strategy.underlyingToken() -> address
/// Selector: 0x2495a599
#[allow(dead_code)]
pub async fn get_underlying_token(strategy: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(strategy, "0x2495a599", rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() >= 40 {
        Ok(format!("0x{}", &clean[clean.len() - 40..]))
    } else {
        Ok("0x0000000000000000000000000000000000000000".to_string())
    }
}

/// Parse the last 32 hex bytes of an ABI-encoded uint256 hex string to u128.
/// Ignores values exceeding u128::MAX by clamping to 0.
pub fn parse_u128_from_hex(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    u128::from_str_radix(trimmed, 16).unwrap_or(0)
}

/// Format a u128 wei value as a decimal ETH string (18 decimals).
pub fn format_eth(wei: u128) -> String {
    let whole = wei / 1_000_000_000_000_000_000u128;
    let frac = (wei % 1_000_000_000_000_000_000u128) / 1_000_000_000_000u128; // 6 decimal places
    format!("{}.{:06}", whole, frac)
}
