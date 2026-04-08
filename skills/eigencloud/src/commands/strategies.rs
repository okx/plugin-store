use clap::Args;
use serde_json::json;

use crate::config::{lst_strategies, rpc_url};
use crate::rpc::{format_eth, get_total_shares};

#[derive(Args)]
pub struct StrategiesArgs {
    /// Show only strategies with non-zero TVL
    #[arg(long)]
    pub active_only: bool,
}

pub async fn run(args: StrategiesArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let mut strategies = vec![];

    for strat in lst_strategies() {
        let total_shares = get_total_shares(strat.strategy, rpc)
            .await
            .unwrap_or(0);

        if args.active_only && total_shares == 0 {
            continue;
        }

        strategies.push(json!({
            "symbol": strat.symbol,
            "tokenAddress": strat.token,
            "strategyAddress": strat.strategy,
            "decimals": strat.decimals,
            "totalSharesWei": total_shares.to_string(),
            "totalSharesFormatted": format_eth(total_shares)
        }));
    }

    let output = json!({
        "ok": true,
        "chain": "ethereum",
        "chainId": 1,
        "strategyManager": crate::config::strategy_manager(),
        "count": strategies.len(),
        "strategies": strategies
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
