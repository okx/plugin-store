//! request-redeem — initiate async withdrawal via requestRedeem(shares, token)
//! Selector: 0x107703ab — requestRedeem(uint256,address)
//!
//! WARNING: Standard ERC4626 withdraw()/redeem() are DISABLED in CIAN vaults.
//! This is the ONLY supported withdrawal path. Shares are immediately transferred
//! to the rebalancer and cannot be cancelled. Assets arrive ~5 days later.

use crate::config::{
    CHAIN_ID, encode_address, encode_uint256, format_18, resolve_vault, resolve_token,
    WITHDRAWAL_WAIT_DAYS,
};
use crate::onchainos::{resolve_wallet, wallet_contract_call_force, extract_tx_hash};
use crate::rpc::{balance_of, preview_redeem};

pub async fn run(
    vault_input: &str,
    shares_str: &str,
    token_input: &str,
    from_override: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let vault = resolve_vault(vault_input)?;
    let (token_addr, _token_decimals) = resolve_token(token_input, vault.address)?;

    println!("=== CIAN Yield Layer — Request Redeem (Async Withdrawal) ===");
    println!("Chain:       Ethereum Mainnet (chain 1)");
    println!("Vault:       {} ({})", vault.name, vault.symbol);
    println!("Redeem to:   {} ({})", token_input.to_uppercase(), token_addr);
    println!();
    println!("IMPORTANT: This initiates an ASYNC withdrawal.");
    println!("  - Shares are transferred to the rebalancer IMMEDIATELY.");
    println!("  - This action is IRREVERSIBLE once submitted.");
    println!("  - Assets will be returned to your wallet in ~{} days.", WITHDRAWAL_WAIT_DAYS);
    println!("  - Standard withdraw()/redeem() are DISABLED in CIAN vaults.");
    println!();

    let wallet = if let Some(w) = from_override {
        w.to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    if wallet.is_empty() {
        anyhow::bail!("No wallet found on Ethereum (chain 1). Run: onchainos wallet login");
    }
    println!("Wallet: {}", wallet);

    // Parse shares amount
    let shares_raw: u128 = if shares_str.contains('.') {
        let f: f64 = shares_str.parse().map_err(|_| anyhow::anyhow!("invalid shares: {}", shares_str))?;
        (f * 1e18) as u128
    } else {
        shares_str.parse().map_err(|_| anyhow::anyhow!("invalid shares: {}", shares_str))?
    };

    println!("Shares to redeem: {} {} (raw: {})", shares_str, vault.symbol, shares_raw);

    // Verify user has enough shares
    let current_shares = balance_of(vault.address, &wallet).await.unwrap_or(0);
    println!("Current balance:  {} {}", format_18(current_shares), vault.symbol);

    if shares_raw > current_shares {
        anyhow::bail!(
            "Insufficient shares: requested {} but wallet only holds {}",
            format_18(shares_raw),
            format_18(current_shares)
        );
    }

    // Preview redeem
    if let Ok(assets_preview) = preview_redeem(vault.address, shares_raw).await {
        let decimals = vault.asset_decimals;
        let divisor = 10u128.pow(decimals);
        let whole = assets_preview / divisor;
        let frac = (assets_preview % divisor) / 10u128.pow(if decimals >= 6 { decimals - 6 } else { 0 });
        println!("Estimated assets: {}.{:06} (before exit fee, if any)", whole, frac);
        println!("Note: Exit fee may be up to 1.2% at operator's discretion.");
    }
    println!();

    if dry_run {
        println!("[DRY-RUN] Would call requestRedeem(shares={}, token={})", shares_raw, token_addr);
        println!("Run without --dry-run and confirm to execute.");
        return Ok(());
    }

    // requestRedeem(uint256 _shares, address _token)
    // Selector: 0x107703ab
    let shares_padded = encode_uint256(shares_raw);
    let token_padded = encode_address(token_addr);
    let calldata = format!("0x107703ab{}{}", shares_padded, token_padded);

    println!("Submitting requestRedeem...");
    println!("  Shares:      {}", shares_raw);
    println!("  Receive as:  {} ({})", token_input.to_uppercase(), token_addr);
    println!();
    println!("Ask user to confirm: This is IRREVERSIBLE. Shares transfer to rebalancer immediately.");
    println!("Assets arrive in ~{} days automatically.", WITHDRAWAL_WAIT_DAYS);
    println!();

    let result = wallet_contract_call_force(
        CHAIN_ID,
        vault.address,
        &calldata,
        Some(&wallet),
        None,
        Some(300_000),
        false,
    ).await?;
    if result["ok"].as_bool() != Some(true) {
        anyhow::bail!("request-redeem failed: {}", result["error"].as_str().unwrap_or("unknown error"));
    }

    let tx_hash = extract_tx_hash(&result);
    println!("Request Redeem TX: {}", tx_hash);
    println!("Etherscan:         https://etherscan.io/tx/{}", tx_hash);
    println!();
    println!("Redeem request submitted successfully.");
    println!("Your {} {} shares have been transferred to the rebalancer.", shares_str, vault.symbol);
    println!("Assets (~{} days) will be sent to: {}", WITHDRAWAL_WAIT_DAYS, wallet);
    println!("No further action needed. Track status: cian-yield-layer positions");

    Ok(())
}
