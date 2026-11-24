/// `fourmeme-plugin agent-register --name X [--image-url URL] [--description X]`
///
/// Mints an ERC-8004 agent identity NFT on `0x8004A169FB4a3325136EB29fA0ceB6D2e539a432`.
/// Constructs `agentURI = data:application/json;base64,<payload>` and calls
/// `register(string)`. After mint, `aiCreator=true` flag will appear on tokens
/// this wallet creates via Four.meme.

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{eth_get_balance_wei, estimate_native_gas_cost_wei, wei_to_bnb};

const NFT_8004: &str = "0x8004A169FB4a3325136EB29fA0ceB6D2e539a432";
const REGISTRATION_TYPE: &str = "https://eips.ethereum.org/EIPS/eip-8004#registration-v1";
const GAS_LIMIT_REGISTER: u64 = 250_000;

#[derive(Args)]
pub struct AgentRegisterArgs {
    /// Display name (required)
    #[arg(long)]
    pub name: String,

    #[arg(long, default_value = "I'm a four.meme trading agent")]
    pub description: String,

    /// Optional image URL embedded in the agentURI metadata
    #[arg(long, default_value = "")]
    pub image_url: String,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Pass --confirm to actually submit the on-chain tx. Default is preview-only
    /// (prints the planned tx without spending gas) so accidental invocation is safe.
    #[arg(long, default_value_t = false)]
    pub confirm: bool,
}

pub async fn run(args: AgentRegisterArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("agent-register"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: AgentRegisterArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    if args.name.trim().is_empty() {
        anyhow::bail!("--name is required");
    }
    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    // Build agentURI: data:application/json;base64,<base64({name, description, image, ...})>
    let payload = serde_json::json!({
        "type": REGISTRATION_TYPE,
        "name": args.name.trim(),
        "description": args.description,
        "image": args.image_url,
        "active": true,
        "supportedTrust": [""],
    });
    let json = serde_json::to_string(&payload)?;
    let b64 = B64.encode(json.as_bytes());
    let agent_uri = format!("data:application/json;base64,{}", b64);
    let calldata = crate::calldata::build_8004_register(&agent_uri);

    if !args.confirm {
        let resp = serde_json::json!({
            "ok": true,
            "preview_only": true,
            "data": {
                "action": "agent-register",
                "chain": chain_name(args.chain),
                "wallet": wallet,
                "contract": NFT_8004,
                "agent_uri_length_bytes": agent_uri.len(),
                "agent_uri_preview": format!("{}...", &agent_uri[..agent_uri.len().min(80)]),
                "name": args.name,
                "description": args.description,
                "image_url": args.image_url,
                "tx_plan": format!("ERC8004NFT.register(\"data:application/json;base64,...{}b\") at {}",
                                   agent_uri.len(), NFT_8004),
                "note": "preview only (--confirm omitted): no transactions submitted.",
            }
        });
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    let need_gas = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_REGISTER).await?;
    let have = eth_get_balance_wei(args.chain, &wallet).await?;
    if have < need_gas {
        anyhow::bail!("Insufficient BNB for gas: have {:.6}, need ~{:.6}.",
            wei_to_bnb(have), wei_to_bnb(need_gas));
    }

    eprintln!("[fourmeme] minting ERC-8004 agent NFT for wallet {}...", wallet);
    let resp = crate::onchainos::wallet_contract_call(
        args.chain, NFT_8004, &calldata,
        Some(&wallet), None, false,
    ).await?;
    let tx_hash = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[fourmeme] register tx: {} (waiting...)", tx_hash);
    crate::onchainos::wait_for_tx_receipt(&tx_hash, args.chain, 120).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": "agent-register",
            "chain": chain_name(args.chain),
            "wallet": wallet,
            "contract": NFT_8004,
            "name": args.name,
            "register_tx": tx_hash,
            "on_chain_status": "0x1",
            "tip": "Verify with `agent-balance` (should now show 1+). Future Four.meme creates from this wallet will have aiCreator=true.",
        }
    }))?);
    Ok(())
}
