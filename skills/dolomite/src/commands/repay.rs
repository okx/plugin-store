use crate::config::{get_chain_config, get_known_token};
use crate::onchainos;
use crate::rpc;

/// Repay borrowed tokens to Dolomite (DRY-RUN ONLY).
///
/// Repaying reduces the negative balance for the given market.
///
/// This command is always dry-run only.
pub async fn run(
    asset_input: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !dry_run {
        anyhow::bail!(
            "Repay is dry-run only. Add --dry-run to simulate. \
             For on-chain repay, deposit the borrowed asset to reduce debt position."
        );
    }
    if !all && amount.is_none() {
        anyhow::bail!("Specify --amount <n> or --all to repay entire debt.");
    }

    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let margin = cfg.dolomite_margin;

    let (token_addr, decimals) = resolve_token(asset_input, chain_id, rpc).await?;

    let wallet = onchainos::resolve_wallet(chain_id, dry_run)?;
    let symbol = rpc::erc20_symbol(&token_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());
    let market_id = rpc::find_market_id(margin, &token_addr, rpc).await?;

    let (raw_amount, max_repay) = if all {
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
        "[dolomite] [dry-run] Would repay {} {} (marketId={}) to DolomiteMargin on {}",
        amount_display, symbol, market_id, cfg.name
    );

    // Repay = Deposit action (ActionType=0) to reduce negative borrow balance
    // sign=true, denomination=Wei, ref=Delta (or Target for all)
    let operate_calldata = rpc::encode_operate(
        &wallet,
        0,          // Deposit (repay debt)
        true,       // sign = positive
        raw_amount,
        market_id,
        &wallet,    // otherAddress = from (tokens pulled from wallet)
        max_repay,
    );

    // Always dry-run for repay
    let _result = onchainos::wallet_contract_call(
        chain_id, margin, &operate_calldata, from, None, true
    ).await?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "repay",
        "token": token_addr,
        "symbol": symbol,
        "marketId": market_id,
        "amount": amount_display,
        "rawAmount": raw_amount.to_string(),
        "maxRepay": max_repay,
        "wallet": wallet,
        "dolomiteMargin": margin,
        "chain": cfg.name,
        "chainId": chain_id,
        "dryRun": true,
        "warning": "Repay is always dry-run only. To repay on-chain, use 'deposit' with the borrowed asset.",
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
