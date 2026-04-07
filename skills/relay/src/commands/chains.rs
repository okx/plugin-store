use crate::rpc;
use clap::Args;

#[derive(Args)]
pub struct ChainsArgs {
    /// Filter by chain name (partial match, case-insensitive)
    #[arg(long)]
    pub filter: Option<String>,
}

pub fn run(args: ChainsArgs) -> anyhow::Result<()> {
    let data = rpc::get_chains()?;
    let chains = data["chains"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Unexpected API response: missing 'chains' field"))?;

    let filter = args.filter.as_deref().unwrap_or("").to_lowercase();

    println!("=== Relay Supported Chains ===");
    let mut count = 0;
    for chain in chains {
        let id = chain["id"].as_u64().unwrap_or(0);
        let name = chain["displayName"].as_str().unwrap_or("Unknown");
        let deposit_enabled = chain["depositEnabled"].as_bool().unwrap_or(false);
        let disabled = chain["disabled"].as_bool().unwrap_or(false);
        let currency = chain["currency"]["symbol"].as_str().unwrap_or("ETH");

        if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
            continue;
        }

        let status = if disabled {
            "disabled"
        } else if deposit_enabled {
            "active"
        } else {
            "receive-only"
        };

        println!("  {:>6}  {:<30}  {}  [{}]", id, name, currency, status);
        count += 1;
    }
    println!();
    println!("Total: {} chains", count);
    Ok(())
}
