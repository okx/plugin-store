use clap::Args;
use crate::config::{nfpm, rpc_url};
use crate::onchainos::resolve_wallet;
use crate::rpc::{get_symbol, nfpm_balance_of, nfpm_positions, nfpm_token_of_owner_by_index};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (defaults to logged-in wallet)
    #[arg(long)]
    pub owner: Option<String>,
    /// Chain ID (default: 42161 Arbitrum)
    #[arg(long, default_value = "42161")]
    pub chain: u64,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url(args.chain)?;
    let nfpm_addr = nfpm(args.chain)?;

    let owner = match args.owner {
        Some(addr) => addr,
        None => resolve_wallet(args.chain)?,
    };

    let balance = nfpm_balance_of(nfpm_addr, &owner, &rpc).await?;

    if balance == 0 {
        let result = serde_json::json!({
            "ok": true,
            "data": {
                "owner": owner,
                "positions": [],
                "total": 0,
                "chain_id": args.chain
            }
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let mut positions = Vec::new();
    let max_positions = balance.min(20); // cap at 20 to avoid too many RPC calls

    for i in 0..max_positions {
        let token_id = nfpm_token_of_owner_by_index(nfpm_addr, &owner, i, &rpc).await?;
        match nfpm_positions(nfpm_addr, token_id, &rpc).await {
            Ok(mut pos) => {
                // Try to get token symbols
                let token0 = pos["token0"].as_str().unwrap_or("").to_string();
                let token1 = pos["token1"].as_str().unwrap_or("").to_string();
                let sym0 = get_symbol(&token0, &rpc).await.unwrap_or_else(|_| "?".to_string());
                let sym1 = get_symbol(&token1, &rpc).await.unwrap_or_else(|_| "?".to_string());
                pos["token0_symbol"] = serde_json::json!(sym0);
                pos["token1_symbol"] = serde_json::json!(sym1);
                positions.push(pos);
            }
            Err(e) => {
                eprintln!("Warning: failed to fetch position {}: {}", token_id, e);
            }
        }
    }

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "owner": owner,
            "positions": positions,
            "total": balance,
            "shown": positions.len(),
            "chain_id": args.chain
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
