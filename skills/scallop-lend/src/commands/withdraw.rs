use anyhow::Result;
use clap::Args;

use crate::config::{known_coin_type, MARKET_OBJECT, PROTOCOL_PACKAGE, VERSION_OBJECT};

#[derive(Args, Debug)]
pub struct WithdrawArgs {
    /// Asset to withdraw (e.g. sui, usdc)
    #[arg(long)]
    pub asset: String,

    /// Amount to withdraw (human-readable)
    #[arg(long)]
    pub amount: String,

    /// Show preview without submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit transaction (Sui support required)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: &WithdrawArgs) -> Result<()> {
    let asset = args.asset.to_lowercase();
    let coin_type = known_coin_type(&asset)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("<coin_type_for_{}>", asset));

    let amount_str = &args.amount;

    if !args.confirm || args.dry_run {
        // sCoin type placeholder — user needs to pass their sCoin object
        let scoin_module = format!("scallop_{}", asset);
        let scoin_type = format!(
            "<scallop_package>::{}::SCALLOP_{}",
            scoin_module,
            asset.to_uppercase()
        );

        let preview = serde_json::json!({
            "preview": true,
            "action": "withdraw",
            "protocol": "Scallop Lend",
            "chain": "Sui mainnet",
            "asset": asset.to_uppercase(),
            "coin_type": coin_type,
            "amount": amount_str,
            "move_call": {
                "package": PROTOCOL_PACKAGE,
                "module": "lending_core_open",
                "function": "withdraw",
                "type_args": [coin_type, scoin_type],
                "args": [
                    VERSION_OBJECT,
                    MARKET_OBJECT,
                    "<scoin_object_id>",
                    "<clock_object_0x6>"
                ]
            },
            "note": "Withdraw burns your sCoin (e.g. sSUI, sUSDC) and returns the underlying asset. Sui transaction submission requires onchainos Sui support."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    anyhow::bail!(
        "Sui transaction submission not yet supported by onchainos CLI. \
         Use --dry-run to preview the transaction details."
    )
}
