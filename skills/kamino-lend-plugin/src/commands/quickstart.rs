/// `kamino-lend quickstart` — onboarding status and suggested first command.

use anyhow::Result;

use crate::{api, config, onchainos};

const ABOUT: &str = "Kamino Lend is a leading lending protocol on Solana — supply assets to earn \
    yield and borrow against your collateral with real-time liquidation protection \
    and multi-market support across USDC, SOL, and more.";

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Minimum SOL required to cover a Solana transaction (lamports in float form).
const MIN_SOL_GAS: f64 = 0.01; // 0.01 SOL ≈ covers any Kamino tx
/// Minimum USDC to be considered "funded".
const MIN_USDC: f64 = 1.0;

pub async fn run(wallet_override: Option<&str>) -> Result<()> {
    let wallet = match wallet_override {
        Some(w) => w.to_string(),
        None => onchainos::resolve_wallet_solana()?,
    };

    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --wallet or ensure onchainos is logged in.");
    }

    eprintln!("Checking assets for {}...", &wallet[..wallet.len().min(10)]);

    // Fetch balances (sync subprocess calls)
    let sol_balance = onchainos::get_sol_balance();
    let usdc_balance = onchainos::get_spl_token_balance(USDC_MINT);

    // Check for active obligations (async API call)
    let obligations = api::get_obligations(config::MAIN_MARKET, &wallet).await.ok();
    let has_positions = obligations
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    let (status, suggestion, onboarding_steps, next_command) =
        build_suggestion(&wallet, sol_balance, usdc_balance, has_positions);

    let mut out = serde_json::json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "assets": {
            "sol_balance":  format!("{:.6}", sol_balance),
            "usdc_balance": format!("{:.4}", usdc_balance),
        },
        "status":       status,
        "suggestion":   suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = serde_json::json!(onboarding_steps);
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn build_suggestion(
    wallet: &str,
    sol: f64,
    usdc: f64,
    has_positions: bool,
) -> (&'static str, &'static str, Vec<String>, String) {
    // Case 1: active — has lending positions
    if has_positions {
        return (
            "active",
            "You have active lending positions on Kamino. Check your deposits and borrows.",
            vec![],
            format!("kamino-lend positions --wallet {}", wallet),
        );
    }

    // Case 2: ready — has gas + USDC to supply
    if sol >= MIN_SOL_GAS && usdc >= MIN_USDC {
        let supply_amount = (usdc * 0.9 * 100.0).floor() / 100.0;
        return (
            "ready",
            "Your wallet is funded. You can supply USDC to earn yield on Kamino.",
            vec![
                "1. Preview supplying USDC (dry-run):".to_string(),
                format!("   kamino-lend supply --token USDC --amount {:.2} --dry-run", supply_amount),
                "2. Execute supply:".to_string(),
                format!("   kamino-lend supply --token USDC --amount {:.2} --confirm", supply_amount),
                "3. Check your positions after:".to_string(),
                format!("   kamino-lend positions --wallet {}", wallet),
            ],
            format!("kamino-lend supply --token USDC --amount {:.2} --dry-run", supply_amount),
        );
    }

    // Case 3: has USDC but not enough SOL for gas
    if usdc >= MIN_USDC {
        return (
            "needs_gas",
            "You have USDC but need SOL for transaction fees. Send at least 0.01 SOL to your wallet.",
            vec![
                "1. Send at least 0.01 SOL to your Solana wallet:".to_string(),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm:".to_string(),
                "   kamino-lend quickstart".to_string(),
            ],
            "kamino-lend quickstart".to_string(),
        );
    }

    // Case 4: has SOL but no USDC
    if sol >= MIN_SOL_GAS {
        return (
            "needs_funds",
            "You have SOL for gas but need USDC or other assets to supply. Send at least 1 USDC to your wallet.",
            vec![
                "1. Send at least 1 USDC to your Solana wallet:".to_string(),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm:".to_string(),
                "   kamino-lend quickstart".to_string(),
                "3. Or supply SOL directly (once you have gas covered):".to_string(),
                "   kamino-lend supply --token SOL --amount 0.1 --dry-run".to_string(),
            ],
            "kamino-lend quickstart".to_string(),
        );
    }

    // Case 5: no funds
    (
        "no_funds",
        "No SOL or USDC found. Send SOL (for gas) and USDC or SOL to your wallet to get started.",
        vec![
            "1. Send at least 0.01 SOL and 1 USDC to your Solana wallet:".to_string(),
            format!("   {}", wallet),
            "2. Run quickstart again to confirm:".to_string(),
            "   kamino-lend quickstart".to_string(),
            "3. Preview supplying USDC:".to_string(),
            "   kamino-lend supply --token USDC --amount 1 --dry-run".to_string(),
            "4. Execute with --confirm when ready.".to_string(),
        ],
        "kamino-lend quickstart".to_string(),
    )
}
