use clap::Args;
use serde_json::json;

use crate::config::{chain_id, delegation_manager, lst_strategies, rpc_url, strategy_manager};
use crate::onchainos::resolve_wallet;
use crate::rpc::{format_eth, get_delegated_operator, get_deposits, shares_to_underlying};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (defaults to connected onchainos wallet)
    #[arg(long)]
    pub owner: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let sm = strategy_manager();
    let dm = delegation_manager();

    let owner = match args.owner {
        Some(addr) => addr,
        None => resolve_wallet(chain_id())?,
    };

    // Fetch all deposits in one call
    let deposits = get_deposits(sm, &owner, rpc).await?;

    // Build a lookup map from strategy address (lowercase) to symbol
    let strategy_map: std::collections::HashMap<String, (&str, &str, u8)> = lst_strategies()
        .iter()
        .map(|s| (s.strategy.to_lowercase(), (s.symbol, s.token, s.decimals)))
        .collect();

    let mut positions = vec![];
    let mut total_eth_equivalent = 0u128;

    for (strategy_addr, shares) in &deposits {
        if *shares == 0 {
            continue;
        }

        let strategy_lower = strategy_addr.to_lowercase();
        let (symbol, token_addr) = if let Some((sym, tok, _)) = strategy_map.get(&strategy_lower) {
            (*sym, *tok)
        } else {
            ("UNKNOWN", "0x0000000000000000000000000000000000000000")
        };

        // Convert shares to underlying token amount
        let underlying = shares_to_underlying(strategy_addr, *shares, rpc)
            .await
            .unwrap_or(*shares); // fallback: 1:1 if call fails
        total_eth_equivalent += underlying;

        positions.push(json!({
            "token": symbol,
            "tokenAddress": token_addr,
            "strategyAddress": strategy_addr,
            "sharesWei": shares.to_string(),
            "underlyingWei": underlying.to_string(),
            "underlyingFormatted": format_eth(underlying)
        }));
    }

    // Check delegation status
    let operator = get_delegated_operator(dm, &owner, rpc).await?;
    let is_delegated = operator != "0x0000000000000000000000000000000000000000";

    let output = json!({
        "ok": true,
        "owner": owner,
        "chain": "ethereum",
        "chainId": 1,
        "totalPositions": positions.len(),
        "totalEthEquivalentWei": total_eth_equivalent.to_string(),
        "totalEthEquivalentFormatted": format_eth(total_eth_equivalent),
        "isDelegated": is_delegated,
        "operator": if is_delegated { operator.as_str() } else { "none" },
        "positions": positions
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
