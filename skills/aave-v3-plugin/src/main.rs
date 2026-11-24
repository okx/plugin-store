mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "aave-v3",
    about = "Aave V3 lending and borrowing via OnchaionOS",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Chain ID (default: 8453 Base)
    #[arg(long, global = true, default_value = "8453")]
    chain: u64,
    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,
    /// Execute the transaction on-chain. Without this flag the operation is simulated only.
    #[arg(long, global = true, default_value = "false")]
    confirm: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Supply/deposit an asset to earn interest (aTokens)
    Supply {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// Human-readable amount (e.g. 1000.0)
        #[arg(long)]
        amount: f64,
    },
    /// Withdraw a previously supplied asset
    Withdraw {
        /// Asset ERC-20 address or symbol
        #[arg(long)]
        asset: String,
        /// Human-readable amount to withdraw (omit if using --all)
        #[arg(long)]
        amount: Option<f64>,
        /// Withdraw the full balance
        #[arg(long, default_value = "false")]
        all: bool,
    },
    /// Borrow an asset against posted collateral
    Borrow {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// Human-readable amount (e.g. 0.5 for 0.5 WETH)
        #[arg(long)]
        amount: f64,
    },
    /// Repay borrowed debt (partial or full)
    Repay {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// Human-readable amount to repay (omit if using --all)
        #[arg(long)]
        amount: Option<f64>,
        /// Repay the full outstanding balance
        #[arg(long, default_value = "false")]
        all: bool,
    },
    /// View current supply and borrow positions
    Positions {},
    /// Check health factor and liquidation risk
    HealthFactor {},
    /// List market rates, APYs, and liquidity for all assets
    Reserves {
        /// Filter by asset address or symbol (optional)
        #[arg(long)]
        asset: Option<String>,
    },
    /// Enable or disable an asset as collateral
    SetCollateral {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// true to enable as collateral, false to disable
        #[arg(long)]
        enable: bool,
    },
    /// Set efficiency mode (E-Mode) category
    SetEmode {
        /// E-Mode category ID: 0=none, 1=stablecoins, 2=ETH-correlated
        #[arg(long)]
        category: u8,
    },
    /// Claim accrued AAVE/GHO/token rewards
    ClaimRewards {},
    /// Check wallet assets and get a personalised next step for Aave V3
    Quickstart {},
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::Supply { asset, amount } => {
            commands::supply::run(cli.chain, &asset, amount, cli.from.as_deref(), !cli.confirm)
                .await
        }
        Commands::Withdraw { asset, amount, all } => {
            commands::withdraw::run(
                cli.chain,
                &asset,
                amount,
                all,
                cli.from.as_deref(),
                !cli.confirm,
            )
            .await
        }
        Commands::Borrow { asset, amount } => {
            commands::borrow::run(cli.chain, &asset, amount, cli.from.as_deref(), !cli.confirm)
                .await
        }
        Commands::Repay { asset, amount, all } => {
            commands::repay::run(
                cli.chain,
                &asset,
                amount,
                all,
                cli.from.as_deref(),
                !cli.confirm,
            )
            .await
        }
        Commands::Positions {} => {
            commands::positions::run(cli.chain, cli.from.as_deref()).await
        }
        Commands::HealthFactor {} => {
            commands::health_factor::run(cli.chain, cli.from.as_deref()).await
        }
        Commands::Reserves { asset } => {
            commands::reserves::run(cli.chain, asset.as_deref()).await
        }
        Commands::SetCollateral { asset, enable } => {
            commands::set_collateral::run(
                cli.chain,
                &asset,
                enable,
                cli.from.as_deref(),
                !cli.confirm,
            )
            .await
        }
        Commands::SetEmode { category } => {
            commands::set_emode::run(cli.chain, category, cli.from.as_deref(), !cli.confirm).await
        }
        Commands::ClaimRewards {} => {
            commands::claim_rewards::run(cli.chain, cli.from.as_deref(), !cli.confirm).await
        }
        Commands::Quickstart {} => {
            commands::quickstart::run(cli.chain, cli.from.as_deref()).await
        }
    };

    match result {
        Ok(val) => {
            println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        }
        Err(err) => {
            // Failures belong on stdout with ok:false, the JSON contract SKILL.md
            // documents and the other plugins already follow. They used to go to
            // stderr with exit 1 and an empty stdout, leaving an agent that parses
            // stdout with nothing to report but "the command failed".
            println!(
                "{}",
                serde_json::to_string_pretty(&error_json(&err)).unwrap_or_default()
            );
        }
    }
}

/// The failure payload shared by every command.
///
/// `err.to_string()` renders only the outermost context and silently drops the
/// cause chain — which is exactly where the reason lives: onchainos's stderr and
/// stdout, an unusable search payload, a JSON parse failure. `{:#}` flattens the
/// whole chain onto one line for humans; `causes` keeps each layer addressable
/// for a caller that wants to match on the innermost one.
fn error_json(err: &anyhow::Error) -> Value {
    serde_json::json!({
        "ok": false,
        "error": format!("{:#}", err),
        "causes": err.chain().map(|c| c.to_string()).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::error_json;
    use anyhow::Context;

    #[test]
    fn error_json_reports_ok_false_and_flattens_the_cause_chain() {
        let err = Err::<(), _>(anyhow::anyhow!("onchainos data has 0 entries"))
            .context("No token match for 'NOSUCHTOKEN' on chain 42161")
            .context("Could not resolve token address for 'NOSUCHTOKEN'")
            .unwrap_err();

        let v = error_json(&err);

        assert_eq!(v["ok"], false);
        // The outermost context alone is not enough to act on; the flattened form
        // has to carry the innermost reason too.
        let msg = v["error"].as_str().expect("error is a string");
        assert!(msg.contains("Could not resolve token address"), "got: {msg}");
        assert!(msg.contains("onchainos data has 0 entries"), "got: {msg}");

        let causes: Vec<&str> = v["causes"]
            .as_array()
            .expect("causes is an array")
            .iter()
            .map(|c| c.as_str().unwrap_or_default())
            .collect();
        assert_eq!(causes.len(), 3, "every layer is listed: {causes:?}");
        assert_eq!(causes[0], "Could not resolve token address for 'NOSUCHTOKEN'");
        assert_eq!(causes[2], "onchainos data has 0 entries");
    }

    #[test]
    fn error_json_handles_a_single_layer_error() {
        let err = anyhow::anyhow!("No --from address and could not resolve active wallet.");
        let v = error_json(&err);

        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "No --from address and could not resolve active wallet.");
        assert_eq!(v["causes"].as_array().expect("array").len(), 1);
    }
}
