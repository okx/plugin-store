use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct BridgeArgs {
    /// Source chain ID (e.g. 8453 for Base)
    #[arg(long)]
    pub from_chain: u64,

    /// Destination chain ID (e.g. 1 for Ethereum)
    #[arg(long)]
    pub to_chain: u64,

    /// Token symbol or address (e.g. ETH or 0x0000...)
    #[arg(long, default_value = "ETH")]
    pub token: String,

    /// Amount to bridge (in token units, e.g. 0.001)
    #[arg(long)]
    pub amount: f64,

    /// Recipient address on destination chain (defaults to wallet address)
    #[arg(long)]
    pub recipient: Option<String>,

    /// Wallet address to send from (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: BridgeArgs) -> anyhow::Result<()> {
    // Resolve wallet address
    let wallet = if let Some(f) = args.from.clone() {
        f
    } else {
        onchainos::resolve_wallet(args.from_chain, args.dry_run)?
    };

    let origin_currency = resolve_currency(&args.token);
    let destination_currency = resolve_currency(&args.token);

    // Convert amount to wei (18 decimals for ETH/native)
    let amount_wei = (args.amount * 1e18) as u128;
    let amount_str = amount_wei.to_string();

    let recipient = args.recipient.as_deref().unwrap_or(&wallet);

    println!("=== Relay Bridge ===");
    println!("From:      {} (Chain {})", wallet, args.from_chain);
    println!("To:        {} (Chain {})", recipient, args.to_chain);
    println!("Token:     {}", args.token);
    println!("Amount:    {} ({} wei)", args.amount, amount_wei);
    println!();

    // Get quote first
    println!("Fetching quote...");
    let quote = rpc::get_quote(
        &wallet,
        args.from_chain,
        args.to_chain,
        &origin_currency,
        &destination_currency,
        &amount_str,
        Some(recipient),
    )?;

    if let Some(err) = quote.get("message") {
        anyhow::bail!("API error getting quote: {}", err);
    }

    // Extract the deposit step
    let steps = quote["steps"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No steps in quote response"))?;

    if steps.is_empty() {
        anyhow::bail!("Quote returned empty steps array");
    }

    let step = &steps[0];
    let step_id = step["id"].as_str().unwrap_or("deposit");
    let request_id = step["requestId"].as_str().unwrap_or("");

    let items = step["items"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No items in step"))?;

    if items.is_empty() {
        anyhow::bail!("Step has no items");
    }

    let tx_data = &items[0]["data"];
    let to_addr = tx_data["to"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'to' address in step data"))?;
    let calldata = tx_data["data"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'data' in step data"))?;
    let value_str = tx_data["value"].as_str().unwrap_or("0");
    let value_wei: u128 = value_str.parse().unwrap_or(0);

    // Display quote summary
    if let Some(fees) = quote.get("fees") {
        if let Some(relayer) = fees.get("relayer") {
            println!("Relayer fee:   {} ETH (~${} USD)",
                relayer["amountFormatted"].as_str().unwrap_or("?"),
                relayer["amountUsd"].as_str().unwrap_or("?"));
        }
    }
    if let Some(details) = quote.get("details") {
        let time_est = details["timeEstimate"].as_u64().unwrap_or(0);
        println!("Time estimate: ~{} seconds", time_est);
        if let Some(currency_out) = details.get("currencyOut") {
            println!("You receive:   {} {} (Chain {})",
                currency_out["amountFormatted"].as_str().unwrap_or("?"),
                currency_out["currency"]["symbol"].as_str().unwrap_or("?"),
                args.to_chain);
        }
    }
    println!();
    println!("Step:      {}", step_id);
    println!("To:        {}", to_addr);
    println!("Calldata:  {}", calldata);
    println!("Value:     {} wei", value_wei);
    println!("RequestId: {}", request_id);
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted.");
        println!("To execute: relay bridge --from-chain {} --to-chain {} --token {} --amount {}",
            args.from_chain, args.to_chain, args.token, args.amount);
        return Ok(());
    }

    // IMPORTANT: Ask user to confirm before submitting bridge transaction
    println!("WARNING: This will submit a bridge transaction. Please confirm the details above.");
    println!("Submitting bridge transaction...");

    let result = onchainos::wallet_contract_call(
        args.from_chain,
        to_addr,
        calldata,
        Some(value_wei),
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Transaction submitted: {}", tx_hash);
    println!();
    println!("Bridge in progress. Request ID: {}", request_id);
    println!("Monitor status: relay status --request-id {}", request_id);
    println!("Expected completion: check status endpoint for updates");

    Ok(())
}

fn resolve_currency(token: &str) -> String {
    match token.to_uppercase().as_str() {
        "ETH" => config::ETH_ADDRESS.to_string(),
        _ => {
            if token.starts_with("0x") {
                token.to_string()
            } else {
                config::ETH_ADDRESS.to_string()
            }
        }
    }
}
