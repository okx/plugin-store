use clap::Args;
use serde_json::Value;
use crate::{api, onchainos};

#[derive(Args)]
pub struct DepositArgs {
    /// Token symbol to deposit (e.g. wstETH, rETH, cbETH)
    #[arg(long, default_value = "wstETH")]
    pub token: String,

    /// Amount to deposit (human-readable, e.g. 0.01)
    #[arg(long)]
    pub amount: String,

    /// Vault address to deposit into (defaults to the largest vault for the token)
    #[arg(long)]
    pub vault: Option<String>,

    /// Recipient address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value = "1")]
    pub chain: u64,

    /// Simulate only — do not broadcast transaction
    #[arg(long, default_value = "false")]
    pub dry_run: bool,
}

pub async fn run(args: DepositArgs) -> anyhow::Result<Value> {
    // Resolve wallet — must be done AFTER dry_run check (but we need it for encoding)
    let wallet = if args.dry_run {
        args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
    } else {
        match args.from.clone() {
            Some(addr) => addr,
            None => {
                let w = onchainos::resolve_wallet(args.chain)?;
                if w.is_empty() {
                    anyhow::bail!("Cannot resolve wallet address. Pass --from or log in via onchainos.");
                }
                w
            }
        }
    };

    // Fetch vault info
    let vaults = api::fetch_vaults(100).await?;

    let vault_info = if let Some(ref addr) = args.vault {
        api::find_vault_by_address(&vaults, addr)
            .ok_or_else(|| anyhow::anyhow!("Vault {} not found", addr))?
    } else {
        api::find_vault_by_token(&vaults, &args.token)
            .ok_or_else(|| anyhow::anyhow!("No vault found for token {}. Use --vault <address> to specify.", args.token))?
    };

    let vault_name = vault_info.meta.as_ref()
        .and_then(|m| m.name.as_deref())
        .unwrap_or(&vault_info.token.symbol);

    let decimals = vault_info.token.decimals;
    let amount_raw = api::parse_amount(&args.amount, decimals)?;

    let usd_value = vault_info.token.usd_price.map(|p| {
        let amount_f = amount_raw as f64 / 10f64.powi(decimals as i32);
        p * amount_f
    });

    if args.dry_run {
        // Compute calldata for preview
        let token_addr = &vault_info.token.address;
        let vault_addr = &vault_info.address;

        let spender_clean = vault_addr.trim_start_matches("0x");
        let spender_padded = format!("{:0>64}", spender_clean);
        let amount_hex = format!("{:064x}", amount_raw);
        let approve_calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);

        let wallet_clean = wallet.trim_start_matches("0x");
        let wallet_padded = format!("{:0>64}", wallet_clean);
        let deposit_calldata = format!("0x47e7ef24{}{}", wallet_padded, amount_hex);

        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "action": "deposit",
            "vault_address": vault_addr,
            "vault_name": vault_name,
            "token_symbol": vault_info.token.symbol,
            "token_address": token_addr,
            "amount": args.amount,
            "amount_raw": amount_raw.to_string(),
            "usd_value": usd_value.map(|v| format!("${:.4}", v)),
            "step1_approve_calldata": approve_calldata,
            "step2_deposit_calldata": deposit_calldata,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" }
        }));
    }

    let token_addr = &vault_info.token.address;
    let vault_addr = vault_info.address.clone();

    // Step 1: ERC-20 approve
    // approve(address spender, uint256 amount) — selector 0x095ea7b3
    // User must confirm before this transaction is sent
    let approve_result = onchainos::erc20_approve(
        args.chain,
        token_addr,
        &vault_addr,
        amount_raw,
        Some(&wallet),
        false,
    ).await?;

    let approve_tx = onchainos::extract_tx_hash(&approve_result);

    // Wait for approve to be broadcast before submitting deposit (multi-step tx delay pattern)
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    // Step 2: Vault deposit
    // deposit(address onBehalfOf, uint256 amount) — selector 0x47e7ef24
    // Confirm deposit with user before proceeding
    let wallet_clean = wallet.trim_start_matches("0x");
    let wallet_padded = format!("{:0>64}", wallet_clean);
    let amount_hex = format!("{:064x}", amount_raw);
    let deposit_calldata = format!("0x47e7ef24{}{}", wallet_padded, amount_hex);

    let deposit_result = onchainos::wallet_contract_call(
        args.chain,
        &vault_addr,
        &deposit_calldata,
        Some(&wallet),
        None,
        false,
    ).await?;

    let deposit_tx = onchainos::extract_tx_hash(&deposit_result);

    Ok(serde_json::json!({
        "ok": true,
        "action": "deposit",
        "vault_address": vault_addr,
        "vault_name": vault_name,
        "token_symbol": vault_info.token.symbol,
        "amount": args.amount,
        "amount_raw": amount_raw.to_string(),
        "usd_value": usd_value.map(|v| format!("${:.4}", v)),
        "approve_txHash": approve_tx,
        "deposit_txHash": deposit_tx,
        "data": { "txHash": deposit_tx }
    }))
}
