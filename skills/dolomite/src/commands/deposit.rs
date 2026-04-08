use crate::config::{get_chain_config, get_known_token};
use crate::onchainos;
use crate::rpc;

/// Supply (deposit) tokens to Dolomite DolomiteMargin to earn lending yield.
///
/// Steps:
///   1. ERC-20 approve(DolomiteMargin, amount)
///   2. DolomiteMargin.operate([account], [DepositAction])
///
/// CONFIRM: This is an on-chain write operation. Review asset, amount, and chain before confirming.
pub async fn run(
    asset_input: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let margin = cfg.dolomite_margin;

    let (token_addr, decimals) = resolve_token(asset_input, chain_id, rpc).await?;
    let raw_amount = rpc::parse_amount(amount, decimals)?;

    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    let symbol = rpc::erc20_symbol(&token_addr, rpc).await.unwrap_or_else(|_| "TOKEN".to_string());

    // Check user balance
    let user_balance = rpc::erc20_balance_of(&token_addr, &wallet, rpc).await.unwrap_or(0);
    if !dry_run && user_balance < raw_amount {
        anyhow::bail!(
            "Insufficient {} balance. Have: {}, Need: {}",
            symbol,
            rpc::format_amount(user_balance, decimals),
            amount
        );
    }

    // Find market ID
    let market_id = find_market_id_for_token(margin, &token_addr, chain_id, rpc).await?;

    eprintln!(
        "[dolomite] Depositing {} {} (marketId={}) to DolomiteMargin on {}",
        amount, symbol, market_id, cfg.name
    );

    // Step 1: approve DolomiteMargin to spend tokens
    eprintln!("[dolomite] Step 1/2: Approving DolomiteMargin to spend {} {}...", amount, symbol);
    let approve_calldata = format!(
        "0x095ea7b3{:0>64}{:064x}",
        margin.trim_start_matches("0x").to_lowercase(),
        raw_amount
    );
    let approve_result = onchainos::wallet_contract_call(
        chain_id, &token_addr, &approve_calldata, from, None, dry_run
    ).await?;
    let approve_tx = onchainos::extract_tx_hash(&approve_result).to_string();

    // Wait for approve to confirm
    if !dry_run {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Step 2: operate() with Deposit action (ActionType=0)
    // sign=true (positive), denomination=Wei(0), ref=Delta(0), value=rawAmount
    // otherAddress = wallet (from address)
    eprintln!("[dolomite] Step 2/2: Depositing {} {} via DolomiteMargin.operate()...", amount, symbol);
    let operate_calldata = rpc::encode_operate(
        &wallet,    // owner
        0,          // Deposit
        true,       // sign = positive
        raw_amount,
        market_id,
        &wallet,    // otherAddress = from
        false,      // not max
    );
    let operate_result = onchainos::wallet_contract_call(
        chain_id, margin, &operate_calldata, from, None, dry_run
    ).await?;
    let operate_tx = onchainos::extract_tx_hash(&operate_result).to_string();

    let output = serde_json::json!({
        "ok": true,
        "operation": "deposit",
        "token": token_addr,
        "symbol": symbol,
        "marketId": market_id,
        "amount": amount,
        "rawAmount": raw_amount.to_string(),
        "wallet": wallet,
        "dolomiteMargin": margin,
        "chain": cfg.name,
        "chainId": chain_id,
        "dryRun": dry_run,
        "approveTxHash": approve_tx,
        "depositTxHash": operate_tx,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Resolve token address and decimals from symbol or address.
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

/// Find market ID, trying known fast-path first then scanning.
async fn find_market_id_for_token(
    margin: &str,
    token_addr: &str,
    _chain_id: u64,
    rpc: &str,
) -> anyhow::Result<u64> {
    rpc::find_market_id(margin, token_addr, rpc).await
}
