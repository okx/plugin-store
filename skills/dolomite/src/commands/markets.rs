use crate::config::get_chain_config;
use crate::rpc;

/// List Dolomite lending markets on the given chain.
/// Queries DolomiteMargin for all markets and their token addresses, TVL, and interest rates.
pub async fn run(chain_id: u64, asset_filter: Option<&str>) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let margin = cfg.dolomite_margin;

    let total = rpc::get_num_markets(margin, rpc).await?;

    let mut markets = Vec::new();
    let filter_upper = asset_filter.map(|s| s.to_uppercase());

    // Fetch up to 30 markets
    let fetch_count = total.min(30);

    for market_id in 0..fetch_count {
        let token_addr = match rpc::get_market_token_address(margin, market_id, rpc).await {
            Ok(a) => a,
            Err(_) => continue,
        };

        let symbol = rpc::erc20_symbol(&token_addr, rpc).await.unwrap_or_else(|_| "UNKNOWN".to_string());
        let decimals = rpc::erc20_decimals(&token_addr, rpc).await.unwrap_or(18);

        // Apply filter
        if let Some(ref f) = filter_upper {
            if !symbol.to_uppercase().contains(f.as_str())
                && !token_addr.to_lowercase().contains(&f.to_lowercase())
            {
                continue;
            }
        }

        // Get total par (borrow and supply)
        let (borrow_par, supply_par) = rpc::get_market_total_par(margin, market_id, rpc)
            .await
            .unwrap_or((0, 0));

        let supply_formatted = rpc::format_amount(supply_par, decimals);
        let borrow_formatted = rpc::format_amount(borrow_par, decimals);

        markets.push(serde_json::json!({
            "marketId": market_id,
            "token": token_addr,
            "symbol": symbol,
            "decimals": decimals,
            "totalSupply": supply_formatted,
            "totalBorrow": borrow_formatted,
            "depositInstruction": format!(
                "dolomite --chain {} deposit --asset {} --amount <n>",
                chain_id, symbol
            )
        }));
    }

    let output = serde_json::json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "dolomiteMargin": margin,
        "totalMarkets": total,
        "showing": markets.len(),
        "markets": markets,
        "note": "Supply tokens via 'dolomite --chain <id> deposit --asset <SYMBOL> --amount <n>'"
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
