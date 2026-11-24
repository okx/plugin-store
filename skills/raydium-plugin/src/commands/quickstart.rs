/// quickstart: Check wallet state and emit guided onboarding steps for new users.
///
/// Flow:
///   1. Resolve Solana wallet address (sync, via onchainos)
///   2. Fetch SOL and USDC balances in parallel via Solana RPC
///   3. Emit JSON with status + next steps
use anyhow::Result;
use crate::onchainos;

const SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
const USDC_DECIMALS: f64 = 1_000_000.0;
const MIN_SOL_GAS: f64 = 0.01;
const MIN_USDC_RAW: u64 = 1_000_000; // 1 USDC

pub async fn run() -> Result<()> {
    // Resolve wallet (sync)
    let wallet = onchainos::resolve_wallet_solana()?;

    // Progress to stderr
    let short = &wallet[..wallet.len().min(8)];
    eprintln!("Checking assets for {}... on Solana...", short);

    // Fetch SOL and USDC balances in parallel
    let (sol_res, usdc_res) = tokio::join!(
        onchainos::get_sol_balance(&wallet, SOLANA_RPC_URL),
        onchainos::get_spl_token_balance(&wallet, USDC_MINT, SOLANA_RPC_URL),
    );

    // Tolerate RPC errors silently — quickstart is a best-effort status probe,
    // not a trading command. An RPC blip surfaces as "no_funds" and the user
    // re-runs; cheaper than failing the whole onboarding flow.
    let lamports = sol_res.unwrap_or(0);
    let usdc_raw = usdc_res.unwrap_or(0);

    let sol = lamports as f64 / LAMPORTS_PER_SOL;
    let usdc_balance = usdc_raw as f64 / USDC_DECIMALS;
    let sol_str = format!("{:.6}", sol);
    let usdc_str = format!("{:.6}", usdc_balance);

    let has_gas = sol >= MIN_SOL_GAS;
    let has_usdc = usdc_raw >= MIN_USDC_RAW;
    let has_sol = sol > 0.0;

    let (status, suggestion, next_command, onboarding_steps) = if has_usdc && !has_gas {
        // Has USDC but not enough SOL for gas
        let steps = serde_json::json!([
            {
                "step": 1,
                "description": "Send at least 0.01 SOL to your wallet for gas fees:",
                "wallet": wallet,
                "note": "Minimum recommended: 0.01 SOL (covers transaction fees)"
            },
            {
                "step": 2,
                "description": "Run quickstart again:",
                "command": "raydium-plugin quickstart"
            }
        ]);
        (
            "needs_gas",
            "You have USDC but need SOL for gas. Send at least 0.01 SOL.",
            "raydium-plugin quickstart".to_string(),
            steps,
        )
    } else if has_usdc && has_gas {
        // Has both USDC and SOL — ready to swap USDC → SOL or other tokens
        let steps = serde_json::json!([
            {
                "step": 1,
                "description": "Get a swap quote (USDC → SOL, no gas):",
                "command": format!(
                    "raydium-plugin get-swap-quote --input-mint {} --output-mint {} --amount 1",
                    USDC_MINT, SOL_MINT
                )
            },
            {
                "step": 2,
                "description": "Execute swap (USDC → SOL):",
                "command": format!(
                    "raydium-plugin swap --input-mint {} --output-mint {} --amount 1 --confirm",
                    USDC_MINT, SOL_MINT
                )
            },
            {
                "step": 3,
                "description": "Get token price:",
                "command": format!("raydium-plugin get-token-price --mints {}", USDC_MINT)
            }
        ]);
        (
            "ready",
            "Your wallet has USDC and SOL. Get a quote or swap tokens on Raydium.",
            format!(
                "raydium-plugin get-swap-quote --input-mint {} --output-mint {} --amount 1",
                USDC_MINT, SOL_MINT
            ),
            steps,
        )
    } else if has_sol {
        // Has SOL only — ready to swap SOL → USDC or other tokens
        let steps = serde_json::json!([
            {
                "step": 1,
                "description": "Get a swap quote (SOL → USDC, no gas):",
                "command": format!(
                    "raydium-plugin get-swap-quote --input-mint {} --output-mint {} --amount 0.1",
                    SOL_MINT, USDC_MINT
                )
            },
            {
                "step": 2,
                "description": "Execute swap (SOL → USDC):",
                "command": format!(
                    "raydium-plugin swap --input-mint {} --output-mint {} --amount 0.1 --confirm",
                    SOL_MINT, USDC_MINT
                )
            },
            {
                "step": 3,
                "description": "Get token price:",
                "command": format!("raydium-plugin get-token-price --mints {}", SOL_MINT)
            }
        ]);
        (
            "ready_sol_only",
            "Your wallet has SOL. Swap SOL for USDC or other tokens on Raydium.",
            format!(
                "raydium-plugin get-swap-quote --input-mint {} --output-mint {} --amount 0.1",
                SOL_MINT, USDC_MINT
            ),
            steps,
        )
    } else {
        // No funds
        let steps = serde_json::json!([
            {
                "step": 1,
                "description": "Send SOL or USDC to your wallet on Solana mainnet:",
                "wallet": wallet,
                "note": "Minimum recommended: 0.1 SOL (covers fees + swap amount) or 1+ USDC with 0.01 SOL for gas"
            },
            {
                "step": 2,
                "description": "Run quickstart again:",
                "command": "raydium-plugin quickstart"
            }
        ]);
        (
            "no_funds",
            "Send SOL or USDC to your wallet before swapping.",
            "raydium-plugin quickstart".to_string(),
            steps,
        )
    };

    let output = serde_json::json!({
        "ok": true,
        "about": "Raydium is Solana's leading AMM — swap tokens at competitive rates with deep liquidity across hundreds of pairs.",
        "wallet": wallet,
        "chain": "Solana",
        "assets": {
            "sol_balance": sol_str,
            "usdc_balance": usdc_str
        },
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command,
        "onboarding_steps": onboarding_steps
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
