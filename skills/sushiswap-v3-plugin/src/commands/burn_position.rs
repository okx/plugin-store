use clap::Args;
use crate::config::{chain_config, pad_u256, token_symbol};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::nfpm_positions;

#[derive(Args)]
pub struct BurnPositionArgs {
    /// Token ID of the position NFT to permanently destroy
    #[arg(long)]
    pub token_id: u128,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos (dry-run)
    #[arg(long)]
    pub dry_run: bool,
}

/// NFPM.burn(uint256 tokenId) — selector 0x42966c68
fn build_burn(token_id: u128) -> String {
    format!("0x42966c68{}", pad_u256(token_id))
}

pub async fn run(args: BurnPositionArgs, chain_id: u64) -> anyhow::Result<()> {
    let cfg = chain_config(chain_id)?;
    let rpc_owned = crate::config::rpc_url(chain_id)?;
    let rpc: &str = &rpc_owned;
    let nfpm = cfg.nfpm;

    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };

    // Fetch position and validate it is empty before burning
    let pos = nfpm_positions(nfpm, args.token_id, rpc).await.map_err(|_| {
        anyhow::anyhow!(
            "Position {} not found on {}. Verify the token ID is correct and the position \
             exists on this chain (it may have already been burned).",
            args.token_id, cfg.name
        )
    })?;

    if pos.liquidity > 0 {
        anyhow::bail!(
            "Position {} still has liquidity ({}). \
             Remove all liquidity first:\n  \
             sushiswap-v3 remove-liquidity --token-id {} --liquidity max --confirm",
            args.token_id, pos.liquidity, args.token_id
        );
    }
    if pos.tokens_owed0 > 0 || pos.tokens_owed1 > 0 {
        anyhow::bail!(
            "Position {} has uncollected fees (token0_owed={}, token1_owed={}). \
             Collect fees first:\n  \
             sushiswap-v3 collect-fees --token-id {} --confirm",
            args.token_id, pos.tokens_owed0, pos.tokens_owed1, args.token_id
        );
    }

    let sym0 = if token_symbol(&pos.token0, chain_id) != "UNKNOWN" {
        token_symbol(&pos.token0, chain_id).to_string()
    } else {
        pos.token0.clone()
    };
    let sym1 = if token_symbol(&pos.token1, chain_id) != "UNKNOWN" {
        token_symbol(&pos.token1, chain_id).to_string()
    } else {
        pos.token1.clone()
    };

    let preview = serde_json::json!({
        "preview": true,
        "action": "burn-position",
        "token_id": args.token_id,
        "token0":   sym0,
        "token1":   sym1,
        "fee_bps":  pos.fee,
        "wallet":   wallet,
        "chain":    cfg.name,
        "warning":  "This permanently destroys the position NFT. This action is irreversible.",
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to permanently burn this position NFT.");
        return Ok(());
    }

    let calldata = build_burn(args.token_id);
    let result = wallet_contract_call(chain_id, nfpm, &calldata, false, args.dry_run, Some(&wallet)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "burn-position",
        "token_id": args.token_id,
        "token0":   sym0,
        "token1":   sym1,
        "tx_hash":  tx_hash,
        "explorer": format!("{}/{}", cfg.explorer, tx_hash),
        "chain":    cfg.name,
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
