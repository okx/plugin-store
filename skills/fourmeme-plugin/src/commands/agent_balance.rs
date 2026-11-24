/// `fourmeme-plugin agent-balance [--owner 0x...]` — count of ERC-8004 agent
/// identity NFTs owned by a wallet. Without `--owner`, queries the active
/// onchainos wallet.

use anyhow::Result;
use clap::Args;

use crate::config::is_supported_chain;
use crate::rpc::{build_address_call, eth_call, parse_uint256_to_u128};

const NFT_8004: &str = "0x8004A169FB4a3325136EB29fA0ceB6D2e539a432";

#[derive(Args)]
pub struct AgentBalanceArgs {
    /// Wallet to query (default: active onchainos wallet on chain)
    #[arg(long)]
    pub owner: Option<String>,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,
}

pub async fn run(args: AgentBalanceArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("agent-balance"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: AgentBalanceArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let owner = match args.owner {
        Some(o) => o.to_lowercase(),
        None => crate::onchainos::get_wallet_address(args.chain).await?,
    };
    let data = build_address_call(crate::calldata::SEL_BALANCE_OF, &owner);
    let hex = eth_call(args.chain, NFT_8004, &data).await?;
    let bal = parse_uint256_to_u128(&hex);

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "owner": owner,
            "agent_nft_balance": bal.to_string(),
            "is_agent": bal > 0,
            "contract": NFT_8004,
            "tip": if bal == 0 {
                "Wallet has no ERC-8004 agent NFT. Run `agent-register --name \"<name>\"` to mint one."
            } else {
                "Wallet is registered as an Agent — token creates from this wallet are flagged with aiCreator=true."
            },
        }
    }))?);
    Ok(())
}
