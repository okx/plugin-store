use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// View user's Dolomite supply and borrow positions.
pub async fn run(chain_id: u64, from: Option<&str>, dry_run: bool) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let margin = cfg.dolomite_margin;

    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    // Query DolomiteMargin for account balances
    let balances = rpc::get_account_balances(margin, &wallet, 0, rpc).await?;

    let mut positions = Vec::new();

    for bal in &balances {
        // Get Wei balance (actual token amount with sign)
        let (sign, wei_value) = rpc::get_account_wei(margin, &wallet, 0, bal.market_id, rpc)
            .await
            .unwrap_or((true, 0));

        if wei_value == 0 {
            continue;
        }

        let symbol = rpc::erc20_symbol(&bal.token_address, rpc)
            .await
            .unwrap_or_else(|_| "UNKNOWN".to_string());
        let decimals = rpc::erc20_decimals(&bal.token_address, rpc)
            .await
            .unwrap_or(18);

        let amount_formatted = rpc::format_amount(wei_value, decimals);
        let position_type = if sign { "supply" } else { "borrow" };

        positions.push(serde_json::json!({
            "marketId": bal.market_id,
            "token": bal.token_address,
            "symbol": symbol,
            "type": position_type,
            "amount": amount_formatted,
            "rawAmount": wei_value.to_string(),
            "positive": sign,
        }));
    }

    let output = serde_json::json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "wallet": wallet,
        "dryRun": dry_run,
        "positionCount": positions.len(),
        "positions": positions,
        "note": if positions.is_empty() {
            "No active Dolomite positions found. Deposit via 'dolomite --chain <id> deposit --asset USDC --amount <n>'"
        } else {
            "Positive = supply (lending), negative = borrow (debt)"
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
