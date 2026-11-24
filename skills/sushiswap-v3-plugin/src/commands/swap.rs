use clap::Args;
use tokio::time::{sleep, Duration};
use crate::config::{build_approve_calldata, chain_config, resolve_token, token_symbol};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_amount, get_allowance, get_decimals, parse_human_amount, sushi_quote};

#[derive(Args)]
pub struct SwapArgs {
    /// Input token (symbol or address)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or address)
    #[arg(long)]
    pub token_out: String,
    /// Amount of token_in to swap (human-readable)
    #[arg(long)]
    pub amount_in: String,
    /// Slippage tolerance in percent (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
    /// Broadcast the swap. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos (dry-run)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: SwapArgs, chain_id: u64) -> anyhow::Result<()> {
    let cfg = chain_config(chain_id)?;
    let rpc_owned = crate::config::rpc_url(chain_id)?;
    let rpc: &str = &rpc_owned;
    let token_in = resolve_token(&args.token_in, chain_id);
    let token_out = resolve_token(&args.token_out, chain_id);
    let sym_in = if token_symbol(&token_in, chain_id) != "UNKNOWN" {
        token_symbol(&token_in, chain_id).to_string()
    } else { args.token_in.clone() };
    let sym_out = if token_symbol(&token_out, chain_id) != "UNKNOWN" {
        token_symbol(&token_out, chain_id).to_string()
    } else { args.token_out.clone() };

    let dec_in = get_decimals(&token_in, rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, rpc).await.unwrap_or(18);
    let amount_in_raw = parse_human_amount(&args.amount_in, dec_in)?;

    if amount_in_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(chain_id)?
    };

    // Get quote + calldata from Sushi API
    let (amount_out_raw, router_to, calldata) =
        sushi_quote(chain_id, &token_in, &token_out, amount_in_raw, args.slippage, &wallet).await?;

    let slippage_factor = 1.0 - (args.slippage / 100.0);
    let amount_out_min = (amount_out_raw as f64 * slippage_factor) as u128;

    let preview = serde_json::json!({
        "preview": true,
        "action": "swap",
        "token_in":      sym_in,
        "token_out":     sym_out,
        "amount_in":     args.amount_in,
        "expected_out":  format_amount(amount_out_raw, dec_out),
        "minimum_out":   format_amount(amount_out_min, dec_out),
        "slippage":      format!("{}%", args.slippage),
        "router":        router_to,
        "wallet":        wallet,
        "chain":         cfg.name,
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast this swap.");
        return Ok(());
    }

    // Approve if needed
    if !args.dry_run {
        let allowance = get_allowance(&token_in, &wallet, &router_to, rpc).await?;
        if allowance < amount_in_raw {
            eprintln!("[sushiswap-v3] Approving {} for router...", sym_in);
            let approve_data = build_approve_calldata(&router_to, amount_in_raw);
            let approve_result = wallet_contract_call(chain_id, &token_in, &approve_data, false, false, Some(&wallet)).await?;
            let approve_hash = extract_tx_hash(&approve_result);
            eprintln!("[sushiswap-v3] Approve tx: {}", approve_hash);
            sleep(Duration::from_secs(5)).await;
        }
    }

    let result = wallet_contract_call(chain_id, &router_to, &calldata, false, args.dry_run, Some(&wallet)).await?;
    let tx_hash = extract_tx_hash(&result);

    let mut out = serde_json::json!({
        "ok": true,
        "action": "swap",
        "token_in":    sym_in,
        "token_out":   sym_out,
        "amount_in":   args.amount_in,
        "expected_out": format_amount(amount_out_raw, dec_out),
        "minimum_out": format_amount(amount_out_min, dec_out),
        "tx_hash":     tx_hash,
        "explorer":    format!("{}/{}", cfg.explorer, tx_hash),
        "chain":       cfg.name,
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
