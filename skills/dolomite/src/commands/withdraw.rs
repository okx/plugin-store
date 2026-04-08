use crate::config::{get_chain_config, get_known_token};
use crate::onchainos;
use crate::rpc;

/// Withdraw supplied tokens from Dolomite DolomiteMargin.
///
/// Uses DolomiteMargin.operate() with Withdraw action (ActionType=1).
///
/// CONFIRM: This is an on-chain write operation. Review asset, amount, and chain before confirming.
pub async fn run(
    asset_input: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !all && amount.is_none() {
        anyhow::bail!("Specify --amount <n> or --all to withdraw entire balance.");
    }

    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let margin = cfg.dolomite_margin;

    let (token_addr, decimals) = resolve_token(asset_input, chain_id, rpc).await?;

    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let symbol = rpc::erc20_symbol(&token_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());
    let market_id = rpc::find_market_id(margin, &token_addr, rpc).await?;

    let (raw_amount, max_withdraw) = if all {
        (0u128, true)
    } else {
        (rpc::parse_amount(amount.unwrap(), decimals)?, false)
    };

    let amount_display = if all {
        "ALL".to_string()
    } else {
        amount.unwrap_or("0").to_string()
    };

    eprintln!(
        "[dolomite] Withdrawing {} {} (marketId={}) from DolomiteMargin on {}",
        amount_display, symbol, market_id, cfg.name
    );

    // operate() with Withdraw action (ActionType=1)
    // For regular withdraw: sign=false, denomination=Wei, ref=Delta, value=rawAmount
    // For max withdraw: sign=false, denomination=Wei, ref=Target(1), value=0
    let operate_calldata = rpc::encode_operate(
        &wallet,
        1,           // Withdraw
        false,       // sign = negative (reduce supply)
        raw_amount,
        market_id,
        &wallet,     // otherAddress = to
        max_withdraw,
    );

    let operate_result = onchainos::wallet_contract_call(
        chain_id, margin, &operate_calldata, from, None, dry_run
    ).await?;
    let tx_hash = onchainos::extract_tx_hash(&operate_result).to_string();

    let output = serde_json::json!({
        "ok": true,
        "operation": "withdraw",
        "token": token_addr,
        "symbol": symbol,
        "marketId": market_id,
        "amount": amount_display,
        "rawAmount": raw_amount.to_string(),
        "maxWithdraw": max_withdraw,
        "wallet": wallet,
        "dolomiteMargin": margin,
        "chain": cfg.name,
        "chainId": chain_id,
        "dryRun": dry_run,
        "txHash": tx_hash,
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
