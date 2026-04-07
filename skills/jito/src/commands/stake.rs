use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::commands::{
    build_unsigned_transaction, derive_ata, derive_withdraw_authority, encode_transaction_base64,
    SolanaInstruction, SolanaMessage,
};
use crate::config;
use crate::onchainos;
use crate::rpc;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of SOL to stake
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (501 = Solana mainnet)
    #[arg(long, default_value_t = 501)]
    pub chain: u64,

    /// Preview the transaction without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs) -> Result<Value> {
    if args.chain != config::SOLANA_CHAIN_ID {
        anyhow::bail!("Jito only supports Solana (chain 501)");
    }
    if args.amount <= 0.0 {
        anyhow::bail!("Amount must be positive");
    }
    let lamports = (args.amount * config::LAMPORTS_PER_SOL as f64) as u64;
    if lamports < 100_000 {
        // Minimum ~0.0001 SOL to avoid dust
        anyhow::bail!("Minimum stake amount is 0.0001 SOL");
    }

    // Resolve wallet address
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet. Make sure onchainos is logged in.");
    }

    // Fetch stake pool state
    let pool_data = rpc::get_account_data(config::JITO_STAKE_POOL).await?;
    let pool_info = rpc::parse_stake_pool(&pool_data)?;

    let reserve_stake = bs58::encode(&pool_info.reserve_stake).into_string();
    let pool_mint = bs58::encode(&pool_info.pool_mint).into_string();

    // Verify pool mint matches our expected JitoSOL mint
    if pool_mint != config::JITOSOL_MINT {
        anyhow::bail!("Pool mint mismatch: expected {} got {}", config::JITOSOL_MINT, pool_mint);
    }

    // Calculate expected JitoSOL to receive
    let sol_per_jitosol = if pool_info.pool_token_supply > 0 {
        pool_info.total_lamports as f64 / pool_info.pool_token_supply as f64
    } else {
        1.0
    };
    let expected_jitosol = args.amount / sol_per_jitosol;

    // Derive PDAs
    let withdraw_authority_bytes = derive_withdraw_authority(config::JITO_STAKE_POOL)?;
    let withdraw_authority = bs58::encode(&withdraw_authority_bytes).into_string();

    // Resolve user's JitoSOL token account:
    // Try existing token accounts first (getTokenAccountsByOwner),
    // fall back to computing the canonical ATA address.
    let (user_token_account, user_token_account_bytes) =
        resolve_jitosol_token_account(&wallet).await?;

    // Preview for dry-run
    let preview = serde_json::json!({
        "operation": "stake",
        "wallet": wallet,
        "sol_amount": args.amount,
        "lamports": lamports.to_string(),
        "expected_jitosol": format!("{:.9}", expected_jitosol),
        "sol_per_jitosol_rate": format!("{:.8}", sol_per_jitosol),
        "user_jitosol_token_account": user_token_account,
        "stake_pool": config::JITO_STAKE_POOL,
        "reserve_stake": reserve_stake,
        "withdraw_authority": withdraw_authority,
        "note": "Ask user to confirm before submitting the stake transaction",
        "unstake_note": "JitoSOL earns MEV-enhanced staking rewards (~5-10% APY)"
    });

    if args.dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": preview
        }));
    }

    // Build the Solana transaction
    let blockhash = rpc::get_latest_blockhash().await?;
    let blockhash_bytes = bs58::decode(&blockhash)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid blockhash: {}", e))?;

    // Account key table for DepositSol (no create_ata — using existing token account):
    //
    // DepositSol accounts (SPL Stake Pool v0.7):
    //   0. stake_pool (w)
    //   1. withdraw_authority (r) — PDA
    //   2. reserve_stake (w) — receives the SOL
    //   3. from = wallet (w, s) — lamports source
    //   4. dest_token_account = user JitoSOL account (w) — receives JitoSOL
    //   5. manager_fee_account (w)
    //   6. referrer_fee_account = same as dest (w)
    //   7. pool_mint (w) — JitoSOL mint
    //   8. system_program (r)
    //   9. token_program (r)
    //
    // Account key ordering in message:
    //   [writable-signers] [writable-non-signers] [readonly-signers] [readonly-non-signers]
    //
    // Writable + signer: wallet (0)
    // Writable, non-signer: stake_pool(1), reserve_stake(2), user_token_account(3),
    //                        manager_fee_account(4), pool_mint(5)
    // Readonly, non-signer: withdraw_authority(6), system_program(7), token_program(8), stake_pool_program(9)

    let wallet_bytes = bs58::decode(&wallet)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid wallet: {}", e))?;
    let stake_pool_bytes = bs58::decode(config::JITO_STAKE_POOL)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid stake pool: {}", e))?;
    let system_program_bytes = bs58::decode(config::SYSTEM_PROGRAM)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid system program: {}", e))?;
    let token_program_bytes = bs58::decode(config::TOKEN_PROGRAM)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid token program: {}", e))?;
    let stake_pool_program_bytes = bs58::decode(config::STAKE_POOL_PROGRAM)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid stake pool program: {}", e))?;

    let account_keys: Vec<Vec<u8>> = vec![
        wallet_bytes.clone(),                            // 0: wallet (signer, writable)
        stake_pool_bytes,                                // 1: stake_pool (writable)
        pool_info.reserve_stake.clone(),                 // 2: reserve_stake (writable)
        user_token_account_bytes.clone(),                // 3: user JitoSOL token account (writable)
        pool_info.manager_fee_account.clone(),           // 4: manager_fee_account (writable)
        bs58::decode(config::JITOSOL_MINT)
            .into_vec()
            .unwrap(),                                   // 5: pool_mint (writable)
        withdraw_authority_bytes.clone(),                // 6: withdraw_authority (readonly)
        system_program_bytes,                            // 7: system_program (readonly)
        token_program_bytes,                             // 8: token_program (readonly)
        stake_pool_program_bytes,                        // 9: stake_pool_program (readonly)
    ];

    // Header: 1 required sig (wallet), 0 readonly signed, 4 readonly unsigned
    let num_required_sigs = 1u8;
    let num_readonly_signed = 0u8;
    let num_readonly_unsigned = 4u8; // withdraw_authority, system_program, token_program, stake_pool_program

    // DepositSol instruction: discriminator=14, lamports as u64 LE
    let mut deposit_data = vec![14u8];
    deposit_data.extend_from_slice(&lamports.to_le_bytes());

    let deposit_sol_ix = SolanaInstruction {
        program_id_index: 9, // stake_pool_program
        account_indices: vec![
            1, // stake_pool (writable)
            6, // withdraw_authority (readonly)
            2, // reserve_stake (writable)
            0, // wallet/from (writable, signer)
            3, // user JitoSOL token account dest (writable)
            4, // manager_fee_account (writable)
            3, // referrer_fee = same as dest (writable)
            5, // pool_mint (writable)
            7, // system_program
            8, // token_program
        ],
        data: deposit_data,
    };

    // Build v0 versioned message
    let message = SolanaMessage {
        num_required_sigs,
        num_readonly_signed,
        num_readonly_unsigned,
        account_keys,
        recent_blockhash: blockhash_bytes,
        instructions: vec![deposit_sol_ix],
    };

    let msg_bytes = message.serialize();
    let tx_bytes = build_unsigned_transaction(&msg_bytes);
    let tx_base64 = encode_transaction_base64(&tx_bytes);

    // Submit via onchainos
    let result = onchainos::wallet_contract_call_solana(
        config::STAKE_POOL_PROGRAM,
        &tx_base64,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "operation": "stake",
            "sol_amount": args.amount,
            "expected_jitosol": format!("{:.9}", expected_jitosol),
            "wallet": wallet,
            "solscan": format!("https://solscan.io/tx/{}", tx_hash),
            "note": preview
        }
    }))
}

/// Resolve the user's JitoSOL token account address.
/// Tries getTokenAccountsByOwner first (handles non-ATA accounts),
/// falls back to computing the canonical ATA.
async fn resolve_jitosol_token_account(wallet: &str) -> Result<(String, Vec<u8>)> {
    // Try getTokenAccountsByOwner to find existing JitoSOL accounts
    if let Ok((_ui, _raw, addr)) =
        rpc::get_token_accounts_by_owner(wallet, config::JITOSOL_MINT).await
    {
        if !addr.is_empty() {
            let bytes = bs58::decode(&addr)
                .into_vec()
                .map_err(|e| anyhow::anyhow!("Invalid token account address: {}", e))?;
            return Ok((addr, bytes));
        }
    }

    // Fall back to canonical ATA derivation
    let ata_bytes = derive_ata(wallet, config::JITOSOL_MINT)?;
    let ata_addr = bs58::encode(&ata_bytes).into_string();
    Ok((ata_addr, ata_bytes))
}
