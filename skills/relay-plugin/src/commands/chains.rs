use clap::Args;
use crate::api::get_chains;

#[derive(Args)]
pub struct ChainsArgs {
    /// Filter by chain name or ID (optional)
    #[arg(long)]
    pub filter: Option<String>,
}

pub async fn run(args: ChainsArgs) -> anyhow::Result<()> {
    let mut chains = get_chains().await?;

    // Filter out disabled chains
    chains.retain(|c| c.disabled != Some(true));

    // Apply optional name/id filter
    if let Some(f) = &args.filter {
        let f_lower = f.to_lowercase();
        chains.retain(|c| {
            c.name.to_lowercase().contains(&f_lower)
                || c.id.to_string() == f_lower
                || c.display_name.as_deref().unwrap_or("").to_lowercase().contains(&f_lower)
        });
    }

    let out: Vec<serde_json::Value> = chains.iter().map(|c| {
        serde_json::json!({
            "chain_id":    c.id,
            "name":        c.display_name.as_deref().unwrap_or(&c.name),
            "slug":        c.name,
            "native_token": c.currency.as_ref().map(|cu| cu.symbol.as_str()).unwrap_or("ETH"),
            "explorer":    c.explorer_url.as_deref().unwrap_or(""),
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
