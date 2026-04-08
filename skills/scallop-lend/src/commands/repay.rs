use anyhow::Result;
use clap::Args;

use crate::config::{known_coin_type, MARKET_OBJECT, PROTOCOL_PACKAGE, VERSION_OBJECT};

#[derive(Args, Debug)]
pub struct RepayArgs {
    /// Asset to repay (e.g. usdc, sui)
    #[arg(long)]
    pub asset: String,

    /// Amount to repay (human-readable). Use "max" to repay full debt.
    #[arg(long)]
    pub amount: String,

    /// Show preview without submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit transaction (Sui support required)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: &RepayArgs) -> Result<()> {
    let asset = args.asset.to_lowercase();
    let coin_type = known_coin_type(&asset)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("<coin_type_for_{}>", asset));

    let amount_str = &args.amount;
    let is_max = amount_str.to_lowercase() == "max";

    if !args.confirm || args.dry_run {
        let preview = serde_json::json!({
            "preview": true,
            "action": "repay",
            "protocol": "Scallop Lend",
            "chain": "Sui mainnet",
            "asset": asset.to_uppercase(),
            "coin_type": coin_type,
            "amount": amount_str,
            "repay_max": is_max,
            "move_call": {
                "package": PROTOCOL_PACKAGE,
                "module": "lending_core_open",
                "function": "repay",
                "type_args": [coin_type],
                "args": [
                    VERSION_OBJECT,
                    MARKET_OBJECT,
                    "<obligation_object_id>",
                    "<coin_object_id>",
                    "<clock_object_0x6>"
                ]
            },
            "note": "Repay reduces debt on the specified Obligation. Pass your coin object containing the repayment amount. Sui transaction submission requires onchainos Sui support."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    anyhow::bail!(
        "Sui transaction submission not yet supported by onchainos CLI. \
         Use --dry-run to preview the transaction details."
    )
}
