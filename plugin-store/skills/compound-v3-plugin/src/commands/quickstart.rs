use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

const ABOUT: &str = "Compound V3 (Comet) is an on-chain lending protocol. Each market has one base asset you can supply to earn interest or borrow against collateral. Supported chains: Ethereum (1), Base (8453), Arbitrum (42161), Polygon (137).";

pub async fn run(chain_id: u64, market: &str, wallet: Option<String>) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;

    let wallet_addr = match wallet {
        Some(w) => w,
        None => {
            let w = onchainos::resolve_wallet(chain_id)?;
            if w.is_empty() {
                anyhow::bail!("Cannot resolve wallet address. Pass --wallet or log in via onchainos.");
            }
            w
        }
    };

    // Parallel fetch: Comet supply + borrow balance. Tolerate RPC errors silently —
    // this command is a status probe, not a trading command.
    let (supply_res, borrow_res) = tokio::join!(
        rpc::get_balance_of(cfg.comet_proxy, &wallet_addr, cfg.rpc_url),
        rpc::get_borrow_balance_of(cfg.comet_proxy, &wallet_addr, cfg.rpc_url),
    );

    let supply_raw = supply_res.unwrap_or(0);
    let borrow_raw = borrow_res.unwrap_or(0);

    let factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;
    let supply_balance = supply_raw as f64 / factor;
    let borrow_balance = borrow_raw as f64 / factor;

    let (status, suggestion, next_command) =
        build_suggestion(chain_id, market, cfg.base_asset_symbol, supply_balance, borrow_balance);

    let out = serde_json::json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet_addr,
        "chain_id": chain_id,
        "market": market,
        "base_asset": cfg.base_asset_symbol,
        "assets": {
            "comet_supply_balance":     format!("{:.6}", supply_balance),
            "comet_supply_balance_raw": supply_raw.to_string(),
            "comet_borrow_balance":     format!("{:.6}", borrow_balance),
            "comet_borrow_balance_raw": borrow_raw.to_string(),
        },
        "status":       status,
        "suggestion":   suggestion,
        "next_command": next_command,
    });

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Returns (status, human-readable suggestion, ready-to-run command).
fn build_suggestion(
    chain_id: u64,
    market: &str,
    base_asset_symbol: &str,
    supply_balance: f64,
    borrow_balance: f64,
) -> (&'static str, String, String) {
    // 1. borrowed — active borrow position; review health and plan repay
    if borrow_balance > 0.0 {
        return (
            "borrowed",
            format!(
                "You have an active borrow of {:.6} {}. Review your position and health factor; repay when ready.",
                borrow_balance, base_asset_symbol
            ),
            format!(
                "compound-v3-plugin --chain {} --market {} get-position",
                chain_id, market
            ),
        );
    }

    // 2. earning — supply base asset, no borrow
    if supply_balance > 0.0 {
        return (
            "earning",
            format!(
                "You are supplying {:.6} {} and earning interest. No active borrow.",
                supply_balance, base_asset_symbol
            ),
            format!(
                "compound-v3-plugin --chain {} --market {} get-position",
                chain_id, market
            ),
        );
    }

    // 3. new_user — no Comet position on this market
    (
        "new_user",
        format!(
            "No Compound V3 position on {} (chain {}). Browse current APRs and supply or collateralize to start.",
            market, chain_id
        ),
        format!(
            "compound-v3-plugin --chain {} --market {} get-markets",
            chain_id, market
        ),
    )
}
