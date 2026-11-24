use clap::Args;
use serde_json::json;

use crate::config::SUPPORTED_CHAINS;
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{get_assets_in, native_balance, pad_address, pad_u256, selectors, wait_for_tx};

/// Mark cTokens as collateral via Comptroller.enterMarkets(address[]).
///
/// Rarely needed by hand: `borrow` auto-enters the borrow cToken's market when called
/// (and the user's supply cTokens are typically auto-entered by the Compound UI).
/// Use this if you want to make a supply-only cToken serve as collateral for a future borrow.
#[derive(Args)]
pub struct EnterMarketsArgs {
    /// Comma-separated cToken symbols (cDAI, cUSDC, …) or 0x cToken addresses.
    /// Singular `--ctoken` is accepted as an alias for symmetry with `exit-market`.
    #[arg(long, alias = "ctoken")]
    pub ctokens: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

pub async fn run(args: EnterMarketsArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    let mut ctokens: Vec<String> = Vec::new();
    for entry in args.ctokens.split(',') {
        let t = entry.trim();
        if t.is_empty() { continue; }
        if let Some(info) = crate::config::resolve_market(t) {
            ctokens.push(info.ctoken.to_lowercase());
        } else if t.starts_with("0x") && t.len() == 42 {
            ctokens.push(t.to_lowercase());
        } else {
            return print_err(&format!("Unknown cToken: {}", t),
                "TOKEN_NOT_FOUND", "Use one of cDAI/cUSDC/cUSDT/cETH/cWBTC2/cCOMP, or pass 0x cToken address.");
        }
    }
    if ctokens.is_empty() {
        return print_err("--ctokens parsed to empty list", "INVALID_ARGUMENT",
            "Pass at least one cToken, e.g. `--ctokens cDAI,cUSDC`.");
    }

    // Pre-flight: gas + show currently-entered set
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < 5_000_000_000_000_000 {
        return print_err("Native ETH below 0.005 floor", "INSUFFICIENT_GAS",
            "Top up at least 0.005 ETH on mainnet.");
    }
    let already_in = get_assets_in(chain.comptroller, &from_addr, chain.rpc).await.unwrap_or_default();

    // Build calldata: enterMarkets(address[])
    let mut calldata = String::new();
    calldata.push_str(selectors::ENTER_MARKETS);
    calldata.push_str(&pad_u256(32));                  // offset to dynamic array
    calldata.push_str(&pad_u256(ctokens.len() as u128)); // array length
    for c in &ctokens {
        calldata.push_str(&pad_address(c));
    }

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "enter_markets",
            "chain": chain.key,
            "holder": from_addr,
            "comptroller": chain.comptroller,
            "ctokens_to_enter": ctokens,
            "currently_entered": already_in,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    let gas_limit = 100_000u64 + (ctokens.len() as u64) * 30_000;
    let result = match wallet_contract_call(chain.id, chain.comptroller, &calldata, None, Some(gas_limit), false) {
        Ok(r) => r,
        Err(e) => return print_err(&format!("enterMarkets failed: {:#}", e),
            "ENTER_MARKETS_FAILED", "Inspect onchainos output."),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[enter-markets] Submit tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on Etherscan.");
            }
            eprintln!("[enter-markets] On-chain confirmed.");
        }
        None => return print_err("enterMarkets broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "enter_markets",
        "chain": chain.key,
        "ctokens_entered": ctokens,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `compound-v2-plugin positions` to verify entered_as_collateral=true. Now you can borrow against these.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
