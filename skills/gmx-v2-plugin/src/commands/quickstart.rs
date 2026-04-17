use clap::Args;
use serde_json::json;

/// Native USDC on Arbitrum One (6 decimals)
const USDC_ARBITRUM: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
/// Native USDC.e (bridged) on Avalanche (6 decimals)
const USDC_AVALANCHE: &str = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6C";
/// Minimum ETH for Arbitrum execution fees (2× single fee of 0.001 ETH)
const MIN_ETH_FEE_WEI: u128 = 2_000_000_000_000_000; // 0.002 ETH
/// Minimum AVAX for Avalanche execution fees (2× single fee of 0.012 AVAX)
const MIN_AVAX_FEE_WEI: u128 = 24_000_000_000_000_000; // 0.024 AVAX

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to currently logged-in onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(chain: &str, args: QuickstartArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = match args.address {
        Some(addr) => addr,
        None => crate::onchainos::resolve_wallet(cfg.chain_id)?,
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --address or ensure onchainos is logged in.");
    }

    eprintln!("Checking assets for {} on {}...", &wallet[..std::cmp::min(10, wallet.len())], chain);

    // Pick USDC address for this chain
    let usdc_addr = match chain.to_lowercase().as_str() {
        "arbitrum" | "arb" | "42161" => USDC_ARBITRUM,
        _ => USDC_AVALANCHE,
    };

    // Fetch in parallel: native balance, USDC balance, and open positions
    let datastore_clean = cfg.datastore.trim_start_matches("0x");
    let wallet_clean = wallet.trim_start_matches("0x");
    let positions_calldata = format!(
        "0x77cfb162{:0>64}{:0>64}{:064x}{:064x}",
        datastore_clean, wallet_clean, 0u128, 20u128
    );

    let (native_wei, usdc_raw, positions_raw) = tokio::join!(
        crate::rpc::get_eth_balance(&wallet, cfg.rpc_url),
        crate::rpc::check_erc20_balance(cfg.rpc_url, usdc_addr, &wallet),
        crate::rpc::eth_call(cfg.reader, &positions_calldata, cfg.rpc_url),
    );

    let usdc_units = usdc_raw.unwrap_or(0);
    let usdc_balance = usdc_units as f64 / 1_000_000.0;
    let native_balance = native_wei as f64 / 1e18;

    // Count positions from ABI-decoded response (only need the length)
    let position_count = count_positions(positions_raw.as_deref().unwrap_or(""));

    // Build suggestion
    let (native_symbol, min_fee_wei) = if chain.to_lowercase().contains("aval") || chain == "43114" {
        ("AVAX", MIN_AVAX_FEE_WEI)
    } else {
        ("ETH", MIN_ETH_FEE_WEI)
    };

    let (status, suggestion, next_command) = build_suggestion(
        chain,
        native_wei,
        min_fee_wei,
        native_symbol,
        usdc_balance,
        position_count,
    );

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "wallet": wallet,
            "chain": chain,
            "assets": {
                format!("{}_balance", native_symbol.to_lowercase()): native_balance,
                "usdc_balance": usdc_balance,
                "open_positions": position_count,
            },
            "status": status,
            "suggestion": suggestion,
            "next_command": next_command,
        }))?
    );
    Ok(())
}

/// Decode just the array length from getAccountPositions ABI response.
fn count_positions(raw: &str) -> usize {
    let data = raw.trim_start_matches("0x");
    if data.len() < 128 {
        return 0;
    }
    let offset = usize::from_str_radix(&data[0..64], 16).unwrap_or(0) * 2;
    if data.len() < offset + 64 {
        return 0;
    }
    usize::from_str_radix(&data[offset..offset + 64], 16).unwrap_or(0)
}

fn build_suggestion(
    chain: &str,
    native_wei: u128,
    min_fee_wei: u128,
    native_symbol: &str,
    usdc_balance: f64,
    position_count: usize,
) -> (&'static str, String, String) {
    // Case 1: active trader — has open positions
    if position_count > 0 {
        return (
            "active",
            format!("You have {} open position(s) on GMX V2 ({}). Review them below.", position_count, chain),
            format!("gmx-v2 --chain {} get-positions", chain),
        );
    }

    // Case 2: funded and ready — has execution fee + collateral
    if native_wei >= min_fee_wei && usdc_balance >= 10.0 {
        return (
            "ready",
            format!(
                "You have {} and {:.2} USDC on {}. Ready to open a position.",
                native_symbol, usdc_balance, chain
            ),
            format!("gmx-v2 --chain {} list-markets", chain),
        );
    }

    // Case 3: has USDC but missing execution fee
    if usdc_balance >= 10.0 && native_wei < min_fee_wei {
        let min_fee = min_fee_wei as f64 / 1e18;
        return (
            "needs_fee",
            format!(
                "You have {:.2} USDC but need at least {:.3} {} for execution fees. Get some {} first.",
                usdc_balance, min_fee, native_symbol, native_symbol
            ),
            format!("gmx-v2 --chain {} get-prices --token {}", chain, native_symbol),
        );
    }

    // Case 4: has execution fee but no collateral
    if native_wei >= min_fee_wei && usdc_balance < 10.0 {
        return (
            "needs_collateral",
            format!(
                "You have {} for fees but need at least $10 USDC as collateral to open a position.",
                native_symbol
            ),
            format!("gmx-v2 --chain {} get-prices", chain),
        );
    }

    // Case 5: new user — nothing
    (
        "no_funds",
        format!(
            "No funds found on {}. Transfer USDC and {} to your wallet to start trading on GMX V2 (minimum ~$10 USDC + 0.002 {} for fees).",
            chain, native_symbol, native_symbol
        ),
        format!("gmx-v2 --chain {} get-prices", chain),
    )
}
