use clap::Args;
use crate::api::{
    fetch_perp_dexs, get_clearinghouse_state, get_clearinghouse_state_for_dex,
    get_spot_clearinghouse_state, parse_outcome_coin, BuilderDex,
};
use crate::config::{info_url, ARBITRUM_CHAIN_ID, USDC_ARBITRUM};
use crate::onchainos::resolve_wallet;
use crate::rpc::{ARBITRUM_RPC, erc20_balance};

const ABOUT: &str = "Hyperliquid is a high-performance on-chain perpetuals DEX - trade BTC/ETH/SOL on the default DEX, RWAs (CL, NVDA, SP500, etc.) on HIP-3 builder DEXs, and binary YES/NO outcomes on HIP-4 prediction markets.";

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    // 1. Resolve wallet
    let wallet = match args.address {
        Some(addr) => addr,
        None => resolve_wallet(ARBITRUM_CHAIN_ID)?,
    };

    eprintln!("Checking assets for {}...", &wallet[..std::cmp::min(10, wallet.len())]);

    let url = info_url();

    // 2. Phase A: parallel-fetch Arbitrum balance + default DEX state + builder DEX
    //    registry + spot clearinghouse (which contains USDH balance and HIP-4 outcome legs).
    let (arb_result, hl_default_result, registry_result, spot_result) = tokio::join!(
        erc20_balance(USDC_ARBITRUM, &wallet, ARBITRUM_RPC),
        get_clearinghouse_state(url, &wallet),
        fetch_perp_dexs(url),
        get_spot_clearinghouse_state(url, &wallet),
    );

    let arb_usdc_units = arb_result.unwrap_or(0);
    let arb_usdc = arb_usdc_units as f64 / 1_000_000.0;

    // Parse spot state for USDH balance + HIP-4 outcome positions (`+N` coins)
    let (usdh_balance, outcome_positions_count, outcome_positions_detail) =
        parse_spot_for_outcomes(spot_result.as_ref().ok());

    // Parse default DEX state
    let (hl_account_value, hl_withdrawable, default_positions, default_positions_detail) =
        parse_clearinghouse(hl_default_result.as_ref().ok());

    // 3. Phase B: parallel-fetch user's clearinghouse state on every builder DEX
    let registry: Vec<BuilderDex> = registry_result.unwrap_or_default();
    let builder_futs: Vec<_> = registry.iter().map(|d| {
        let wallet = wallet.clone();
        let dex_name = d.name.clone();
        async move {
            let state = get_clearinghouse_state_for_dex(url, &wallet, Some(&dex_name)).await;
            (dex_name, state.ok())
        }
    }).collect();
    let builder_states: Vec<(String, Option<serde_json::Value>)> =
        futures::future::join_all(builder_futs).await;

    // Aggregate builder DEX summary
    let mut builder_summary = Vec::with_capacity(builder_states.len());
    let mut builder_total_value = 0.0f64;
    let mut builder_total_positions = 0usize;
    let mut richest_builder_dex: Option<(String, f64)> = None;
    for (name, state) in &builder_states {
        let (value, _wd, positions, _detail) = parse_clearinghouse(state.as_ref());
        builder_total_value += value;
        builder_total_positions += positions.len();
        if value > 0.0 {
            if richest_builder_dex.as_ref().map(|(_, v)| value > *v).unwrap_or(true) {
                richest_builder_dex = Some((name.clone(), value));
            }
        }
        builder_summary.push(serde_json::json!({
            "dex":             name,
            "account_value":   value,
            "position_count":  positions.len(),
            "position_coins":  positions,
        }));
    }

    // 4. Decide status + next_command. Priority order:
    //    has_outcome_position > has_builder_dex_position > active > ready > needs_deposit > low_balance > no_funds
    let (status, suggestion, onboarding_steps, next_command) = build_suggestion(
        &wallet, arb_usdc, hl_account_value,
        &default_positions, builder_total_positions,
        builder_total_value, &richest_builder_dex,
        outcome_positions_count, usdh_balance,
    );

    let mut out = serde_json::json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "assets": {
            "arb_usdc_balance":         arb_usdc,
            "hl_default_account_value": hl_account_value,
            "hl_default_withdrawable":  hl_withdrawable,
            "hl_default_positions":     default_positions.len(),
            "hl_builder_total_value":   builder_total_value,
            "hl_builder_total_positions": builder_total_positions,
            "spot_usdh_balance":        usdh_balance,
            "hip4_outcome_positions":   outcome_positions_count,
        },
        "default_dex_positions":  default_positions_detail,
        "builder_dexs":           builder_summary,
        "builder_dex_count":      registry.len(),
        "outcome_positions":      outcome_positions_detail,
        "status":       status,
        "suggestion":   suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = serde_json::json!(onboarding_steps);
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Decode HIP-4 outcome state from spotClearinghouseState. Returns
/// (usdh_balance, outcome_positions_count, outcome_positions_detail).
/// Outcome positions are spot balances with coin starting with `+`.
fn parse_spot_for_outcomes(state: Option<&serde_json::Value>)
    -> (f64, usize, Vec<serde_json::Value>)
{
    let s = match state { Some(v) => v, None => return (0.0, 0, vec![]) };
    let empty = vec![];
    let balances = s["balances"].as_array().unwrap_or(&empty);
    let mut usdh = 0.0_f64;
    let mut detail: Vec<serde_json::Value> = Vec::new();
    for b in balances {
        let coin = match b["coin"].as_str() { Some(c) => c, None => continue };
        let total: f64 = b["total"].as_str().and_then(|x| x.parse().ok()).unwrap_or(0.0);
        if coin == "USDH" {
            usdh = total;
            continue;
        }
        if coin.starts_with('+') && total != 0.0 {
            if let Some((outcome_id, side)) = parse_outcome_coin(coin) {
                let entry_ntl: f64 = b["entryNtl"].as_str().and_then(|x| x.parse().ok()).unwrap_or(0.0);
                detail.push(serde_json::json!({
                    "balance_coin": coin,
                    "outcome_id":   outcome_id,
                    "side":         if side == 0 { "yes" } else { "no" },
                    "size":         total,
                    "entry_ntl_usdh": entry_ntl,
                }));
            }
        }
    }
    (usdh, detail.len(), detail)
}

/// Decode account_value, withdrawable, position-coins, position-detail from a
/// clearinghouse state response. Returns zeros / empty if state is None.
fn parse_clearinghouse(state: Option<&serde_json::Value>)
    -> (f64, f64, Vec<String>, Vec<serde_json::Value>)
{
    let s = match state { Some(v) => v, None => return (0.0, 0.0, vec![], vec![]) };
    let margin = &s["marginSummary"];
    let account_value: f64 = margin["accountValue"]
        .as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let withdrawable: f64 = s["withdrawable"]
        .as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let empty = vec![];
    let asset_positions = s["assetPositions"].as_array().unwrap_or(&empty);
    let coins: Vec<String> = asset_positions.iter()
        .filter_map(|p| p["position"]["coin"].as_str().map(|s| s.to_string()))
        .collect();
    let detail: Vec<serde_json::Value> = asset_positions.iter().map(|p| {
        let pos = &p["position"];
        let szi = pos["szi"].as_str().unwrap_or("0");
        serde_json::json!({
            "coin":         pos["coin"].as_str().unwrap_or("?"),
            "side":         if szi.starts_with('-') { "short" } else { "long" },
            "size":         szi,
            "entryPrice":   pos["entryPx"].as_str().unwrap_or("0"),
            "unrealizedPnl": pos["unrealizedPnl"].as_str().unwrap_or("0"),
        })
    }).collect();
    (account_value, withdrawable, coins, detail)
}

/// Returns (status, human-readable suggestion, onboarding_steps, ready-to-run command).
fn build_suggestion(
    wallet: &str,
    arb_usdc: f64,
    hl_account_value: f64,
    default_positions: &[String],
    builder_total_positions: usize,
    builder_total_value: f64,
    richest_builder: &Option<(String, f64)>,
    outcome_positions_count: usize,
    usdh_balance: f64,
) -> (&'static str, String, Vec<String>, String) {
    // Case 0 (HIP-4): user has open outcome positions — review/manage them first
    if outcome_positions_count > 0 {
        return (
            "has_outcome_position",
            format!(
                "You have {} open HIP-4 outcome position(s). Settlement is automatic at expiry; sell early if you want to lock in or change exposure.",
                outcome_positions_count
            ),
            vec![],
            "hyperliquid-plugin outcome-positions".to_string(),
        );
    }

    // Case 0a (NEW HIP-3): user has positions on a builder DEX
    if builder_total_positions > 0 {
        let dex = richest_builder.as_ref().map(|(n, _)| n.clone()).unwrap_or_else(|| "xyz".to_string());
        return (
            "has_builder_dex_position",
            format!("You have {} open position(s) on HIP-3 builder DEX(s). Review them below or run `positions --dex <name>` for detail.",
                builder_total_positions),
            vec![],
            format!("hyperliquid-plugin positions --dex {}", dex),
        );
    }

    // Case 0b (HIP-4): user has USDH but no outcome positions yet → suggest outcome-list
    if usdh_balance >= 0.5 && outcome_positions_count == 0 {
        return (
            "ready",
            format!(
                "You have {:.4} USDH on spot. List HIP-4 outcome markets to deploy it (binary YES/NO contracts on real-world events).",
                usdh_balance
            ),
            vec![
                "1. List active outcome markets:".to_string(),
                "   hyperliquid-plugin outcome-list".to_string(),
                "2. Buy a YES or NO leg (e.g. on the BTC-79980-1d outcome, betting YES that BTC > $79980 in 1d):".to_string(),
                "   hyperliquid-plugin outcome-buy --outcome BTC-79980-1d --side yes --shares 1 --price 0.65 --confirm".to_string(),
                "3. Track positions: hyperliquid-plugin outcome-positions".to_string(),
            ],
            "hyperliquid-plugin outcome-list".to_string(),
        );
    }

    // Case 1: active trader on default DEX
    if !default_positions.is_empty() {
        return (
            "active",
            "You have open positions on the default Hyperliquid perp DEX. Review them below.".to_string(),
            vec![],
            "hyperliquid-plugin positions".to_string(),
        );
    }

    // Case 2 (NEW HIP-3): user has USDC on a builder DEX but no position
    if builder_total_value >= 1.0 {
        let dex = richest_builder.as_ref().map(|(n, _)| n.clone()).unwrap_or_else(|| "xyz".to_string());
        return (
            "ready",
            format!("You have ${:.2} USDC on builder DEX `{}`. Place a trade on a HIP-3 RWA market.",
                builder_total_value, dex),
            vec![
                format!("1. Inspect the {} DEX universe (RWAs / commodities / equities):", dex),
                format!("   hyperliquid-plugin prices --dex {}", dex),
                format!("2. Preview an order on a builder-DEX coin (e.g. xyz:CL = WTI Crude):"),
                format!("   hyperliquid-plugin order --coin {}:CL --side buy --size 50 --leverage 5", dex),
                format!("3. Add --confirm to submit the order."),
            ],
            format!("hyperliquid-plugin prices --dex {}", dex),
        );
    }

    // Case 3: funded on default DEX, no positions yet
    if hl_account_value >= 1.0 {
        return (
            "ready",
            "Your default-DEX Hyperliquid perp account is funded. Place your first trade, OR fund a builder DEX (HIP-3) for RWA trading.".to_string(),
            vec![
                "1. Check default DEX markets (BTC/ETH/SOL/...):  hyperliquid-plugin prices".to_string(),
                "2. Or list builder DEXs and their RWA markets:".to_string(),
                "   hyperliquid-plugin dex-list".to_string(),
                "   hyperliquid-plugin prices --dex xyz       # RWAs (CL/BRENTOIL/NVDA/TSLA/etc)".to_string(),
                "3. Preview a default-DEX trade (no --confirm = preview only):".to_string(),
                "   hyperliquid-plugin order --coin BTC --side buy --size 10 --leverage 5".to_string(),
                "4. Or fund builder DEX first (e.g. xyz) for RWAs:".to_string(),
                "   hyperliquid-plugin dex-transfer --to-dex xyz --amount 5 --confirm".to_string(),
                "5. When ready, add --confirm to execute:".to_string(),
                "   hyperliquid-plugin order --coin BTC --side buy --size 10 --leverage 5 --confirm".to_string(),
            ],
            "hyperliquid-plugin order --coin BTC --side buy --size 10 --leverage 5".to_string(),
        );
    }

    // Case 4: has enough Arbitrum USDC to deposit (min $5)
    if arb_usdc >= 5.0 {
        let suggest = ((arb_usdc * 0.9 * 100.0).floor() / 100.0).max(5.0);
        let suggest = suggest.min(arb_usdc);
        return (
            "needs_deposit",
            "You have USDC on Arbitrum. Deposit to Hyperliquid to start trading perps (minimum $5).".to_string(),
            vec![
                format!("1. Deposit USDC from Arbitrum to Hyperliquid (default DEX, minimum $5):"),
                format!("   hyperliquid-plugin deposit --amount {:.2} --confirm", suggest),
                "2. After deposit confirms, optionally fund a builder DEX for RWAs:".to_string(),
                "   hyperliquid-plugin dex-transfer --to-dex xyz --amount 5 --confirm".to_string(),
                "3. Run quickstart again to confirm balances on default + builder DEXs.".to_string(),
                "4. Place a trade:".to_string(),
                "   hyperliquid-plugin order --coin BTC --side buy --size 10 --leverage 5 --confirm".to_string(),
            ],
            format!("hyperliquid-plugin deposit --amount {:.2} --confirm", suggest),
        );
    }

    // Case 5: some Arbitrum USDC but below $5 minimum
    if arb_usdc > 0.0 {
        return (
            "low_balance",
            "You have some USDC on Arbitrum but below the $5 deposit minimum. Add more USDC to your Arbitrum wallet.".to_string(),
            vec![
                format!("1. Send at least $5 USDC to your Arbitrum wallet:"),
                format!("   {}", wallet),
                "2. Run quickstart again to check your balance:".to_string(),
                "   hyperliquid-plugin quickstart".to_string(),
                "3. Then deposit to Hyperliquid:".to_string(),
                "   hyperliquid-plugin deposit --amount 5 --confirm".to_string(),
            ],
            "hyperliquid-plugin address".to_string(),
        );
    }

    // Case 6: no funds anywhere
    (
        "no_funds",
        "No USDC found on Arbitrum or Hyperliquid. Transfer USDC to your Arbitrum wallet, then deposit (minimum $5).".to_string(),
        vec![
            "1. Send USDC to your Arbitrum wallet (minimum $5):".to_string(),
            format!("   {}", wallet),
            "2. Run quickstart again to confirm your balance:".to_string(),
            "   hyperliquid-plugin quickstart".to_string(),
            "3. Deposit USDC to Hyperliquid:".to_string(),
            "   hyperliquid-plugin deposit --amount <amount> --confirm".to_string(),
            "4. Place your first trade:".to_string(),
            "   hyperliquid-plugin order --coin BTC --side buy --size 10 --leverage 5 --confirm".to_string(),
        ],
        "hyperliquid-plugin address".to_string(),
    )
}
