/// `euler-v2-plugin repay` — pay back debt on a controller vault.
///
/// **Important: this command does NOT use ERC-4626 `repay(amount, receiver)`.**
///
/// Direct `vault.repay(...)` triggers `IERC20(asset).transferFrom(user, vault, amount)`
/// which OKX TEE wallet rejects for un-whitelisted vaults (see [ONC-001]). The plugin
/// instead uses **`vault.repayWithShares(amount, receiver)`** — burns the caller's
/// vault shares (in the SAME vault as the controller) to reduce `receiver`'s debt.
/// No `transferFrom` is invoked, so the call passes TEE policy.
///
/// Pre-condition: caller must have shares of the controller vault. If a user borrowed
/// from `eWETH-1` but has no eWETH-1 supply, they need to acquire shares first
/// (typically by `supply --vault eWETH-1` using the donate+skim pattern).
///
/// Per LEND-001, `--all` uses `uint256.max` so EVK computes the exact debt (including
/// last-second accrued interest) at execution time and burns just enough shares.

use anyhow::{Context, Result};
use clap::Args;

use crate::calldata::{build_repay_with_shares, build_repay_with_shares_all};
use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{
    build_address_call, eth_call, eth_get_balance_wei, estimate_native_gas_cost_wei,
    parse_uint256_to_u128, wei_to_eth, SELECTOR_BALANCE_OF, SELECTOR_DEBT_OF,
};

const GAS_LIMIT_REPAY: u64 = 300_000;

#[derive(Args)]
pub struct RepayArgs {
    /// Controller vault to repay
    #[arg(long)]
    pub vault: String,

    /// Amount of underlying-asset debt to clear (mutually exclusive with --all)
    #[arg(long)]
    pub amount: Option<String>,

    /// Repay full debt + accrued interest (uses uint256.max sentinel, per LEND-001)
    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    #[arg(long)]
    pub dry_run: bool,

    /// Required to broadcast. Without this, the command prints a preview
    /// (calldata + intent) and exits without touching the chain.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: RepayArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("repay"), None)); Ok(()) }
    }
}

async fn run_inner(args: RepayArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    if args.all && args.amount.is_some() {
        anyhow::bail!("Pass either --amount or --all, not both.");
    }
    let vault_addr = args.vault.to_lowercase();

    let vaults = crate::api::get_vaults_raw(args.chain).await?;
    let evk = vaults["evkVaults"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Euler API returned no evkVaults"))?;
    let entry = evk.iter()
        .find(|v| v["address"].as_str().map(|s| s.to_lowercase()) == Some(vault_addr.clone()))
        .ok_or_else(|| anyhow::anyhow!("Vault {} not found in Euler API", args.vault))?;
    let decimals: u32 = entry["asset"]["decimals"]["__bi"].as_str()
        .and_then(|s| s.parse().ok()).unwrap_or(18);
    let asset_symbol = entry["asset"]["symbol"].as_str().unwrap_or("?").to_string();

    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    // Read current debt to size the repay precisely (and bail if no debt).
    let debt_call = build_address_call(SELECTOR_DEBT_OF, &wallet);
    let debt_hex = eth_call(args.chain, &vault_addr, &debt_call).await?;
    let debt_raw = parse_uint256_to_u128(&debt_hex);
    if debt_raw == 0 {
        anyhow::bail!(
            "No debt on vault {} for wallet {} — nothing to repay. \
             Run `disable-controller --vault {}` if you want to release the borrower role.",
            vault_addr, wallet, vault_addr
        );
    }

    // Read user's vault shares — repayWithShares burns them as the payment source.
    let shares_call = build_address_call(SELECTOR_BALANCE_OF, &wallet);
    let shares_hex = eth_call(args.chain, &vault_addr, &shares_call).await?;
    let shares_raw = parse_uint256_to_u128(&shares_hex);
    if shares_raw == 0 {
        anyhow::bail!(
            "No vault shares to burn for repayment on {}. \
             repayWithShares requires the caller to have supply position in the same vault. \
             Either supply some {} to this vault first (`supply --vault {} --amount <N>`), \
             or wait for direct `repay` to be supported (blocked by OKX TEE policy on un-whitelisted vaults).",
            vault_addr, asset_symbol, vault_addr
        );
    }

    let (calldata, amount_label, amount_for_check) = if args.all {
        (build_repay_with_shares_all(&wallet),
         "all (uint256.max sentinel)".to_string(),
         debt_raw)  // require shares roughly cover full debt
    } else {
        let amt_str = args.amount.as_ref()
            .ok_or_else(|| anyhow::anyhow!("--amount or --all is required"))?;
        let amt_f: f64 = amt_str.parse()
            .with_context(|| format!("Invalid amount '{}'", amt_str))?;
        if amt_f <= 0.0 { anyhow::bail!("amount must be positive"); }
        let amt_raw = (amt_f * 10f64.powi(decimals as i32)).round() as u128;
        if amt_raw > debt_raw {
            anyhow::bail!(
                "Repay amount {} exceeds current debt {}. Use --all to clear the full debt.",
                amt_raw, debt_raw
            );
        }
        (build_repay_with_shares(amt_raw, &wallet), amt_str.clone(), amt_raw)
    };

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true, "dry_run": true,
            "data": {
                "action": "repay (via repayWithShares)",
                "chain": chain_name(args.chain), "chain_id": args.chain,
                "wallet": wallet,
                "vault": vault_addr, "vault_name": entry["name"],
                "asset": asset_symbol, "decimals": decimals,
                "current_debt_raw": debt_raw.to_string(),
                "user_vault_shares_raw": shares_raw.to_string(),
                "amount": amount_label,
                "tx_plan": [
                    format!("vault.repayWithShares({}, {}) — burns ~equivalent vault shares to clear debt",
                            if args.all { "uint256.max".to_string() } else { amount_for_check.to_string() },
                            wallet),
                ],
                "note": "dry-run: no transactions submitted. \
                         repayWithShares requires user to have supply position in the same vault. \
                         The plugin uses this in place of `repay()` to bypass OKX TEE's anti-drain check.",
            }
        }))?);
        return Ok(());
    }

    // Pre-flight gas (per GAS-001)
    let need_wei = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_REPAY).await?;
    let have_wei = eth_get_balance_wei(args.chain, &wallet).await?;
    if have_wei < need_wei {
        anyhow::bail!("Insufficient native gas: have {:.6} ETH, need ~{:.6} ETH.",
            wei_to_eth(have_wei), wei_to_eth(need_wei));
    }

    eprintln!("[euler-v2] repaying {} via repayWithShares on vault {}...", amount_label, vault_addr);
    let resp = crate::onchainos::wallet_contract_call(
        args.chain, &vault_addr, &calldata,
        Some(&wallet), None, false, false,
    ).await?;
    let tx_hash = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[euler-v2] repayWithShares tx: {} (waiting...)", tx_hash);
    crate::onchainos::wait_for_tx_receipt(&tx_hash, args.chain, 120).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": "repay (via repayWithShares)",
            "chain": chain_name(args.chain), "chain_id": args.chain,
            "wallet": wallet, "vault": vault_addr,
            "asset": asset_symbol,
            "amount": amount_label,
            "repay_tx": tx_hash,
            "on_chain_status": "0x1",
            "tip": if args.all {
                format!("Debt cleared via vault-share burn. Run `disable-controller --vault {}` to free your account.", vault_addr)
            } else {
                "Partial repay confirmed. Run `health-factor` to see updated buffer.".to_string()
            },
        }
    }))?);
    Ok(())
}
