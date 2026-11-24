use clap::Args;
use serde_json::{json, Value};

use crate::api;
use crate::config::{is_native_token, parse_chain, supported_chains_help, ChainInfo, SUPPORTED_CHAINS};
use crate::onchainos::resolve_wallet;
use crate::rpc::{erc20_balance, fmt_token_amount, native_balance};

#[derive(Args)]
pub struct BalanceArgs {
    /// Wallet address (defaults to onchainos wallet on the first listed chain)
    #[arg(long)]
    pub address: Option<String>,

    /// Single chain (id or key). If omitted, query all 6 supported chains.
    #[arg(long)]
    pub chain: Option<String>,

    /// Specific token (symbol or 0x address). If omitted, only the native gas token is shown.
    #[arg(long)]
    pub token: Option<String>,
}

pub async fn run(args: BalanceArgs) -> anyhow::Result<()> {
    let chains: Vec<&'static ChainInfo> = if let Some(s) = &args.chain {
        match parse_chain(s) {
            Some(c) => vec![c],
            None => {
                println!("{}", super::error_response(
                    &format!("Unsupported chain '{}'", s),
                    "UNSUPPORTED_CHAIN",
                    &format!("Use one of: {}", supported_chains_help()),
                ));
                return Ok(());
            }
        }
    } else {
        SUPPORTED_CHAINS.iter().collect()
    };

    let mut entries: Vec<Value> = Vec::with_capacity(chains.len());
    for chain in chains {
        let address = match args.address.clone() {
            Some(a) => a,
            None => match resolve_wallet(chain.id) {
                Ok(a) => a,
                Err(e) => {
                    entries.push(json!({
                        "chain": chain.key,
                        "chain_id": chain.id,
                        "error": format!("wallet resolve failed: {:#}", e),
                        "error_code": "WALLET_NOT_FOUND",
                    }));
                    continue;
                }
            },
        };

        // Native balance is always reported.
        let native_raw = match native_balance(&address, chain.rpc).await {
            Ok(v) => v,
            Err(e) => {
                entries.push(json!({
                    "chain": chain.key,
                    "chain_id": chain.id,
                    "address": address,
                    "error": format!("native balance failed: {:#}", e),
                    "error_code": "RPC_ERROR",
                }));
                continue;
            }
        };

        let mut entry = json!({
            "chain": chain.key,
            "chain_id": chain.id,
            "address": address,
            "native": {
                "symbol": chain.native_symbol,
                "amount": fmt_token_amount(native_raw, 18),
                "amount_raw": native_raw.to_string(),
            }
        });

        // Optional ERC-20 lookup.
        if let Some(tok) = &args.token {
            // Resolve token (may be sentinel for native, or symbol/address)
            let (token_addr, decimals, sym) = match resolve_token(chain.id, tok, chain.native_symbol).await {
                Ok(t) => t,
                Err(e) => {
                    entry["token"] = json!({
                        "input": tok,
                        "error": format!("{:#}", e),
                        "error_code": "TOKEN_NOT_FOUND",
                    });
                    entries.push(entry);
                    continue;
                }
            };
            if is_native_token(&token_addr) {
                entry["token"] = json!({
                    "address": token_addr,
                    "symbol": sym,
                    "decimals": decimals,
                    "amount": fmt_token_amount(native_raw, 18),
                    "amount_raw": native_raw.to_string(),
                    "note": "Same as native gas balance.",
                });
            } else {
                let bal = match erc20_balance(&token_addr, &address, chain.rpc).await {
                    Ok(v) => v,
                    Err(e) => {
                        entry["token"] = json!({
                            "address": token_addr,
                            "symbol": sym,
                            "error": format!("{:#}", e),
                            "error_code": "RPC_ERROR",
                        });
                        entries.push(entry);
                        continue;
                    }
                };
                entry["token"] = json!({
                    "address": token_addr,
                    "symbol": sym,
                    "decimals": decimals,
                    "amount": fmt_token_amount(bal, decimals),
                    "amount_raw": bal.to_string(),
                });
            }
        }

        entries.push(entry);
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "count": entries.len(),
        "balances": entries,
    }))?);
    Ok(())
}

async fn resolve_token(
    chain_id: u64,
    user_input: &str,
    native_symbol: &str,
) -> anyhow::Result<(String, u32, String)> {
    let trimmed = user_input.trim();
    let upper = trimmed.to_uppercase();
    if is_native_token(trimmed)
        || upper == native_symbol
        || upper == "ETH" || upper == "BNB" || upper == "MATIC" || upper == "POL"
        || upper == "NATIVE"
    {
        use crate::config::NATIVE_TOKEN_SENTINEL;
        return Ok((NATIVE_TOKEN_SENTINEL.to_string(), 18, native_symbol.to_string()));
    }
    let info = api::get_token(chain_id, trimmed).await?;
    let address = info["address"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("LI.FI did not return an address for '{}'", trimmed))?
        .to_string();
    let decimals = info["decimals"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("LI.FI did not return decimals for '{}'", trimmed))?
        as u32;
    let symbol = info["symbol"].as_str().unwrap_or(trimmed).to_string();
    Ok((address, decimals, symbol))
}
