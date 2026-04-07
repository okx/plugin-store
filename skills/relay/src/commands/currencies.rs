use crate::rpc;
use clap::Args;

#[derive(Args)]
pub struct CurrenciesArgs {
    /// Chain ID to list currencies for
    #[arg(long)]
    pub chain: u64,

    /// Maximum number of tokens to display (default: 20)
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

pub fn run(args: CurrenciesArgs) -> anyhow::Result<()> {
    let data = rpc::get_currencies(args.chain, args.limit)?;

    let groups = data
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Unexpected API response format for currencies"))?;

    println!("=== Relay Supported Currencies (Chain {}) ===", args.chain);
    let mut count = 0;
    for group in groups {
        if let Some(tokens) = group.as_array() {
            for token in tokens {
                let symbol = token["symbol"].as_str().unwrap_or("?");
                let name = token["name"].as_str().unwrap_or("?");
                let address = token["address"].as_str().unwrap_or("?");
                let decimals = token["decimals"].as_u64().unwrap_or(18);
                let verified = token["metadata"]["verified"].as_bool().unwrap_or(false);
                let verified_str = if verified { "verified" } else { "" };
                println!("  {:<8}  {:<30}  {}  ({}d)  {}", symbol, name, address, decimals, verified_str);
                count += 1;
            }
        }
    }
    println!();
    println!("Total: {} tokens", count);
    Ok(())
}
