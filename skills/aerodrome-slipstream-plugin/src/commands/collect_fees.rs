use clap::Args;
use crate::config::{nfpm, rpc_url, token_symbol, CHAIN_ID, pad_address};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_amount, get_decimals, nfpm_positions};

#[derive(Args)]
pub struct CollectFeesArgs {
    /// NFT token ID of the position to collect fees from
    #[arg(long)]
    pub token_id: u128,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos
    #[arg(long)]
    pub dry_run: bool,
}

/// NFPM.collect(CollectParams) — selector 0xfc6f7865
fn build_collect(token_id: u128, recipient: &str) -> String {
    format!(
        "0xfc6f7865{}{}{}{}",
        format!("{:0>64x}", token_id),
        pad_address(recipient),
        format!("{:0>64x}", u128::MAX), // amount0Max = type(uint128).max
        format!("{:0>64x}", u128::MAX), // amount1Max = type(uint128).max
    )
}

pub async fn run(args: CollectFeesArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let nfpm_addr = nfpm();

    let pos = nfpm_positions(nfpm_addr, args.token_id, rpc).await?;

    let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
    let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);
    let sym0 = token_symbol(&pos.token0).to_string();
    let sym1 = token_symbol(&pos.token1).to_string();

    if pos.tokens_owed0 == 0 && pos.tokens_owed1 == 0 {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "token_id": args.token_id,
            "message": "No uncollected fees for this position.",
            "uncollected_fees_token0": "0",
            "uncollected_fees_token1": "0"
        }))?);
        return Ok(());
    }

    let recipient = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let calldata = build_collect(args.token_id, &recipient);

    let preview = serde_json::json!({
        "preview": true,
        "action": "collect-fees",
        "token_id": args.token_id,
        "uncollected_fees_token0": format!("{} {}", format_amount(pos.tokens_owed0, dec0), sym0),
        "uncollected_fees_token1": format!("{} {}", format_amount(pos.tokens_owed1, dec1), sym1),
        "recipient": recipient,
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to collect fees from position {}.", args.token_id);
        return Ok(());
    }

    let result = wallet_contract_call(CHAIN_ID, nfpm_addr, &calldata, true, args.dry_run, Some(&recipient)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "collect-fees",
        "token_id": args.token_id,
        "collected_token0": format_amount(pos.tokens_owed0, dec0),
        "collected_token1": format_amount(pos.tokens_owed1, dec1),
        "token0_symbol": sym0,
        "token1_symbol": sym1,
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
