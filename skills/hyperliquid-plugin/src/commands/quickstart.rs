use clap::Args;
use crate::api::get_clearinghouse_state;
use crate::config::{info_url, ARBITRUM_CHAIN_ID, USDC_ARBITRUM};
use crate::onchainos::resolve_wallet;
use crate::rpc::{ARBITRUM_RPC, erc20_balance};

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    // 1. Resolve wallet (EVM address shared by Arbitrum + Hyperliquid)
    let wallet = match args.address {
        Some(addr) => addr,
        None => resolve_wallet(ARBITRUM_CHAIN_ID)?,
    };

    eprintln!("Checking assets for {}...", &wallet[..std::cmp::min(10, wallet.len())]);

    // 2. Fetch in parallel: Arbitrum USDC balance + HL perp clearinghouse state
    let url = info_url();
    let (arb_result, hl_result) = tokio::join!(
        erc20_balance(USDC_ARBITRUM, &wallet, ARBITRUM_RPC),
        get_clearinghouse_state(url, &wallet),
    );

    let arb_usdc_units = arb_result.unwrap_or(0);
    let arb_usdc = arb_usdc_units as f64 / 1_000_000.0;

    // Parse HL clearinghouse state
    let (hl_account_value, hl_withdrawable, open_positions, positions_detail) = match hl_result {
        Ok(ref state) => {
            let margin = &state["marginSummary"];
            let account_value: f64 = margin["accountValue"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let withdrawable: f64 = state["withdrawable"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let empty = vec![];
            let asset_positions = state["assetPositions"].as_array().unwrap_or(&empty);
            let coins: Vec<String> = asset_positions
                .iter()
                .filter_map(|p| p["position"]["coin"].as_str().map(|s| s.to_string()))
                .collect();
            let detail: Vec<serde_json::Value> = asset_positions
                .iter()
                .map(|p| {
                    let pos = &p["position"];
                    let szi = pos["szi"].as_str().unwrap_or("0");
                    serde_json::json!({
                        "coin":         pos["coin"].as_str().unwrap_or("?"),
                        "side":         if szi.starts_with('-') { "short" } else { "long" },
                        "size":         szi,
                        "entryPrice":   pos["entryPx"].as_str().unwrap_or("0"),
                        "unrealizedPnl": pos["unrealizedPnl"].as_str().unwrap_or("0"),
                    })
                })
                .collect();
            (account_value, withdrawable, coins, detail)
        }
        Err(_) => (0.0, 0.0, vec![], vec![]),
    };

    // 3. Build guidance based on account state
    let (status, suggestion, next_command) =
        build_suggestion(arb_usdc, hl_account_value, &open_positions);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "wallet": wallet,
            "assets": {
                "arb_usdc_balance":     arb_usdc,
                "hl_account_value_usd": hl_account_value,
                "hl_withdrawable_usd":  hl_withdrawable,
                "hl_open_positions":    open_positions.len(),
            },
            "positions": positions_detail,
            "status":       status,
            "suggestion":   suggestion,
            "next_command": next_command,
        }))?
    );

    Ok(())
}

/// Returns (status, human-readable suggestion, ready-to-run command).
fn build_suggestion(
    arb_usdc: f64,
    hl_account_value: f64,
    open_positions: &[String],
) -> (&'static str, &'static str, String) {
    // Case 1: active trader — has open positions
    if !open_positions.is_empty() {
        return (
            "active",
            "You have open positions on Hyperliquid. Review them below.",
            "hyperliquid positions".to_string(),
        );
    }

    // Case 2: funded and ready — USDC on HL, no positions yet
    if hl_account_value >= 1.0 {
        return (
            "ready",
            "Your Hyperliquid perp account is funded. Place your first trade.",
            "hyperliquid order --coin BTC --side long --size 10 --leverage 5".to_string(),
        );
    }

    // Case 3: has enough Arbitrum USDC to deposit (minimum $5)
    if arb_usdc >= 5.0 {
        // Suggest depositing 90% and keeping a small buffer for gas, minimum $5
        let suggest = ((arb_usdc * 0.9 * 100.0).floor() / 100.0).max(5.0);
        let suggest = suggest.min(arb_usdc);
        return (
            "needs_deposit",
            "You have USDC on Arbitrum. Deposit to Hyperliquid to start trading perps (minimum $5).",
            format!("hyperliquid deposit --amount {:.2} --confirm", suggest),
        );
    }

    // Case 4: some Arbitrum USDC but below $5 minimum
    if arb_usdc > 0.0 {
        return (
            "low_balance",
            "You have some USDC on Arbitrum but below the $5 deposit minimum. Add more USDC to your Arbitrum wallet.",
            "hyperliquid address".to_string(),
        );
    }

    // Case 5: new user — no funds anywhere
    (
        "no_funds",
        "No USDC found on Arbitrum or Hyperliquid. Transfer USDC to your Arbitrum wallet, then deposit (minimum $5).",
        "hyperliquid address".to_string(),
    )
}
