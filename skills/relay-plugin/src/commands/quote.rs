use clap::Args;
use crate::api::{get_quote, resolve_token, token_symbol, QuoteRequest, NATIVE_ETH};
use crate::onchainos::resolve_wallet;

#[derive(Args)]
pub struct QuoteArgs {
    /// Source chain ID (e.g. 1 for Ethereum, 42161 for Arbitrum)
    #[arg(long)]
    pub from_chain: u64,
    /// Destination chain ID
    #[arg(long)]
    pub to_chain: u64,
    /// Token to send (symbol or address, e.g. ETH, USDC, 0x...)
    #[arg(long, default_value = "ETH")]
    pub token: String,
    /// Amount to send in human-readable form (e.g. 0.01 for 0.01 ETH)
    #[arg(long)]
    pub amount: String,
    /// Destination token (defaults to same as --token)
    #[arg(long)]
    pub to_token: Option<String>,
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let origin_token = resolve_token(&args.token, args.from_chain);
    let dest_token_input = args.to_token.as_deref().unwrap_or(&args.token);
    let dest_token = resolve_token(dest_token_input, args.to_chain);

    // Parse amount: need raw amount in smallest unit
    let amount_raw = parse_human_amount_eth(&args.amount, &origin_token, args.from_chain)?;

    // Quote only — wallet not required; fall back to zero address if none found
    let wallet = resolve_wallet(args.from_chain)
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());

    let quote = get_quote(QuoteRequest {
        user: wallet.clone(),
        recipient: wallet.clone(),
        origin_chain_id: args.from_chain,
        destination_chain_id: args.to_chain,
        origin_currency: origin_token.clone(),
        destination_currency: dest_token.clone(),
        amount: amount_raw.to_string(),
        trade_type: "EXACT_INPUT".to_string(),
    }).await?;

    let request_id = quote.steps.first()
        .and_then(|s| s.request_id.as_deref())
        .unwrap_or("unknown");

    let sym_in = if token_symbol(&origin_token, args.from_chain) != "UNKNOWN" {
        token_symbol(&origin_token, args.from_chain).to_string()
    } else { args.token.clone() };
    let sym_out = if token_symbol(&dest_token, args.to_chain) != "UNKNOWN" {
        token_symbol(&dest_token, args.to_chain).to_string()
    } else { dest_token_input.to_string() };

    let amount_out_fmt = quote.details.as_ref()
        .and_then(|d| d.currency_out.as_ref())
        .and_then(|c| c.amount_formatted.as_deref())
        .unwrap_or("unknown");
    let amount_out_usd = quote.details.as_ref()
        .and_then(|d| d.currency_out.as_ref())
        .and_then(|c| c.amount_usd.as_deref())
        .unwrap_or("unknown");
    let time_secs = quote.details.as_ref()
        .and_then(|d| d.time_estimate)
        .unwrap_or(0);

    let fee_usd = quote.fees.as_ref()
        .and_then(|f| f.pointer("/relayer/amountUsd"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let steps_summary: Vec<&str> = quote.steps.iter().map(|s| s.id.as_str()).collect();

    let out = serde_json::json!({
        "token_in":    sym_in,
        "token_out":   sym_out,
        "amount_in":   args.amount,
        "amount_out":  amount_out_fmt,
        "amount_out_usd": amount_out_usd,
        "fee_usd":     fee_usd,
        "from_chain":  args.from_chain,
        "to_chain":    args.to_chain,
        "estimated_time_secs": time_secs,
        "steps":       steps_summary,
        "request_id":  request_id,
    });

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Parse a human-readable amount into raw units (wei for ETH, 6 decimals for USDC/USDT, 18 for DAI).
pub fn parse_human_amount_eth(s: &str, token_addr: &str, chain_id: u64) -> anyhow::Result<u128> {
    let decimals: u8 = if token_addr == NATIVE_ETH {
        18
    } else {
        match token_symbol(token_addr, chain_id) {
            "USDC" | "USDT" => 6,
            "DAI" => 18,
            _ => 18,
        }
    };
    crate::commands::bridge::parse_amount(s, decimals)
}
