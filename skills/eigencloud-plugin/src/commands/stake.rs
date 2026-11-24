use clap::Args;
use crate::abi::{selector, calldata, encode_address, encode_uint256};
use crate::chain::CHAIN_ID;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::strategies::by_symbol;

const STRATEGY_MANAGER: &str = "0x858646372CC42E1A627fcE94aa7A7033e7CF075A";

#[derive(Args)]
pub struct StakeArgs {
    /// LST symbol to restake (e.g. stETH, rETH, cbETH)
    #[arg(long)]
    pub token: String,
    /// Amount to restake in human-readable form (e.g. 0.01)
    #[arg(long)]
    pub amount: String,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Dry-run: build calldata without calling onchainos
    #[arg(long, conflicts_with = "confirm")]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    let strategy = by_symbol(&args.token)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown token '{}'. Run `eigencloud strategies` for the supported list.", args.token
        ))?;

    let amount_raw = parse_amount(&args.amount, strategy.decimals)?;
    if amount_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let wallet = if args.dry_run {
        resolve_wallet(CHAIN_ID).unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    } else if args.confirm {
        resolve_wallet(CHAIN_ID)?
    } else {
        resolve_wallet(CHAIN_ID).unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    };

    // Build approve calldata: approve(spender, amount)
    let approve_sel = selector("approve(address,uint256)");
    let approve_data = calldata(approve_sel, &[
        encode_address(STRATEGY_MANAGER),
        encode_uint256(amount_raw),
    ]);

    // Build depositIntoStrategy calldata
    let deposit_sel = selector("depositIntoStrategy(address,address,uint256)");
    let deposit_data = calldata(deposit_sel, &[
        encode_address(strategy.strategy),
        encode_address(strategy.token),
        encode_uint256(amount_raw),
    ]);

    let preview = serde_json::json!({
        "preview":         !args.confirm && !args.dry_run,
        "action":          "stake",
        "token":           strategy.symbol,
        "amount":          args.amount,
        "token_contract":  strategy.token,
        "strategy":        strategy.strategy,
        "strategy_manager": STRATEGY_MANAGER,
        "wallet":          wallet,
        "steps":           ["approve", "depositIntoStrategy"],
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast this stake transaction.");
        return Ok(());
    }

    // Step 1: Approve
    eprintln!("[eigencloud] Approving {} {} for StrategyManager...", args.amount, strategy.symbol);
    let approve_result = wallet_contract_call(CHAIN_ID, strategy.token, &approve_data, "0", args.dry_run, Some(&wallet))?;
    let approve_tx = extract_tx_hash(&approve_result);
    eprintln!("[eigencloud] approve tx: {}", approve_tx);

    if !args.dry_run {
        eprintln!("[eigencloud] Waiting for approval to confirm...");
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    }

    // Step 2: depositIntoStrategy
    eprintln!("[eigencloud] Depositing {} {} into EigenLayer...", args.amount, strategy.symbol);
    let deposit_result = wallet_contract_call(CHAIN_ID, STRATEGY_MANAGER, &deposit_data, "0", args.dry_run, Some(&wallet))?;
    let deposit_tx = extract_tx_hash(&deposit_result);
    eprintln!("[eigencloud] deposit tx: {}", deposit_tx);

    let mut out = serde_json::json!({
        "ok":       true,
        "action":   "stake",
        "token":    strategy.symbol,
        "amount":   args.amount,
        "wallet":   wallet,
        "strategy": strategy.strategy,
        "txs": [
            {"step": "approve",              "tx_hash": approve_tx},
            {"step": "depositIntoStrategy",  "tx_hash": deposit_tx},
        ],
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

pub fn parse_amount(s: &str, decimals: u8) -> anyhow::Result<u128> {
    if s == "0" || s.is_empty() {
        anyhow::bail!("Amount must be greater than 0");
    }
    let (whole, frac) = if let Some(dot) = s.find('.') {
        let w: u128 = s[..dot].parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        let frac_str = &s[dot + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!("Amount '{}' has {} decimal places but {} supports only {}", s, frac_str.len(), "token", decimals);
        }
        let padded = format!("{:0<width$}", frac_str, width = decimals as usize);
        let f: u128 = padded.parse().map_err(|_| anyhow::anyhow!("Invalid fractional amount: '{}'", s))?;
        (w, f)
    } else {
        let w: u128 = s.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        (w, 0u128)
    };
    let scale = 10u128.pow(decimals as u32);
    Ok(whole * scale + frac)
}
