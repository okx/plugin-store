use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config;
use crate::rpc;

#[derive(Args)]
pub struct RatesArgs {
    /// Chain ID (501 = Solana mainnet)
    #[arg(long, default_value_t = 501)]
    pub chain: u64,
}

pub async fn run(args: RatesArgs) -> Result<Value> {
    if args.chain != config::SOLANA_CHAIN_ID {
        anyhow::bail!("Jito only supports Solana (chain 501)");
    }

    // Fetch stake pool account data
    let pool_data = rpc::get_account_data(config::JITO_STAKE_POOL).await?;
    let pool_info = rpc::parse_stake_pool(&pool_data)?;

    let total_sol = pool_info.total_lamports as f64 / config::LAMPORTS_PER_SOL as f64;
    let total_jitosol = pool_info.pool_token_supply as f64 / config::LAMPORTS_PER_SOL as f64;

    let sol_per_jitosol = if pool_info.pool_token_supply > 0 {
        pool_info.total_lamports as f64 / pool_info.pool_token_supply as f64
    } else {
        1.0
    };
    let jitosol_per_sol = if sol_per_jitosol > 0.0 {
        1.0 / sol_per_jitosol
    } else {
        1.0
    };

    // Fetch APY from DeFiLlama (free, no auth)
    let apy = fetch_defillama_apy().await.unwrap_or(5.89);

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "protocol": "Jito",
            "chain": "Solana",
            "stake_pool": config::JITO_STAKE_POOL,
            "jitosol_mint": config::JITOSOL_MINT,
            "sol_per_jitosol": format!("{:.8}", sol_per_jitosol),
            "jitosol_per_sol": format!("{:.8}", jitosol_per_sol),
            "total_staked_sol": format!("{:.4}", total_sol),
            "total_jitosol_supply": format!("{:.4}", total_jitosol),
            "estimated_apy_pct": format!("{:.2}", apy),
            "fee_note": "Epoch fee: ~5% of staking rewards. Deposit fee: 0%. Withdrawal fee: ~0.3% (delayed unstake).",
            "unstake_note": "Unstaking creates a stake account that unlocks after the current epoch (~2-3 days)."
        }
    }))
}

/// Fetch JitoSOL APY from DeFiLlama yields API (project: jito-liquid-staking)
async fn fetch_defillama_apy() -> Result<f64> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://yields.llama.fi/pools")
        .send()
        .await?
        .json::<Value>()
        .await?;

    let pools = resp["data"].as_array().ok_or_else(|| anyhow::anyhow!("No data"))?;
    for pool in pools {
        if pool["project"].as_str() == Some("jito-liquid-staking")
            && pool["chain"].as_str() == Some("Solana")
        {
            return Ok(pool["apy"].as_f64().unwrap_or(5.89));
        }
    }
    Ok(5.89)
}
