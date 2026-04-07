use crate::{config, onchainos};
use clap::Args;
use serde_json::json;

#[derive(Args, Debug)]
pub struct ClaimRewardsArgs {
    /// Vault contract address to claim rewards from
    #[arg(long)]
    pub vault: String,

    /// Wallet address (optional, defaults to onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(
    args: &ClaimRewardsArgs,
    dry_run: bool,
    chain_id: u64,
) -> anyhow::Result<()> {
    if dry_run {
        let calldata = build_claim_rewards_calldata(
            "0x0000000000000000000000000000000000000000",
            &args.vault,
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "dry_run": true,
                "action": "claim-rewards",
                "vault": args.vault,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "calldata": calldata
            }))?
        );
        return Ok(());
    }

    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let calldata = build_claim_rewards_calldata(&wallet, &args.vault);
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::MORPHO_LENDING_ROUTER,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "action": "claim-rewards",
            "vault": args.vault,
            "wallet": wallet,
            "tx_hash": tx_hash,
            "etherscan": format!("https://etherscan.io/tx/{}", tx_hash)
        }))?
    );
    Ok(())
}

/// Build claimRewards calldata.
/// claimRewards(address onBehalf, address vault)
fn build_claim_rewards_calldata(on_behalf: &str, vault: &str) -> String {
    use alloy_sol_types::{sol, SolCall};

    sol! {
        function claimRewards(
            address onBehalf,
            address vault
        ) external;
    }

    let on_behalf_addr: alloy_primitives::Address = on_behalf.parse().unwrap_or_default();
    let vault_addr: alloy_primitives::Address = vault.parse().unwrap_or_default();

    let call = claimRewardsCall {
        onBehalf: on_behalf_addr,
        vault: vault_addr,
    };

    format!("0x{}", hex::encode(call.abi_encode()))
}
