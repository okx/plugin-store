/// `kamino-liquidity quickstart` — wallet status, balances, and suggested first command.

use clap::Args;
use anyhow::Result;

use crate::{api, onchainos};

const ABOUT: &str = "Kamino KVaults are automated yield-optimization vaults on Solana — \
    deposit tokens (USDC, SOL, and more) to earn yield from Kamino lending markets \
    without manually managing positions.";

const MIN_SOL_GAS: f64 = 0.01;
const MIN_USDC: f64 = 1.0;

/// USDC mint on Solana mainnet
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

#[derive(Args, Debug)]
pub struct QuickstartArgs {
    /// Wallet address (base58). If omitted, resolved from onchainos.
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: QuickstartArgs) -> Result<()> {
    // Resolve wallet
    let wallet = match args.wallet {
        Some(w) => w,
        None => match onchainos::resolve_wallet_solana() {
            Ok(w) => w,
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": false,
                        "error": format!("{:#}", e),
                        "error_code": "WALLET_NOT_FOUND",
                        "suggestion": "Run `onchainos wallet balance --chain 501` to verify login, or pass --wallet <address>."
                    }))?
                );
                return Ok(());
            }
        },
    };

    // Fetch balances and positions concurrently
    let balances = onchainos::get_all_token_balances();
    let positions = api::get_user_positions(&wallet).await.unwrap_or_default();

    let sol_balance = onchainos::get_sol_balance();
    let usdc_balance = balances
        .iter()
        .find(|(_, _, mint)| mint == USDC_MINT)
        .map(|(_, bal, _)| *bal)
        .unwrap_or(0.0);

    let position_count = positions.as_array().map(|a| a.len()).unwrap_or(0);

    // Determine status
    let (status, suggestion, next_command) = if position_count > 0 {
        (
            "active",
            format!("You have {} active KVault position(s). Check your earnings or deposit more.", position_count),
            "kamino-liquidity positions".to_string(),
        )
    } else if sol_balance >= MIN_SOL_GAS && usdc_balance >= MIN_USDC {
        (
            "ready",
            "Wallet is funded. You can deposit USDC into a KVault to start earning yield.".to_string(),
            "kamino-liquidity vaults --token USDC".to_string(),
        )
    } else if usdc_balance >= MIN_USDC && sol_balance < MIN_SOL_GAS {
        (
            "needs_gas",
            format!("You have {:.2} USDC but need at least {} SOL for transaction fees.", usdc_balance, MIN_SOL_GAS),
            "onchainos wallet balance --chain 501".to_string(),
        )
    } else if sol_balance >= MIN_SOL_GAS {
        (
            "needs_funds",
            "You have SOL for gas but no USDC yet. Deposit USDC or another supported token to start earning.".to_string(),
            "kamino-liquidity vaults".to_string(),
        )
    } else {
        (
            "no_funds",
            "Wallet has no SOL or USDC. Fund your wallet first to use Kamino KVaults.".to_string(),
            "onchainos wallet balance --chain 501".to_string(),
        )
    };

    // Build asset summary
    let asset_list: Vec<serde_json::Value> = balances
        .iter()
        .take(8)
        .map(|(sym, bal, _)| serde_json::json!({ "symbol": sym, "balance": format!("{:.6}", bal) }))
        .collect();

    // Build onboarding steps when not active
    let mut output = serde_json::json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "assets": {
            "sol_balance": format!("{:.6}", sol_balance),
            "usdc_balance": format!("{:.6}", usdc_balance),
            "all_tokens": asset_list
        },
        "kvault_positions": position_count,
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command
    });

    if status != "active" {
        output["onboarding_steps"] = serde_json::json!([
            {
                "step": 1,
                "title": "Check available vaults",
                "command": "kamino-liquidity vaults --token USDC",
                "description": "List all USDC earn vaults with allocation strategies and fees."
            },
            {
                "step": 2,
                "title": "Preview a deposit",
                "command": "kamino-liquidity deposit --vault <VAULT_ADDRESS> --amount <AMOUNT> --dry-run",
                "description": "Dry-run a deposit to verify the transaction before broadcasting."
            },
            {
                "step": 3,
                "title": "Execute deposit",
                "command": "kamino-liquidity deposit --vault <VAULT_ADDRESS> --amount <AMOUNT> --confirm",
                "description": "Deposit tokens into the vault. Returns ok:true only after on-chain confirmation."
            },
            {
                "step": 4,
                "title": "Check positions",
                "command": "kamino-liquidity positions",
                "description": "View your share balances across all KVaults."
            },
            {
                "step": 5,
                "title": "Withdraw when ready",
                "command": "kamino-liquidity withdraw --vault <VAULT_ADDRESS> --amount <SHARES> --confirm",
                "description": "Redeem shares back to tokens. Amount is in shares, not token units."
            }
        ]);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
