//! deposit — ERC-20 approve + optionalDeposit(token, assets, receiver, referral)
//! Selector: 0x32507a5f — optionalDeposit(address,uint256,address,address)
//! For native ETH: token=address(0), assets=0, msg.value=amount (use --amt)

use crate::config::{
    CHAIN_ID, ZERO_ADDRESS, encode_address, encode_uint256,
    resolve_vault, resolve_token, format_units,
};
use crate::onchainos::{resolve_wallet, erc20_approve, wallet_contract_call_force, extract_tx_hash};
use crate::rpc::{allowance, preview_deposit};

pub async fn run(
    vault_input: &str,
    token_input: &str,
    amount_str: &str,
    from_override: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let vault = resolve_vault(vault_input)?;
    let (token_addr, token_decimals) = resolve_token(token_input, vault.address)?;

    println!("=== CIAN Yield Layer — Deposit ===");
    println!("Chain:   Ethereum Mainnet (chain 1)");
    println!("Vault:   {} ({})", vault.name, vault.symbol);
    println!("Token:   {} ({})", token_input.to_uppercase(), token_addr);

    let wallet = if let Some(w) = from_override {
        w.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    if wallet.is_empty() {
        anyhow::bail!("No wallet found on Ethereum (chain 1). Run: onchainos wallet login");
    }
    println!("Wallet:  {}", wallet);

    let is_eth = token_addr == ZERO_ADDRESS || token_input.to_lowercase() == "eth";

    // Parse amount
    let amount_raw: u128 = amount_str.parse().unwrap_or_else(|_| {
        // try float parsing
        let f: f64 = amount_str.parse().unwrap_or(0.0);
        let mult = 10u128.pow(token_decimals);
        (f * mult as f64) as u128
    });

    println!("Amount:  {} {} (raw: {})", amount_str, token_input.to_uppercase(), amount_raw);
    println!();

    if dry_run {
        println!("[DRY-RUN] Would execute:");
        if !is_eth {
            println!("  Step 1: approve {} → vault {}", token_input.to_uppercase(), vault.address);
        }
        println!("  Step 2: optionalDeposit(token={}, assets={}, receiver={}, referral={})",
            token_addr, amount_raw, wallet, ZERO_ADDRESS);

        // Preview shares
        if let Ok(shares) = preview_deposit(vault.address, amount_raw).await {
            println!("  Estimated shares: {} {}", format_units(shares, 18), vault.symbol);
        }
        println!();
        println!("Run without --dry-run and confirm to execute.");
        return Ok(());
    }

    // Step 1: ERC-20 approve (skip for native ETH)
    if !is_eth {
        // Check current allowance
        let current_allowance = allowance(token_addr, &wallet, vault.address).await.unwrap_or(0);
        if current_allowance < amount_raw {
            println!("Step 1/2: Approving {} → vault...", token_input.to_uppercase());
            println!("  Ask user to confirm the approve transaction.");
            println!("  Approving {} (amount: {})", token_addr, amount_raw);

            let approve_result = erc20_approve(
                CHAIN_ID,
                token_addr,
                vault.address,
                u128::MAX,
                Some(&wallet),
                false,
            ).await?;
            if approve_result["ok"].as_bool() != Some(true) {
                anyhow::bail!("approve failed: {}", approve_result["error"].as_str().unwrap_or("unknown error"));
            }
            let approve_hash = extract_tx_hash(&approve_result);
            println!("  Approve TX: {}", approve_hash);
            println!("  Etherscan: https://etherscan.io/tx/{}", approve_hash);
            println!();

            // Wait to avoid nonce conflict
            println!("  Waiting 3 seconds before deposit...");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        } else {
            println!("Step 1/2: Approve skipped (allowance sufficient: {})", current_allowance);
        }
    }

    // Step 2: optionalDeposit(address _token, uint256 _assets, address _receiver, address _referral)
    // Selector: 0x32507a5f
    let token_padded = encode_address(token_addr);
    let assets_padded = encode_uint256(if is_eth { 0 } else { amount_raw });
    let receiver_padded = encode_address(&wallet);
    let referral_padded = encode_address(ZERO_ADDRESS);

    let calldata = format!(
        "0x32507a5f{}{}{}{}",
        token_padded, assets_padded, receiver_padded, referral_padded
    );

    let step_label = if is_eth { "Step 1/1" } else { "Step 2/2" };
    println!("{}: Calling optionalDeposit...", step_label);
    println!("  Vault:    {}", vault.address);
    println!("  Token:    {}", token_addr);
    println!("  Amount:   {}", if is_eth { amount_raw } else { amount_raw });
    println!("  Receiver: {}", wallet);
    println!();
    println!("  Ask user to confirm the deposit transaction.");

    let amt_wei: Option<u64> = if is_eth {
        Some(amount_raw as u64)
    } else {
        None
    };

    let deposit_result = wallet_contract_call_force(
        CHAIN_ID,
        vault.address,
        &calldata,
        Some(&wallet),
        amt_wei,
        Some(600_000),
        false,
    ).await?;

    if deposit_result["ok"].as_bool() != Some(true) {
        anyhow::bail!("deposit failed: {}", deposit_result["error"].as_str().unwrap_or("unknown error"));
    }
    let tx_hash = extract_tx_hash(&deposit_result);
    println!("Deposit TX: {}", tx_hash);
    println!("Etherscan:  https://etherscan.io/tx/{}", tx_hash);
    println!();
    println!("Deposit submitted. Check balance with: cian-yield-layer balance");

    Ok(())
}
