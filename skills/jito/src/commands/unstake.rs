use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::commands::derive_ata;
use crate::config;
use crate::onchainos;
use crate::rpc;

#[derive(Args)]
pub struct UnstakeArgs {
    /// Amount of JitoSOL to unstake
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (501 = Solana mainnet)
    #[arg(long, default_value_t = 501)]
    pub chain: u64,

    /// Preview the transaction without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: UnstakeArgs) -> Result<Value> {
    if args.chain != config::SOLANA_CHAIN_ID {
        anyhow::bail!("Jito only supports Solana (chain 501)");
    }
    if args.amount <= 0.0 {
        anyhow::bail!("Amount must be positive");
    }

    // Resolve wallet address
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet. Make sure onchainos is logged in.");
    }

    // Fetch stake pool state for rate conversion
    let pool_data = rpc::get_account_data(config::JITO_STAKE_POOL).await?;
    let pool_info = rpc::parse_stake_pool(&pool_data)?;

    let sol_per_jitosol = if pool_info.pool_token_supply > 0 {
        pool_info.total_lamports as f64 / pool_info.pool_token_supply as f64
    } else {
        1.0
    };
    let expected_sol = args.amount * sol_per_jitosol;

    // Pool token amount in raw units (lamport-equivalent)
    let pool_tokens_raw = (args.amount * config::LAMPORTS_PER_SOL as f64) as u64;

    // Derive user's JitoSOL ATA
    let user_ata_bytes = derive_ata(&wallet, config::JITOSOL_MINT)?;
    let user_ata = bs58::encode(&user_ata_bytes).into_string();

    // Check user balance
    let (jitosol_balance, _) = rpc::get_token_balance(&user_ata).await.unwrap_or((0.0, 0));
    if jitosol_balance < args.amount && !args.dry_run {
        anyhow::bail!(
            "Insufficient JitoSOL balance: have {:.9}, need {:.9}",
            jitosol_balance,
            args.amount
        );
    }

    let preview = serde_json::json!({
        "operation": "unstake",
        "wallet": wallet,
        "jitosol_amount": args.amount,
        "jitosol_raw": pool_tokens_raw.to_string(),
        "expected_sol": format!("{:.9}", expected_sol),
        "sol_per_jitosol_rate": format!("{:.8}", sol_per_jitosol),
        "user_jitosol_ata": user_ata,
        "current_jitosol_balance": format!("{:.9}", jitosol_balance),
        "stake_pool": config::JITO_STAKE_POOL,
        "delay_note": "Unstaking creates a stake account that unlocks after the current epoch (~2-3 days). You will need to manually deactivate and withdraw the stake account after the epoch ends.",
        "fee_note": "Unstake fee: ~0.3% of withdrawn amount",
        "note": "Ask user to confirm before submitting the unstake transaction"
    });

    if args.dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": preview
        }));
    }

    // NOTE: WithdrawStake (unstake) requires selecting a validator stake account from the
    // validator list, which involves fetching and parsing the validator list account.
    // This is the most complex part of the SPL stake pool interaction.
    //
    // For the initial implementation, we surface clear guidance and return a structured error
    // directing users to use the Jito webapp for unstaking, while stake is fully supported on-chain.
    //
    // Full WithdrawStake implementation would require:
    // 1. Fetch validator list account (from pool_info.validator_list)
    // 2. Parse all validator entries to find one with sufficient stake
    // 3. Generate an ephemeral keypair for the new stake account destination
    // 4. Build the WithdrawStake instruction with all required accounts
    // 5. Include the ephemeral keypair as an additional signer
    //
    // This complexity is deferred; the stake (DepositSol) flow is fully implemented.
    anyhow::bail!(
        "On-chain unstake requires selecting a validator stake account and signing with an ephemeral keypair. \
        This complex flow is currently dry-run only. Use the Jito webapp (jito.network) to complete the unstake, \
        or run with --dry-run to preview the operation.\n\
        Preview: unstake {:.9} JitoSOL → ~{:.9} SOL (after ~2-3 day epoch unlock)",
        args.amount,
        expected_sol
    )
}
