use serde_json::json;

use crate::config::{SOL_NATIVE_MINT, SOLANA_RPC_URL, USDC_SOLANA, USDT_SOLANA, WSOL_MINT};
use crate::onchainos;

const ABOUT: &str = "Orca Whirlpools is the leading concentrated liquidity DEX on Solana — \
    swap tokens with minimal slippage across hundreds of pools including SOL/USDC, \
    mSOL/SOL, and popular meme token pairs. $1B+ TVL.";

// Minimum SOL for transaction fees on Solana
const MIN_SOL_GAS: f64 = 0.01;
// Minimum USDC for a meaningful swap
const MIN_USDC: f64 = 1.0;

pub async fn run(confirm: bool) -> anyhow::Result<()> {
    let _ = confirm; // quickstart is always read-only

    // Resolve active Solana wallet
    let wallet = onchainos::resolve_wallet_solana().map_err(|e| {
        anyhow::anyhow!(
            "Cannot resolve Solana wallet. Log in via onchainos first.\nError: {e}"
        )
    })?;

    eprintln!("Checking assets for {}... on Solana...", &wallet[..8.min(wallet.len())]);

    // Fetch SOL and USDC/USDT balances in parallel
    let (sol_res, usdc_res, usdt_res) = tokio::join!(
        onchainos::get_sol_balance(&wallet, SOLANA_RPC_URL),
        onchainos::get_spl_balance(&wallet, USDC_SOLANA, SOLANA_RPC_URL),
        onchainos::get_spl_balance(&wallet, USDT_SOLANA, SOLANA_RPC_URL),
    );

    // get_sol_balance returns lamports (u64), convert to SOL
    let sol_lamports = sol_res.unwrap_or(0);
    let sol_balance  = sol_lamports as f64 / 1e9;
    let usdc_balance = usdc_res.unwrap_or(0.0);
    let usdt_balance = usdt_res.unwrap_or(0.0);

    let has_gas  = sol_balance  >= MIN_SOL_GAS;
    let has_usdc = usdc_balance >= MIN_USDC || usdt_balance >= MIN_USDC;
    let has_sol_to_swap = sol_balance >= MIN_SOL_GAS + 0.01; // gas + swap amount

    let quote_balance = if usdc_balance >= usdt_balance { usdc_balance } else { usdt_balance };
    let quote_mint    = if usdc_balance >= usdt_balance { USDC_SOLANA } else { USDT_SOLANA };
    let quote_example = format!("{:.2}", (quote_balance * 0.9).max(MIN_USDC).min(quote_balance));
    let sol_swap_amt  = format!("{:.4}", (sol_balance - MIN_SOL_GAS).max(0.01).min(sol_balance - MIN_SOL_GAS));

    let (status, suggestion, onboarding_steps, next_command): (&str, &str, Vec<String>, String) =
        if has_gas && has_usdc {
            (
                "ready",
                "Your wallet is funded with SOL and stablecoins. Swap or explore pools.",
                vec![
                    "1. Check available pools for a token pair:".to_string(),
                    format!(
                        "   orca-plugin get-pools --token-a {} --token-b {}",
                        WSOL_MINT, USDC_SOLANA
                    ),
                    "2. Get a swap quote first (no confirmation needed):".to_string(),
                    format!(
                        "   orca-plugin get-quote --from-token {} --to-token {} --amount {}",
                        quote_mint, SOL_NATIVE_MINT, quote_example
                    ),
                    "3. Execute the swap:".to_string(),
                    format!(
                        "   orca-plugin --confirm swap --from-token {} --to-token {} --amount {}",
                        quote_mint, SOL_NATIVE_MINT, quote_example
                    ),
                ],
                format!(
                    "orca-plugin get-quote --from-token {} --to-token {} --amount {}",
                    quote_mint, SOL_NATIVE_MINT, quote_example
                ),
            )
        } else if has_sol_to_swap && !has_usdc {
            (
                "ready_sol_only",
                "You have SOL. Swap some SOL for USDC or explore pools.",
                vec![
                    "1. Get a swap quote for SOL → USDC:".to_string(),
                    format!(
                        "   orca-plugin get-quote --from-token {} --to-token {} --amount {}",
                        SOL_NATIVE_MINT, USDC_SOLANA, sol_swap_amt
                    ),
                    "2. Execute the swap:".to_string(),
                    format!(
                        "   orca-plugin --confirm swap --from-token {} --to-token {} --amount {}",
                        SOL_NATIVE_MINT, USDC_SOLANA, sol_swap_amt
                    ),
                    "3. Or browse available pools:".to_string(),
                    format!(
                        "   orca-plugin get-pools --token-a {} --token-b {}",
                        WSOL_MINT, USDC_SOLANA
                    ),
                ],
                format!(
                    "orca-plugin get-quote --from-token {} --to-token {} --amount {}",
                    SOL_NATIVE_MINT, USDC_SOLANA, sol_swap_amt
                ),
            )
        } else if !has_gas && has_usdc {
            (
                "needs_gas",
                "You have stablecoins but need SOL for transaction fees. Send at least 0.01 SOL.",
                vec![
                    format!("1. Send at least {} SOL (gas fees) to:", MIN_SOL_GAS),
                    format!("   {}", wallet),
                    "2. Run quickstart again:".to_string(),
                    "   orca-plugin quickstart".to_string(),
                ],
                "orca-plugin quickstart".to_string(),
            )
        } else {
            (
                "no_funds",
                "No SOL or stablecoins found. Send SOL (for gas + swaps) to get started.",
                vec![
                    format!("1. Send at least {} SOL (gas + swap amount) to:", MIN_SOL_GAS + 0.01),
                    format!("   {}", wallet),
                    "2. Optionally send USDC for stable → SOL swaps:".to_string(),
                    format!("   USDC mint: {}", USDC_SOLANA),
                    "3. Run quickstart again:".to_string(),
                    "   orca-plugin quickstart".to_string(),
                    "4. Explore pools:".to_string(),
                    format!(
                        "   orca-plugin get-pools --token-a {} --token-b {}",
                        WSOL_MINT, USDC_SOLANA
                    ),
                ],
                "orca-plugin quickstart".to_string(),
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": "solana",
        "assets": {
            "sol_balance": format!("{:.6}", sol_balance),
            "usdc_balance": format!("{:.2}", usdc_balance),
            "usdt_balance": format!("{:.2}", usdt_balance),
        },
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = json!(onboarding_steps);
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
