use clap::Args;
use serde_json::json;

use crate::onchainos;

const ABOUT: &str = "Meteora DLMM is a concentrated liquidity DEX on Solana — add liquidity to \
    dynamic bins, earn swap fees, and execute token swaps with tight spreads across \
    hundreds of pools including SOL/USDC, memecoins, and LST pairs.";

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

// Minimum SOL needed for fees + a minimum deposit
const MIN_SOL_GAS: f64 = 0.01;
// Minimum quote-side balance for a meaningful deposit or swap
const MIN_USDC: f64 = 1.0;

#[derive(Args, Debug)]
pub struct QuickstartArgs {
    /// Wallet address (Solana pubkey). If omitted, uses the currently logged-in wallet.
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(args: &QuickstartArgs) -> anyhow::Result<()> {
    // Resolve wallet
    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!(
                "Cannot resolve wallet. Pass --wallet or log in via onchainos.\nError: {e}"
            )
        })?
    };

    eprintln!("Checking assets for {}... on Solana...", &wallet[..8.min(wallet.len())]);

    // Fetch balances (sync onchainos CLI calls)
    let sol_balance  = onchainos::get_sol_balance(&wallet);
    let usdc_balance = onchainos::get_spl_token_balance(USDC_MINT);
    let usdt_balance = onchainos::get_spl_token_balance(USDT_MINT);

    let has_gas   = sol_balance  >= MIN_SOL_GAS;
    let has_quote = usdc_balance >= MIN_USDC || usdt_balance >= MIN_USDC;
    let has_sol_liquidity = sol_balance >= MIN_SOL_GAS + 0.001; // gas + a tiny deposit

    let quote_balance = if usdc_balance >= usdt_balance { usdc_balance } else { usdt_balance };
    let quote_example = format!("{:.2}", (quote_balance * 0.9).max(MIN_USDC).min(quote_balance));
    let sol_example = format!("{:.4}", (sol_balance - MIN_SOL_GAS).max(0.001).min(sol_balance - MIN_SOL_GAS));

    let (status, suggestion, onboarding_steps, next_command): (&str, &str, Vec<String>, String) =
        if has_gas && has_sol_liquidity && has_quote {
            (
                "ready",
                "You have both SOL and stablecoins — add two-sided liquidity or swap.",
                vec![
                    "1. Find a high-volume SOL/USDC pool:".to_string(),
                    "   meteora-plugin get-pools --search-term SOL-USDC".to_string(),
                    "2. Add two-sided liquidity (SpotBalanced):".to_string(),
                    format!(
                        "   meteora-plugin --confirm add-liquidity --pool <POOL_ADDRESS> --amount-x {} --amount-y {}",
                        sol_example, quote_example
                    ),
                    "3. Or swap stablecoins for SOL:".to_string(),
                    format!(
                        "   meteora-plugin --confirm swap --from-token {} --to-token So11111111111111111111111111111111111111112 --amount {}",
                        USDC_MINT, quote_example
                    ),
                ],
                "meteora-plugin get-pools --search-term SOL-USDC".to_string(),
            )
        } else if has_gas && !has_quote {
            (
                "ready_sol_only",
                "You have SOL but no stablecoins — add SOL-only liquidity above the active bin, or swap some SOL for USDC first.",
                vec![
                    "Option A — SOL-only liquidity deposit above the active bin:".to_string(),
                    format!(
                        "   meteora-plugin --confirm add-liquidity --pool <POOL_ADDRESS> --amount-x {}",
                        sol_example
                    ),
                    "Option B — swap SOL for USDC first:".to_string(),
                    format!(
                        "   meteora-plugin --confirm swap --from-token So11111111111111111111111111111111111111112 --to-token {} --amount {}",
                        USDC_MINT, sol_example
                    ),
                    "1. Find pools:".to_string(),
                    "   meteora-plugin get-pools --search-term SOL-USDC".to_string(),
                ],
                "meteora-plugin get-pools --search-term SOL-USDC".to_string(),
            )
        } else if !has_gas && has_quote {
            (
                "needs_gas",
                "You have stablecoins but need SOL for transaction fees. Send at least 0.01 SOL to your wallet.",
                vec![
                    format!("1. Send at least {} SOL (gas) to:", MIN_SOL_GAS),
                    format!("   {}", wallet),
                    "2. Run quickstart again:".to_string(),
                    "   meteora-plugin quickstart".to_string(),
                ],
                "meteora-plugin quickstart".to_string(),
            )
        } else {
            (
                "no_funds",
                "No SOL or stablecoins found. Send SOL (for gas + deposits) to get started.",
                vec![
                    format!("1. Send at least {} SOL (gas + deposit) to your wallet:", MIN_SOL_GAS + 0.001),
                    format!("   {}", wallet),
                    "2. Optionally also send USDC for two-sided liquidity:".to_string(),
                    format!("   USDC mint: {}", USDC_MINT),
                    "3. Run quickstart again:".to_string(),
                    "   meteora-plugin quickstart".to_string(),
                    "4. Browse pools:".to_string(),
                    "   meteora-plugin get-pools --search-term SOL-USDC".to_string(),
                ],
                "meteora-plugin quickstart".to_string(),
            )
        };

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": "solana",
        "assets": {
            "sol_balance": sol_balance,
            "usdc_balance": usdc_balance,
            "usdt_balance": usdt_balance,
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
