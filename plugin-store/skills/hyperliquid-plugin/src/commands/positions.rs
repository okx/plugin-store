use clap::Args;
use crate::api::{get_clearinghouse_state_for_dex, get_open_orders_for_dex};
use crate::config::{info_url, CHAIN_ID};
use crate::onchainos::resolve_wallet;

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
    /// HIP-3 builder DEX name (xyz / flx / vntl / hyna / km / cash / para / abcd).
    /// If omitted, queries the default Hyperliquid perp DEX. Each builder DEX has
    /// SEPARATE margin and positions — use `dex-list` for an overview.
    #[arg(long)]
    pub dex: Option<String>,
    /// Also show open orders for the address.
    #[arg(long)]
    pub show_orders: bool,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let url = info_url();

    let address = match args.address {
        Some(addr) => addr,
        None => match resolve_wallet(CHAIN_ID) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
                return Ok(());
            }
        },
    };

    let dex_arg = args.dex.as_deref();
    let dex_label = dex_arg.unwrap_or("default");
    eprintln!("Fetching Hyperliquid {} DEX positions for: {}", dex_label, address);

    let state = match get_clearinghouse_state_for_dex(url, &address, dex_arg).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };

    // Parse margin summary
    let margin = &state["marginSummary"];
    let account_value = margin["accountValue"].as_str().unwrap_or("0");
    let total_margin_used = margin["totalMarginUsed"].as_str().unwrap_or("0");
    let total_ntl_pos = margin["totalNtlPos"].as_str().unwrap_or("0");
    let withdrawable = state["withdrawable"].as_str().unwrap_or("0");

    // Parse asset positions
    let empty_vec = vec![];
    let asset_positions = state["assetPositions"].as_array().unwrap_or(&empty_vec);

    let mut positions_out = Vec::new();
    for pos_wrapper in asset_positions {
        let pos = &pos_wrapper["position"];
        let coin = pos["coin"].as_str().unwrap_or("?");
        let szi = pos["szi"].as_str().unwrap_or("0");
        let entry_px = pos["entryPx"].as_str().unwrap_or("0");
        let unrealized_pnl = pos["unrealizedPnl"].as_str().unwrap_or("0");
        let return_on_equity = pos["returnOnEquity"].as_str().unwrap_or("0");
        let liquidation_px = pos["liquidationPx"].as_str();
        let margin_used = pos["marginUsed"].as_str().unwrap_or("0");
        let position_value = pos["positionValue"].as_str().unwrap_or("0");

        let leverage_type = pos["leverage"]["type"].as_str().unwrap_or("cross");
        let leverage_value = pos["leverage"]["value"].as_u64().unwrap_or(0);

        let cum_funding_all_time = pos["cumFunding"]["allTime"].as_str().unwrap_or("0");

        let side = if szi.starts_with('-') { "short" } else { "long" };

        positions_out.push(serde_json::json!({
            "coin": coin,
            "side": side,
            "size": szi,
            "entryPrice": entry_px,
            "unrealizedPnl": unrealized_pnl,
            "returnOnEquity": return_on_equity,
            "liquidationPrice": liquidation_px,
            "marginUsed": margin_used,
            "positionValue": position_value,
            "leverage": {
                "type": leverage_type,
                "value": leverage_value
            },
            "cumulativeFunding": cum_funding_all_time
        }));
    }

    let mut out = serde_json::json!({
        "ok": true,
        "address": address,
        "dex": dex_label,
        "accountValue": account_value,
        "totalMarginUsed": total_margin_used,
        "totalNotionalPosition": total_ntl_pos,
        "withdrawable": withdrawable,
        "positions": positions_out
    });

    if args.show_orders {
        let orders = match get_open_orders_for_dex(url, &address, dex_arg).await {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
                return Ok(());
            }
        };
        out["openOrders"] = orders;
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
