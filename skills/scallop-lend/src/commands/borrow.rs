use anyhow::Result;
use clap::Args;

use crate::config::{known_coin_type, MARKET_OBJECT, PROTOCOL_PACKAGE, VERSION_OBJECT};

#[derive(Args, Debug)]
pub struct BorrowArgs {
    /// Asset to borrow (e.g. usdc, sui)
    #[arg(long)]
    pub asset: String,

    /// Amount to borrow (human-readable)
    #[arg(long)]
    pub amount: String,

    /// Show preview without submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit transaction (Sui support required)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: &BorrowArgs) -> Result<()> {
    let asset = args.asset.to_lowercase();
    let coin_type = known_coin_type(&asset)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("<coin_type_for_{}>", asset));

    let amount_str = &args.amount;

    if !args.confirm || args.dry_run {
        let preview = serde_json::json!({
            "preview": true,
            "action": "borrow",
            "protocol": "Scallop Lend",
            "chain": "Sui mainnet",
            "asset": asset.to_uppercase(),
            "coin_type": coin_type,
            "amount": amount_str,
            "prerequisites": [
                "Must have an existing Obligation object with sufficient collateral",
                "Collateral health factor must remain above 1.0 after borrow"
            ],
            "move_call": {
                "package": PROTOCOL_PACKAGE,
                "module": "lending_core_open",
                "function": "borrow",
                "type_args": [coin_type],
                "args": [
                    VERSION_OBJECT,
                    MARKET_OBJECT,
                    "<obligation_object_id>",
                    "<obligation_key_object_id>",
                    "<amount_u64>",
                    "<clock_object_0x6>"
                ]
            },
            "note": "Borrow creates or uses an existing Obligation. Must have collateral deposited first. Sui transaction submission requires onchainos Sui support."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    anyhow::bail!(
        "Sui transaction submission not yet supported by onchainos CLI. \
         Use --dry-run to preview the transaction details."
    )
}
