use crate::config::{get_chain_config, get_known_token};
use crate::onchainos;
use crate::rpc;

/// Borrow tokens from Dolomite (DRY-RUN ONLY — liquidation risk).
///
/// Borrowing creates a negative balance for the given market.
/// Requires sufficient collateral in other markets to avoid liquidation.
///
/// This command is always dry-run only. Use --dry-run flag explicitly.
pub async fn run(
    asset_input: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !dry_run {
        anyhow::bail!(
            "Borrow is dry-run only due to liquidation risk. \
             Add --dry-run to simulate. Ensure sufficient collateral before borrowing on-chain."
        );
    }

    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let margin = cfg.dolomite_margin;

    let (token_addr, decimals) = resolve_token(asset_input, chain_id, rpc).await?;
    let raw_amount = rpc::parse_amount(amount, decimals)?;

    let wallet = onchainos::resolve_wallet(chain_id, dry_run)?;
    let symbol = rpc::erc20_symbol(&token_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());
    let market_id = rpc::find_market_id(margin, &token_addr, rpc).await?;

    eprintln!(
        "[dolomite] [dry-run] Would borrow {} {} (marketId={}) from DolomiteMargin on {}",
        amount, symbol, market_id, cfg.name
    );

    // Borrow = Withdraw with negative balance intent
    // ActionType=1 (Withdraw), sign=false, denomination=Wei, ref=Delta, value=rawAmount
    // otherAddress = wallet (tokens sent to wallet)
    let operate_calldata = rpc::encode_operate(
        &wallet,
        1,           // Withdraw (creates borrow position if no prior supply)
        false,       // sign = negative
        raw_amount,
        market_id,
        &wallet,     // otherAddress = recipient
        false,
    );

    // Always dry-run for borrow
    let _result = onchainos::wallet_contract_call(
        chain_id, margin, &operate_calldata, from, None, true
    ).await?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "borrow",
        "token": token_addr,
        "symbol": symbol,
        "marketId": market_id,
        "amount": amount,
        "rawAmount": raw_amount.to_string(),
        "wallet": wallet,
        "dolomiteMargin": margin,
        "chain": cfg.name,
        "chainId": chain_id,
        "dryRun": true,
        "warning": "Borrow is always dry-run only. Liquidation risk: ensure sufficient collateral before borrowing on-chain.",
        "simulatedCalldata": operate_calldata,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn resolve_token(input: &str, chain_id: u64, rpc: &str) -> anyhow::Result<(String, u8)> {
    if input.starts_with("0x") && input.len() == 42 {
        let addr = input.to_lowercase();
        let decimals = rpc::erc20_decimals(&addr, rpc).await.unwrap_or(18);
        Ok((addr, decimals))
    } else if let Some((addr, decimals)) = get_known_token(input, chain_id) {
        Ok((addr.to_string(), decimals))
    } else {
        anyhow::bail!(
            "Unknown asset '{}'. Use a token address (0x...) or symbol (USDC, WETH, USDT). \
             Run 'dolomite --chain {} markets' to list available assets.",
            input, chain_id
        )
    }
}
