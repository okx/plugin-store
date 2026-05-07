/// `polymarket setup-deposit-wallet` — deploy a Polymarket deposit wallet and configure
/// POLY_1271 trading mode. One-time relayer-funded setup; all subsequent trading is gasless.
///
/// Flow (6 steps):
///   1. Identify owner EOA from onchainos.
///   2. Check for existing deposit wallet on-chain (idempotent — skips deploy if found).
///   3. Deploy via relayer WALLET-CREATE.
///   4. Submit pUSD + CTF ERC-1155 approvals as a signed WALLET batch.
///   5. Sync CLOB balance-allowance endpoint with signature_type=3.
///   6. Save deposit_wallet address and switch mode to DepositWallet in creds.json.
///
/// Existing EOA / PolyProxy users are NOT affected — this command is only needed once
/// by new users. Existing users' creds.json already has a mode set and this command
/// is never triggered automatically.
use anyhow::Result;
use reqwest::Client;

use crate::api::{get_builder_api_key, get_wallet_nonce, relayer_wallet_batch, relayer_wallet_create, sync_balance_allowance_deposit_wallet, WalletCreateResult};
use crate::auth::{ensure_credentials, ensure_credentials_deposit_wallet};
use crate::config::{Contracts, TradingMode};
use crate::onchainos::{get_wallet_address, get_pusd_balance};
use crate::signing::{sign_batch_via_onchainos, BatchParams, WalletCall};

pub async fn run(dry_run: bool) -> Result<()> {
    match run_inner(dry_run).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("setup-deposit-wallet"), None)); Ok(()) }
    }
}

async fn run_inner(dry_run: bool) -> Result<()> {
    let client = Client::new();
    let owner_addr = get_wallet_address().await?;

    eprintln!("[polymarket] Setting up deposit wallet for EOA: {}", owner_addr);

    // ── Step 1: check for existing deposit wallet ─────────────────────────────
    let existing = crate::onchainos::get_existing_deposit_wallet(&owner_addr).await;

    if let Some(ref addr) = existing {
        eprintln!("[polymarket] Existing deposit wallet found: {}", addr);
    }

    if dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "owner": owner_addr,
                    "existing_wallet": existing,
                    "factory": Contracts::DEPOSIT_WALLET_FACTORY,
                    "note": if existing.is_some() {
                        "dry-run: deposit wallet already deployed — would skip deploy, re-run approvals + balance sync"
                    } else {
                        "dry-run: would deploy via relayer WALLET-CREATE, then approve pUSD + CTF, then sync CLOB"
                    }
                }
            }))?
        );
        return Ok(());
    }

    // ── Step 1b: derive builder credentials (needed for relayer auth) ─────────
    // Builder credentials are derived from the user's CLOB API key via a single
    // POST /auth/builder-api-key call — no Polymarket app or Builders Program needed.
    eprintln!("[polymarket] Deriving builder credentials for relayer access...");
    let clob_creds = ensure_credentials(&client, &owner_addr).await?;
    let builder = get_builder_api_key(&client, &clob_creds, &owner_addr).await
        .map_err(|e| anyhow::anyhow!(
            "Could not derive builder credentials: {}. \
             Ensure your CLOB API key is valid (run `polymarket balance` to verify).",
            e
        ))?;
    eprintln!("[polymarket] Builder credentials obtained.");

    // ── Step 2: deploy wallet if not already deployed ─────────────────────────
    let wallet_addr = if let Some(addr) = existing {
        eprintln!("[polymarket] Skipping deploy — deposit wallet already exists.");
        addr
    } else {
        eprintln!("[polymarket] Deploying deposit wallet via relayer (WALLET-CREATE)...");

        // Gasless deployment via the Polymarket relayer (WALLET-CREATE, no user signature required).
        // The relayer pays gas and deploys a deterministic ERC-1967 proxy.
        // Builder auth (POLY_BUILDER_* headers) authorises the relayer to call the factory's
        // OnlyOperator deploy() function on behalf of the EOA.
        match relayer_wallet_create(&client, &owner_addr, &builder).await
            .map_err(|e| anyhow::anyhow!("Deposit wallet deployment failed: {}", e))?
        {
            WalletCreateResult::Transaction(tx_hash) => {
                // Fresh deployment — wait for tx, then extract wallet address from factory event
                eprintln!("[polymarket] WALLET-CREATE submitted: {}", tx_hash);
                eprintln!("[polymarket] Waiting for deployment tx to confirm...");
                crate::onchainos::wait_for_wallet_create_receipt(&tx_hash, 120).await
                    .map_err(|e| anyhow::anyhow!("Deposit wallet deploy confirmation failed: {}", e))?
            }
            WalletCreateResult::AlreadyDeployed(addr) => {
                // Wallet already exists — relayer returned address directly
                eprintln!("[polymarket] Deposit wallet already deployed (relayer confirmed): {}", addr);
                addr
            }
            WalletCreateResult::Failed => {
                // The relayer reports STATE_FAILED — this almost always means the factory
                // reverted because a wallet already exists in the owner mapping.
                // Try to find the existing wallet via event log scan (handles pre-upgrade deployments).
                eprintln!("[polymarket] WALLET-CREATE returned STATE_FAILED — searching for existing wallet...");
                crate::onchainos::get_existing_deposit_wallet(&owner_addr).await
                    .ok_or_else(|| anyhow::anyhow!(
                        "Deposit wallet deployment failed (STATE_FAILED) and no existing wallet \
                         could be found in the last 150,000 blocks. \
                         If you believe you already have a deposit wallet, check Polygonscan \
                         for transactions from your address to the factory \
                         (0x00000000000Fb5C9ADea0298D729A0CB3823Cc07) and re-run this command \
                         with the wallet address stored in creds.json manually."
                    ))?
            }
        }
    };

    // ── Step 3: build approval batch calls ───────────────────────────────────
    // Approvals needed for POLY_1271 (sig_type=3) trading:
    //   a. pUSD → CTF Exchange V2 (normal markets)
    //   b. pUSD → Neg Risk CTF Exchange V2 (neg_risk markets)
    //   c. CTF ERC-1155 setApprovalForAll → CTF Exchange V2 (sell / redeem)
    //   d. CTF ERC-1155 setApprovalForAll → Neg Risk CTF Exchange V2 (sell neg_risk)
    //   e. CTF ERC-1155 setApprovalForAll → Neg Risk Adapter (redeem neg_risk)
    let calls = build_approval_calls();

    eprintln!("[polymarket] Submitting {} approval calls via WALLET batch...", calls.len());

    // ── Step 4: fetch nonce and sign batch ────────────────────────────────────
    let nonce = get_wallet_nonce(&client, &owner_addr).await
        .map_err(|e| anyhow::anyhow!("Could not fetch wallet nonce from relayer: {}", e))?;

    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + 300; // 5-minute window

    let calls_json: Vec<serde_json::Value> = calls.iter().map(|c| serde_json::json!({
        "target": c.target,
        "value":  c.value.to_string(),
        "data":   c.data,
    })).collect();

    let batch_params = BatchParams {
        wallet: wallet_addr.clone(),
        nonce,
        deadline,
        calls,
    };

    let batch_sig = sign_batch_via_onchainos(&batch_params).await
        .map_err(|e| anyhow::anyhow!("Batch signing failed: {}", e))?;

    let batch_tx = relayer_wallet_batch(
        &client,
        &owner_addr,
        &wallet_addr,
        nonce,
        deadline,
        calls_json,
        &batch_sig,
        &builder,
    ).await?;

    eprintln!("[polymarket] Approval batch submitted: {}", batch_tx);
    eprintln!("[polymarket] Waiting for approval batch to confirm...");
    crate::onchainos::wait_for_tx_receipt(&batch_tx, 120).await?;
    eprintln!("[polymarket] Approvals confirmed.");

    // ── Step 5: sync CLOB balance-allowance ──────────────────────────────────
    eprintln!("[polymarket] Syncing CLOB balance-allowance (signature_type=3)...");
    sync_balance_allowance_deposit_wallet(&client, &wallet_addr, &owner_addr, &clob_creds).await
        .unwrap_or_else(|e| eprintln!("[polymarket] Warning: balance sync failed ({}); retry with `polymarket balance`.", e));

    // ── Step 5b: derive CLOB credentials for the deposit wallet address ───────
    // For POLY_1271 orders, the CLOB validates: order.signer == address_of(API_KEY).
    // Since the deposit wallet IS the order signer, it needs its own CLOB API key.
    // We sign the credential challenge with the active onchainos key (EOA); the CLOB
    // verifies via ERC-1271 by calling deposit_wallet.isValidSignature(hash, sig).
    // POLY_SIGNATURE_TYPE: 3 in the auth headers enables this ERC-1271 verification path.
    eprintln!("[polymarket] Deriving CLOB credentials for deposit wallet (ERC-1271 / POLY_SIGNATURE_TYPE=3)...");
    match ensure_credentials_deposit_wallet(&client, &wallet_addr).await {
        Ok(_) => eprintln!("[polymarket] Deposit wallet CLOB credentials registered."),
        Err(e) => eprintln!(
            "[polymarket] Warning: couldn't register deposit wallet with CLOB ({}). \
             Will retry on first buy/sell.", e
        ),
    }

    // ── Step 6: save mode + wallet address ───────────────────────────────────
    let mut updated_creds = clob_creds;
    updated_creds.deposit_wallet = Some(wallet_addr.clone());
    updated_creds.mode = TradingMode::DepositWallet;
    crate::config::save_credentials(&updated_creds)?;

    let eoa_pusd = get_pusd_balance(&owner_addr).await.unwrap_or(0.0);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "owner":                owner_addr,
                "eoa_pusd_balance":     eoa_pusd,
                "deposit_wallet":       wallet_addr,
                "mode":                 "deposit_wallet",
                "approval_tx":          batch_tx,
                "note": if eoa_pusd > 0.0 {
                    format!(
                        "Deposit wallet ready. Transfer {:.2} pUSD from your EOA to the deposit_wallet address, then retry your order.",
                        eoa_pusd
                    )
                } else {
                    "Deposit wallet ready. Fund it with pUSD and trade with signature_type=3 (POLY_1271). All trading is gasless.".to_string()
                }
            }
        }))?
    );
    Ok(())
}

/// Build the 5 approval calls required for full deposit wallet trading capability.
fn build_approval_calls() -> Vec<WalletCall> {
    use sha3::{Digest, Keccak256};

    // ERC-20 approve(address spender, uint256 amount) — approve max uint256
    let approve_selector = hex::encode(&Keccak256::digest(b"approve(address,uint256)")[..4]);
    // ERC-1155 setApprovalForAll(address operator, bool approved)
    let set_approval_selector = hex::encode(&Keccak256::digest(b"setApprovalForAll(address,bool)")[..4]);

    let max_uint = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    let make_erc20_approve = |token: &str, spender: &str| -> WalletCall {
        WalletCall {
            target: token.to_string(),
            value: 0,
            data: format!("0x{}{}{}", approve_selector, pad_address_hex(spender), max_uint),
        }
    };

    let make_set_approval = |operator: &str| -> WalletCall {
        WalletCall {
            target: Contracts::CTF.to_string(),
            value: 0,
            data: format!(
                "0x{}{}0000000000000000000000000000000000000000000000000000000000000001",
                set_approval_selector,
                pad_address_hex(operator)
            ),
        }
    };

    vec![
        // a. pUSD → CTF Exchange V2
        make_erc20_approve(Contracts::PUSD, Contracts::CTF_EXCHANGE_V2),
        // b. pUSD → Neg Risk CTF Exchange V2
        make_erc20_approve(Contracts::PUSD, Contracts::NEG_RISK_CTF_EXCHANGE_V2),
        // c. CTF → CTF Exchange V2 (sell / redeem)
        make_set_approval(Contracts::CTF_EXCHANGE_V2),
        // d. CTF → Neg Risk CTF Exchange V2
        make_set_approval(Contracts::NEG_RISK_CTF_EXCHANGE_V2),
        // e. CTF → Neg Risk Adapter (redeem neg_risk)
        make_set_approval(Contracts::NEG_RISK_ADAPTER),
    ]
}

/// Pad an address to a 32-byte ABI word (left-padded with zeros, no 0x prefix).
fn pad_address_hex(addr: &str) -> String {
    let stripped = addr.trim_start_matches("0x");
    format!("{:0>64}", stripped)
}

