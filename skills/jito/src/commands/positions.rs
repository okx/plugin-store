use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::commands::derive_ata;
use crate::config;
use crate::onchainos;
use crate::rpc;

#[derive(Args)]
pub struct PositionsArgs {
    /// Chain ID (501 = Solana mainnet)
    #[arg(long, default_value_t = 501)]
    pub chain: u64,
}

pub async fn run(args: PositionsArgs) -> Result<Value> {
    if args.chain != config::SOLANA_CHAIN_ID {
        anyhow::bail!("Jito only supports Solana (chain 501)");
    }

    // Resolve wallet address
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet. Make sure onchainos is logged in.");
    }

    // Derive the user's JitoSOL ATA address
    let ata_bytes = derive_ata(&wallet, config::JITOSOL_MINT)?;
    let ata_address = bs58::encode(&ata_bytes).into_string();

    // Try ATA first; if empty or doesn't exist, fall back to getTokenAccountsByOwner
    let (jitosol_ui, jitosol_raw, actual_account) =
        get_best_jitosol_balance(&wallet, &ata_address).await;

    // Get current exchange rate for SOL equivalent
    let pool_data = rpc::get_account_data(config::JITO_STAKE_POOL).await?;
    let pool_info = rpc::parse_stake_pool(&pool_data)?;

    let sol_per_jitosol = if pool_info.pool_token_supply > 0 {
        pool_info.total_lamports as f64 / pool_info.pool_token_supply as f64
    } else {
        1.0
    };
    let sol_value = jitosol_ui * sol_per_jitosol;

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "wallet": wallet,
            "jitosol_token_account": actual_account,
            "jitosol_ata": ata_address,
            "jitosol_balance": format!("{:.9}", jitosol_ui),
            "jitosol_raw": jitosol_raw.to_string(),
            "sol_value": format!("{:.9}", sol_value),
            "sol_per_jitosol": format!("{:.8}", sol_per_jitosol),
            "chain": "Solana"
        }
    }))
}

/// Try ATA balance first, then fall back to getTokenAccountsByOwner
/// Returns (ui_balance, raw_balance, account_address)
async fn get_best_jitosol_balance(wallet: &str, ata: &str) -> (f64, u64, String) {
    // Try ATA balance
    if let Ok((ui, raw)) = rpc::get_token_balance(ata).await {
        if raw > 0 {
            return (ui, raw, ata.to_string());
        }
    }

    // Fall back to getTokenAccountsByOwner
    if let Ok((ui, raw, addr)) = rpc::get_token_accounts_by_owner(wallet, config::JITOSOL_MINT).await {
        return (ui, raw, addr);
    }

    (0.0, 0, ata.to_string())
}
