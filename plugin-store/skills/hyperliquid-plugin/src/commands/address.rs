use clap::Args;
use crate::config::{CHAIN_ID, ARBITRUM_CHAIN_ID, HYPER_EVM_RPC};
use crate::onchainos::resolve_wallet;
use crate::rpc::{ARBITRUM_RPC, erc20_balance};
use crate::config::{USDC_ARBITRUM, USDC_HYPER_EVM};

#[derive(Args)]
pub struct AddressArgs {
    /// Also show HyperEVM address (hidden by default — USDC contract TBD)
    #[arg(long)]
    pub hyp_evm: bool,

    /// Show Arbitrum address (default behaviour, kept for explicitness)
    #[arg(long)]
    pub arbitrum: bool,

    /// Show all addresses (Arbitrum + HyperEVM)
    #[arg(long)]
    pub all: bool,
}

pub async fn run(args: AddressArgs) -> anyhow::Result<()> {
    // HyperEVM is opt-in: USDC contract is still a placeholder (0x0000…).
    let show_hyp = args.hyp_evm || args.all;
    let show_arb = !args.hyp_evm || args.all || args.arbitrum;

    if show_hyp {
        let wallet = resolve_wallet(CHAIN_ID)?;

        let usdc_bal = erc20_balance(USDC_HYPER_EVM, &wallet, HYPER_EVM_RPC).await
            .map(|b| format!("  USDC balance : {:.4}", b as f64 / 1_000_000.0))
            .unwrap_or_default();

        println!("HyperEVM address");
        println!("  {}", wallet);
        if !usdc_bal.is_empty() {
            println!("{}", usdc_bal);
        }
        println!();
        println!("  To use 'transfer --via-evm', send at least 0.001 HYPE to this address");
        println!("  for gas. HYPE is the native gas token on HyperEVM (chain 999).");

        if show_arb {
            println!();
        }
    }

    if show_arb {
        let wallet = resolve_wallet(ARBITRUM_CHAIN_ID)?;

        let usdc_bal = erc20_balance(USDC_ARBITRUM, &wallet, ARBITRUM_RPC).await
            .map(|b| format!("  USDC balance : {:.4}", b as f64 / 1_000_000.0))
            .unwrap_or_default();

        println!("Arbitrum One address");
        println!("  {}", wallet);
        if !usdc_bal.is_empty() {
            println!("{}", usdc_bal);
        }
        println!();
        println!("  EOA wallets only. Send ETH for gas, then use 'deposit' to bridge USDC.");
    }

    Ok(())
}
