use clap::Args;
use serde_json::json;

use crate::config::{ETH_KNOWN_MARKETS, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    erc20_balance, fmt_token_amount, get_comp_accrued, native_balance, pad_address, pad_u256,
    selectors, wait_for_tx,
};

/// Claim accrued COMP rewards via Comptroller.claimComp(address holder, address[] cTokens).
///
/// Pre-flight: queries `compAccrued(holder)` to surface the stored value. Note this is
/// often less than the actual claimable — Comptroller distributes COMP at claim time
/// (state side-effect), so the on-chain `compAccrued` may understate by the un-distributed
/// portion. Calling claimComp triggers full settlement.
#[derive(Args)]
pub struct ClaimCompArgs {
    /// cTokens to claim from. Default: all 6 well-known. Pass comma-separated symbols
    /// or 0x cToken addresses, e.g. `--ctokens cDAI,cUSDC` or `--ctokens 0x5d3a…`.
    #[arg(long)]
    pub ctokens: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

pub async fn run(args: ClaimCompArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Resolve cToken list
    let ctokens: Vec<&'static str> = match args.ctokens.as_deref() {
        None => ETH_KNOWN_MARKETS.iter().map(|m| m.ctoken).collect(),
        Some(list) => {
            let mut out = Vec::new();
            for entry in list.split(',') {
                let t = entry.trim();
                if t.is_empty() { continue; }
                if let Some(info) = crate::config::resolve_market(t) {
                    out.push(info.ctoken);
                } else if t.starts_with("0x") && t.len() == 42 {
                    // pass through raw 0x address (still v0.1.0; not validated against Comptroller list)
                    // Use a leak so it stays 'static (only ~6 max in practice)
                    out.push(Box::leak(t.to_lowercase().into_boxed_str()));
                } else {
                    return print_err(&format!("Unknown cToken in --ctokens: {}", t),
                        "TOKEN_NOT_FOUND", "Use one of cDAI/cUSDC/cUSDT/cETH/cWBTC/cCOMP, or pass 0x cToken address.");
                }
            }
            if out.is_empty() {
                return print_err("--ctokens parsed to empty list", "INVALID_ARGUMENT",
                    "Pass at least one cToken or omit the flag for all 6.");
            }
            out
        }
    };

    // Pre-flight: gas + accrued COMP
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < 5_000_000_000_000_000 {
        return print_err("Native ETH below 0.005 floor", "INSUFFICIENT_GAS",
            "Top up at least 0.005 ETH on mainnet.");
    }
    // EVM-012: surface RPC failures distinctly so claim doesn't fire on a
    // misleading "0 accrued" snapshot.
    let accrued = match get_comp_accrued(chain.comptroller, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read compAccrued from Comptroller on {}: {:#}", chain.key, e),
            "RPC_ERROR", "Public RPC may be limited; retry shortly.",
        ),
    };
    // The before-balance snapshot is non-critical (only used for the after-vs-before
    // delta display). Keep the soft fallback but expose the error.
    let (comp_bal_before, comp_balance_before_query_error) =
        match erc20_balance(chain.comp_token, &from_addr, chain.rpc).await {
            Ok(v) => (v, None),
            Err(e) => (0u128, Some(format!("{:#}", e))),
        };

    // Build calldata: claimComp(address holder, address[] cTokens)
    // ABI:
    //   selector + holder(32) + offset_to_array(32, value=0x40) + array_length(32) + addr × N (each 32)
    let mut calldata = String::with_capacity(8 + 4*64 + ctokens.len()*64 + 2);
    calldata.push_str(selectors::CLAIM_COMP_HOLDER_LIST);
    calldata.push_str(&pad_address(&from_addr));
    calldata.push_str(&pad_u256(0x40));        // offset to address[] = 64 bytes (after the two preceding words)
    calldata.push_str(&pad_u256(ctokens.len() as u128));
    for c in &ctokens {
        calldata.push_str(&pad_address(c));
    }

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "claim_comp",
            "chain": chain.key,
            "holder": from_addr,
            "comptroller": chain.comptroller,
            "comp_token": chain.comp_token,
            "ctokens_count": ctokens.len(),
            "ctokens": ctokens,
            "comp_accrued_stored":     fmt_token_amount(accrued, 18),
            "comp_accrued_stored_raw": accrued.to_string(),
            "comp_balance_before":     fmt_token_amount(comp_bal_before, 18),
            "note": "compAccrued is the stored value; actual claim may be slightly higher after Comptroller settles supplier/borrower distributions in this tx.",
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    // Gas: claimComp iterates cTokens × 2 (supply + borrow distribution per market). Heavy on L1.
    let gas_limit = 200_000u64 + (ctokens.len() as u64) * 70_000;
    eprintln!("[claim-comp] claimComp({} cTokens, holder={})…", ctokens.len(), &from_addr[..10]);
    let result = match wallet_contract_call(chain.id, chain.comptroller, &calldata, None, Some(gas_limit), false) {
        Ok(r) => r,
        Err(e) => return print_err(&format!("claimComp failed: {:#}", e),
            "CLAIM_FAILED", "Common: gas, RPC. Compound emits Failure event with code on revert."),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[claim-comp] Submit tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on Etherscan.");
            }
            eprintln!("[claim-comp] On-chain confirmed.");
        }
        None => return print_err("claimComp broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    // Post-claim snapshot — keep soft fallback (tx already confirmed) but
    // expose query error so the rendered claimed-amount can be marked best-effort.
    let (comp_bal_after, comp_balance_after_query_error) =
        match erc20_balance(chain.comp_token, &from_addr, chain.rpc).await {
            Ok(v) => (v, None),
            Err(e) => (comp_bal_before, Some(format!("{:#}", e))),
        };
    let claimed = comp_bal_after.saturating_sub(comp_bal_before);

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "claim_comp",
        "chain": chain.key,
        "holder": from_addr,
        "ctokens_claimed_from": ctokens,
        "comp_balance_before":      fmt_token_amount(comp_bal_before, 18),
        "comp_balance_after":       fmt_token_amount(comp_bal_after, 18),
        "comp_claimed":             fmt_token_amount(claimed, 18),
        "comp_claimed_raw":         claimed.to_string(),
        "comp_balance_before_query_error": comp_balance_before_query_error,
        "comp_balance_after_query_error":  comp_balance_after_query_error,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Claimed COMP is now in your wallet as ERC-20. Hold or transfer freely.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
