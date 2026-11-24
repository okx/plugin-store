/// `fourmeme-plugin send --to 0x... --amount X [--token 0x...]` — send BNB or ERC-20.
///
/// `--token` omitted (or `BNB` / zero address) → native BNB transfer via msg.value.
/// `--token <addr>` → ERC-20 `transfer(to, amount)` to the contract.

use anyhow::{Context, Result};
use clap::Args;

use crate::config::{chain_name, is_supported_chain, TOKEN_DECIMALS};
use crate::rpc::{eth_get_balance_wei, estimate_native_gas_cost_wei, wei_to_bnb};

const GAS_LIMIT_NATIVE_SEND: u64 = 21_000;
const GAS_LIMIT_ERC20_SEND:  u64 = 100_000;
const ZERO_ADDR: &str = "0x0000000000000000000000000000000000000000";

#[derive(Args)]
pub struct SendArgs {
    /// Recipient address
    #[arg(long)]
    pub to: String,

    /// Amount in whole units (e.g. "0.01" BNB; for ERC-20 it's whole tokens at the
    /// token's decimals — Four.meme tokens are 18-decimal)
    #[arg(long)]
    pub amount: String,

    /// Optional ERC-20 token contract. Omit (or pass "BNB"/"0x0") for native BNB.
    #[arg(long)]
    pub token: Option<String>,

    /// Token decimals override. Default 18 for both BNB and Four.meme tokens.
    #[arg(long, default_value_t = 18)]
    pub decimals: u32,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Pass --confirm to actually submit the on-chain tx. Default is preview-only
    /// (prints the planned tx without spending gas) so accidental invocation is safe.
    #[arg(long, default_value_t = false)]
    pub confirm: bool,
}

pub async fn run(args: SendArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("send"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: SendArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    if !args.to.starts_with("0x") || args.to.len() != 42 {
        anyhow::bail!("Invalid --to address {}", args.to);
    }
    let raw = super::parse_human_amount(&args.amount, args.decimals)?;
    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    let is_native = match args.token.as_deref() {
        None => true,
        Some(t) => {
            let lower = t.to_lowercase();
            lower == "bnb" || lower == ZERO_ADDR || lower == "0x0"
        }
    };

    let (target, calldata, value, gas_limit, label) = if is_native {
        (args.to.clone(), String::new(), Some(raw), GAS_LIMIT_NATIVE_SEND, "BNB".to_string())
    } else {
        let token_addr = args.token.as_ref().unwrap().to_lowercase();
        let cd = crate::calldata::format_erc20_transfer(&args.to, raw);
        let sym = super::erc20_symbol(args.chain, &token_addr).await;
        (token_addr, cd, None, GAS_LIMIT_ERC20_SEND, sym)
    };

    if !args.confirm {
        let resp = serde_json::json!({
            "ok": true,
            "preview_only": true,
            "data": {
                "action": "send",
                "chain": chain_name(args.chain),
                "from": wallet,
                "to": args.to,
                "asset": label,
                "amount": super::fmt_decimal(raw, args.decimals),
                "amount_raw": raw.to_string(),
                "is_native": is_native,
                "tx_plan": if is_native {
                    format!("send {} wei BNB to {}", raw, args.to)
                } else {
                    format!("call ERC-20.transfer({}, {}) at {}", args.to, raw, target)
                },
            }
        });
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    // Pre-flight: BNB needs to cover gas; if native send, must also cover the value
    let need_gas = estimate_native_gas_cost_wei(args.chain, gas_limit).await?;
    let need_total = if is_native { raw.saturating_add(need_gas) } else { need_gas };
    let have = eth_get_balance_wei(args.chain, &wallet).await?;
    if have < need_total {
        anyhow::bail!(
            "Insufficient BNB: have {:.6}, need ~{:.6}.",
            wei_to_bnb(have), wei_to_bnb(need_total),
        );
    }
    if !is_native {
        // Verify ERC-20 balance covers transfer. EVM-012: surface RPC failures
        // distinctly from "0 balance" — silent unwrap_or(0) used to misreport
        // INSUFFICIENT_BALANCE on every public-RPC blip even when the wallet
        // actually held enough.
        let bal = super::erc20_balance(args.chain, &target, &wallet).await
            .with_context(|| format!(
                "Failed to read {} balance on chain {}: public RPC may be limited; \
                 retry shortly. Cannot verify whether wallet covers transfer without \
                 an authoritative balance read.",
                label, args.chain
            ))?;
        if bal < raw {
            anyhow::bail!(
                "Insufficient {} balance: have {}, need {}.",
                label,
                super::fmt_decimal(bal, args.decimals),
                super::fmt_decimal(raw, args.decimals),
            );
        }
    }

    eprintln!("[fourmeme] sending {} {} to {}...",
        super::fmt_decimal(raw, args.decimals), label, args.to);

    let resp = if is_native {
        // Native send via onchainos: call the recipient with empty data + msg.value
        crate::onchainos::wallet_contract_call(
            args.chain, &target, "0x", Some(&wallet), value, false,
        ).await?
    } else {
        crate::onchainos::wallet_contract_call(
            args.chain, &target, &calldata, Some(&wallet), None, true,
        ).await?
    };
    let tx_hash = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[fourmeme] send tx: {} (waiting...)", tx_hash);
    crate::onchainos::wait_for_tx_receipt(&tx_hash, args.chain, 120).await?;

    let _ = TOKEN_DECIMALS; // silence dead-import in some configs
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": "send",
            "chain": chain_name(args.chain),
            "from": wallet,
            "to": args.to,
            "asset": label,
            "amount": super::fmt_decimal(raw, args.decimals),
            "amount_raw": raw.to_string(),
            "is_native": is_native,
            "send_tx": tx_hash,
            "on_chain_status": "0x1",
        }
    }))?);
    Ok(())
}
