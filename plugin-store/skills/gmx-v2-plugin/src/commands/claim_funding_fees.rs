use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct ClaimFundingFeesArgs {
    /// Comma-separated market token addresses to claim from
    #[arg(long)]
    pub markets: String,

    /// Comma-separated token addresses corresponding to each market
    #[arg(long)]
    pub tokens: String,

    /// Receiver address (defaults to logged-in wallet)
    #[arg(long)]
    pub receiver: Option<String>,

    /// Wallet address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(chain: &str, dry_run: bool, confirm: bool, args: ClaimFundingFeesArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.from.clone().unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let receiver = args.receiver.as_deref().unwrap_or(&wallet).to_string();

    // Parse comma-separated addresses
    let market_addrs: Vec<&str> = args.markets.split(',').map(|s| s.trim()).collect();
    let token_addrs: Vec<&str> = args.tokens.split(',').map(|s| s.trim()).collect();

    if market_addrs.len() != token_addrs.len() {
        anyhow::bail!(
            "markets and tokens arrays must have the same length ({} vs {})",
            market_addrs.len(),
            token_addrs.len()
        );
    }
    if market_addrs.is_empty() {
        anyhow::bail!("Must provide at least one market address.");
    }

    let calldata_hex = crate::abi::encode_claim_funding_fees(&market_addrs, &token_addrs, &receiver);
    let calldata = format!("0x{}", calldata_hex);

    eprintln!("=== Claim Funding Fees Preview ===");
    eprintln!("Markets: {:?}", market_addrs);
    eprintln!("Tokens: {:?}", token_addrs);
    eprintln!("Receiver: {}", receiver);
    eprintln!("Note: No execution fee needed for claims.");
    if !confirm { eprintln!("Add --confirm to broadcast."); }

    if !confirm && !dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "status": "preview",
                "message": "Add --confirm to broadcast this transaction",
                "chain": chain,
                "markets": market_addrs,
                "tokens": token_addrs,
                "receiver": receiver,
                "calldata": calldata
            }))?
        );
        return Ok(());
    }

    // G20: snapshot balances before tx so we can report claimed amounts
    let pre_balances: Vec<u128> = if confirm && !dry_run {
        let mut bals = Vec::new();
        for token in &token_addrs {
            let bal = crate::rpc::check_erc20_balance(cfg.rpc_url, token, &receiver).await.unwrap_or(0);
            bals.push(bal);
        }
        bals
    } else {
        vec![0u128; token_addrs.len()]
    };

    let result = crate::onchainos::wallet_contract_call_with_gas(
        cfg.chain_id,
        cfg.exchange_router,
        &calldata,
        Some(&wallet),
        None, // no ETH value needed for claim
        dry_run,
        confirm,
        Some(300_000),
    ).await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&result);

    // G20: post-tx balance delta = claimed amounts
    let claimed: Vec<serde_json::Value> = if confirm && !dry_run {
        crate::onchainos::wait_for_tx(cfg.chain_id, &tx_hash, &wallet, 60)?;
        let mut out = Vec::new();
        for (i, token) in token_addrs.iter().enumerate() {
            let post = crate::rpc::check_erc20_balance(cfg.rpc_url, token, &receiver).await.unwrap_or(0);
            let delta = post.saturating_sub(pre_balances[i]);
            out.push(json!({ "token": token, "claimedRaw": delta.to_string() }));
        }
        out
    } else {
        vec![]
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "dry_run": dry_run,
            "chain": chain,
            "txHash": tx_hash,
            "markets": market_addrs,
            "tokens": token_addrs,
            "receiver": receiver,
            "claimed": claimed,
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
