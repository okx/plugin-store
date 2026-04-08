use anyhow::Result;
use clap::Args;

use crate::config::{known_coin_type, MARKET_OBJECT, PROTOCOL_PACKAGE, VERSION_OBJECT};

#[derive(Args, Debug)]
pub struct SupplyArgs {
    /// Asset to supply (e.g. sui, usdc, sca)
    #[arg(long)]
    pub asset: String,

    /// Amount to supply (human-readable, e.g. 10.5)
    #[arg(long)]
    pub amount: String,

    /// Show preview without submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit transaction (Sui support required)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: &SupplyArgs) -> Result<()> {
    let asset = args.asset.to_lowercase();
    let coin_type = known_coin_type(&asset)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("<coin_type_for_{}>", asset));

    // Parse amount string (no f64 arithmetic — keep as string for display)
    let amount_str = &args.amount;

    if !args.confirm || args.dry_run {
        let preview = serde_json::json!({
            "preview": true,
            "action": "supply",
            "protocol": "Scallop Lend",
            "chain": "Sui mainnet",
            "asset": asset.to_uppercase(),
            "coin_type": coin_type,
            "amount": amount_str,
            "move_call": {
                "package": PROTOCOL_PACKAGE,
                "module": "lending_core_open",
                "function": "supply",
                "type_args": [coin_type],
                "args": [
                    VERSION_OBJECT,
                    MARKET_OBJECT,
                    "<coin_object_id>",
                    "<clock_object_0x6>"
                ]
            },
            "note": "Sui transaction submission requires onchainos Sui support. Use --confirm when available."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    anyhow::bail!(
        "Sui transaction submission not yet supported by onchainos CLI. \
         Use --dry-run to preview the transaction details."
    )
}
