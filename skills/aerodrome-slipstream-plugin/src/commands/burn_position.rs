use clap::Args;
use crate::config::{nfpm, rpc_url, token_symbol, CHAIN_ID};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_amount, get_decimals, nfpm_positions};

#[derive(Args)]
pub struct BurnPositionArgs {
    /// NFT token ID of the position to burn
    #[arg(long)]
    pub token_id: u128,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos
    #[arg(long)]
    pub dry_run: bool,
}

/// NFPM.burn(tokenId) — selector 0x42966c68
/// Destroys the NFT for a position that has zero liquidity and zero uncollected fees.
fn build_burn(token_id: u128) -> String {
    format!("0x42966c68{}", format!("{:0>64x}", token_id))
}

pub async fn run(args: BurnPositionArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let nfpm_addr = nfpm();

    let pos = nfpm_positions(nfpm_addr, args.token_id, rpc).await?;

    // Validate preconditions — contract will revert if these aren't met, so catch early
    if pos.liquidity > 0 {
        let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
        let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);
        let sym0 = token_symbol(&pos.token0);
        let sym1 = token_symbol(&pos.token1);
        anyhow::bail!(
            "Position {} still has liquidity ({} units). \
             Run `remove-liquidity --token-id {} --percent 100` first, then burn.\n\
             Token pair: {}/{}, ticks: [{}, {}]\n\
             Uncollected fees: {} {}, {} {}",
            args.token_id, pos.liquidity, args.token_id,
            sym0, sym1, pos.tick_lower, pos.tick_upper,
            format_amount(pos.tokens_owed0, dec0), sym0,
            format_amount(pos.tokens_owed1, dec1), sym1,
        );
    }
    if pos.tokens_owed0 > 0 || pos.tokens_owed1 > 0 {
        let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
        let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);
        let sym0 = token_symbol(&pos.token0);
        let sym1 = token_symbol(&pos.token1);
        anyhow::bail!(
            "Position {} has uncollected fees ({} {}, {} {}). \
             Run `collect-fees --token-id {}` first, then burn.",
            args.token_id,
            format_amount(pos.tokens_owed0, dec0), sym0,
            format_amount(pos.tokens_owed1, dec1), sym1,
            args.token_id,
        );
    }

    let sym0 = token_symbol(&pos.token0).to_string();
    let sym1 = token_symbol(&pos.token1).to_string();
    let calldata = build_burn(args.token_id);

    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    let preview = serde_json::json!({
        "preview": true,
        "action": "burn-position",
        "token_id": args.token_id,
        "token0": sym0,
        "token1": sym1,
        "tick_lower": pos.tick_lower,
        "tick_upper": pos.tick_upper,
        "wallet": wallet,
        "note": "This permanently destroys the NFT. The position has zero liquidity and zero fees.",
        "chain": "Base (8453)"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to permanently burn position {}.", args.token_id);
        return Ok(());
    }

    let result = wallet_contract_call(CHAIN_ID, nfpm_addr, &calldata, true, args.dry_run, Some(&wallet)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "burn-position",
        "token_id": args.token_id,
        "token0": sym0,
        "token1": sym1,
        "tx_hash": tx_hash,
        "explorer": format!("https://basescan.org/tx/{}", tx_hash),
    });
    if args.dry_run {
        out["dry_run"] = serde_json::json!(true);
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
