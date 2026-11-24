use clap::Args;
use crate::config::{chain_config, resolve_token, token_symbol};
use crate::rpc::{format_amount, get_decimals, parse_human_amount, sushi_quote};

#[derive(Args)]
pub struct QuoteArgs {
    /// Input token (symbol or address)
    #[arg(long)]
    pub token_in: String,
    /// Output token (symbol or address)
    #[arg(long)]
    pub token_out: String,
    /// Amount of token_in to quote (human-readable, e.g. "0.1")
    #[arg(long)]
    pub amount_in: String,
    /// Slippage tolerance in percent (default: 0.5%)
    #[arg(long, default_value = "0.5")]
    pub slippage: f64,
}

pub async fn run(args: QuoteArgs, chain_id: u64) -> anyhow::Result<()> {
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

    // Use zero address for quotes (no wallet needed for read-only)
    let zero = "0x0000000000000000000000000000000000000000";
    let (amount_out_raw, _router, _data) =
        sushi_quote(chain_id, &token_in, &token_out, amount_in_raw, args.slippage, zero).await?;

    let slippage_factor = 1.0 - (args.slippage / 100.0);
    let amount_out_min = (amount_out_raw as f64 * slippage_factor) as u128;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "token_in":     sym_in,
        "token_out":    sym_out,
        "amount_in":    args.amount_in,
        "amount_out":   format_amount(amount_out_raw, dec_out),
        "amount_out_min": format_amount(amount_out_min, dec_out),
        "slippage":     format!("{}%", args.slippage),
        "chain":        cfg.name,
    }))?);
    Ok(())
}
