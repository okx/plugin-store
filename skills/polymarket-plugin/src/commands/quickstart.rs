use clap::Args;
use reqwest::Client;

use crate::api::{check_clob_access, get_positions, Position};
use crate::config::load_credentials;
use crate::onchainos::{get_pol_balance, get_usdc_balance, get_wallet_address};

const ABOUT: &str = "Polymarket is the largest prediction-market protocol on Polygon — trade YES/NO outcome tokens on real-world events with USDC.e. This skill supports both EOA and Polymarket proxy (gasless) trading modes.";

/// Minimum USD balance for a first deposit or first trade.
const MIN_FUND: f64 = 5.0;

/// Minimum POL balance considered sufficient to run `setup-proxy` (needs one tx gas).
const MIN_POL_FOR_SETUP: f64 = 0.05;

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    let client = Client::new();

    // 1. Resolve EOA wallet (fails fast if onchainos CLI is not logged in)
    let eoa = match args.address {
        Some(addr) => addr,
        None => get_wallet_address().await?,
    };

    eprintln!(
        "Checking Polymarket status for {}...",
        &eoa[..std::cmp::min(10, eoa.len())]
    );

    // 2. Read local creds — proxy_wallet is Some(addr) after `setup-proxy` has run
    let proxy: Option<String> = load_credentials()
        .ok()
        .flatten()
        .and_then(|c| c.proxy_wallet);

    // 3. Positions belong to the maker wallet — proxy if it exists, else EOA
    let primary_wallet = proxy.clone().unwrap_or_else(|| eoa.clone());

    // 4. Parallel fetch: CLOB access + EOA POL + EOA USDC.e + positions
    let (access_result, pol_result, eoa_usdc_result, positions_result) = tokio::join!(
        check_clob_access(&client),
        get_pol_balance(&eoa),
        get_usdc_balance(&eoa),
        get_positions(&client, &primary_wallet),
    );

    // 5. Optional: proxy USDC balance (only if proxy is initialized)
    let proxy_usdc: Option<f64> = match &proxy {
        Some(paddr) => get_usdc_balance(paddr).await.ok(),
        None => None,
    };

    // Accessibility: check_clob_access returns Some(warning) when blocked
    let accessible = access_result.is_none();
    let access_warning = access_result;

    // Silently tolerate RPC errors on balance/positions — quickstart is a status probe,
    // not a trading command; returning 0 + a clear status is better than aborting.
    let eoa_pol = pol_result.unwrap_or(0.0);
    let eoa_usdc = eoa_usdc_result.unwrap_or(0.0);
    let positions: Vec<Position> = positions_result.unwrap_or_default();
    let open_positions_count = positions.len();

    // Brief positions summary (cap at 10 to keep output readable)
    let positions_summary: Vec<_> = positions
        .iter()
        .take(10)
        .map(|p| {
            serde_json::json!({
                "title":             p.title,
                "slug":              p.slug,
                "outcome":           p.outcome,
                "size":              p.size,
                "avg_price":         p.avg_price,
                "cur_price":         p.cur_price,
                "current_value_usd": p.current_value,
                "cash_pnl_usd":      p.cash_pnl,
            })
        })
        .collect();

    // 6. Build state-machine guidance
    let (status, suggestion, onboarding_steps, next_command) = build_suggestion(
        &eoa,
        accessible,
        access_warning.as_deref(),
        proxy.as_deref(),
        eoa_pol,
        eoa_usdc,
        proxy_usdc,
        open_positions_count,
    );

    let mut assets = serde_json::json!({
        "eoa_pol":    format!("{:.4}", eoa_pol),
        "eoa_usdc_e": format!("{:.2}", eoa_usdc),
    });
    if let Some(u) = proxy_usdc {
        assets["proxy_usdc_e"] = serde_json::json!(format!("{:.2}", u));
    }

    let mut out = serde_json::json!({
        "ok":    true,
        "about": ABOUT,
        "wallet": {
            "eoa":   eoa,
            "proxy": proxy,
        },
        "accessible":           accessible,
        "assets":               assets,
        "positions":            positions_summary,
        "open_positions_count": open_positions_count,
        "status":               status,
        "suggestion":           suggestion,
        "next_command":         next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = serde_json::json!(onboarding_steps);
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Default deposit amount: 90% of EOA USDC.e, floored to cents, clamped to [MIN_FUND, eoa_usdc].
fn suggest_deposit(eoa_usdc: f64) -> f64 {
    let raw = (eoa_usdc * 0.9 * 100.0).floor() / 100.0;
    raw.max(MIN_FUND).min(eoa_usdc)
}

/// Returns (status, human-readable suggestion, onboarding_steps, ready-to-run command).
fn build_suggestion(
    eoa: &str,
    accessible: bool,
    access_warning: Option<&str>,
    proxy: Option<&str>,
    eoa_pol: f64,
    eoa_usdc: f64,
    proxy_usdc: Option<f64>,
    open_positions: usize,
) -> (&'static str, String, Vec<String>, String) {
    // Case 1: region-locked — Polymarket blocks US and OFAC jurisdictions
    if !accessible {
        let base = "Polymarket is not accessible from this network. \
                    The CLOB may be blocking your IP (US/OFAC jurisdictions are blocked). \
                    Switch to an accessible region before continuing.";
        let msg = match access_warning {
            Some(w) => format!("{base} Detail: {w}"),
            None => base.to_string(),
        };
        return (
            "restricted",
            msg,
            vec![],
            "polymarket-plugin check-access".to_string(),
        );
    }

    // Case 2: active trader — has open positions on the maker wallet
    if open_positions > 0 {
        return (
            "active",
            format!(
                "You have {} open position(s) on Polymarket. Review them below.",
                open_positions
            ),
            vec![],
            "polymarket-plugin get-positions".to_string(),
        );
    }

    // Case 3: proxy wallet exists (setup-proxy has been run)
    if proxy.is_some() {
        let pu = proxy_usdc.unwrap_or(0.0);

        // 3a. proxy_ready — gasless trading is fully set up
        if pu >= MIN_FUND {
            return (
                "proxy_ready",
                format!(
                    "Proxy wallet is funded (${:.2} USDC.e). Place your first gasless trade.",
                    pu
                ),
                vec![
                    "1. Browse active markets:".to_string(),
                    "   polymarket-plugin list-markets".to_string(),
                    "2. (Optional) Pick a 5-minute crypto up/down market:".to_string(),
                    "   polymarket-plugin list-5m".to_string(),
                    "3. Preview a buy with --dry-run:".to_string(),
                    "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5 --dry-run"
                        .to_string(),
                    "4. When ready, remove --dry-run to submit the order:".to_string(),
                    "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5"
                        .to_string(),
                ],
                "polymarket-plugin list-markets".to_string(),
            );
        }

        // 3b. needs_deposit — proxy exists but under-funded; EOA has enough to deposit
        if eoa_usdc >= MIN_FUND {
            let suggest = suggest_deposit(eoa_usdc);
            return (
                "needs_deposit",
                format!(
                    "Proxy wallet has ${:.2} USDC.e — below the $5 minimum. Deposit from your EOA wallet (${:.2} USDC.e available).",
                    pu, eoa_usdc
                ),
                vec![
                    "1. Deposit USDC.e from EOA into the proxy wallet (gasless):".to_string(),
                    format!("   polymarket-plugin deposit --amount {:.2}", suggest),
                    "2. Re-run quickstart to confirm the proxy is funded:".to_string(),
                    "   polymarket-plugin quickstart".to_string(),
                    "3. Place your first gasless trade:".to_string(),
                    "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5"
                        .to_string(),
                ],
                format!("polymarket-plugin deposit --amount {:.2}", suggest),
            );
        }
        // 3c. proxy exists but neither proxy nor EOA has enough — fall through to low_balance/no_funds
    }

    // Case 4: EOA has enough USDC.e but proxy not set up → guide to setup-proxy (gasless default)
    if eoa_usdc >= MIN_FUND {
        let suggest = suggest_deposit(eoa_usdc);
        let mut steps = vec![
            "1. Create a Polymarket proxy wallet (one-time POL gas):".to_string(),
            "   polymarket-plugin setup-proxy".to_string(),
            "2. Deposit USDC.e from EOA into the proxy wallet:".to_string(),
            format!("   polymarket-plugin deposit --amount {:.2}", suggest),
            "3. Re-run quickstart to confirm the proxy is ready:".to_string(),
            "   polymarket-plugin quickstart".to_string(),
            "4. Place your first gasless trade:".to_string(),
            "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
        ];
        if eoa_pol < MIN_POL_FOR_SETUP {
            steps.insert(
                0,
                format!(
                    "0. First top up POL on your EOA wallet ({} POL needed for setup-proxy gas; current: {:.4} POL). Send POL to:",
                    MIN_POL_FOR_SETUP, eoa_pol
                ),
            );
            steps.insert(1, format!("   {}", eoa));
        }
        return (
            "needs_setup",
            format!(
                "You have ${:.2} USDC.e on your EOA wallet. Recommended: set up a Polymarket proxy wallet for gasless trading.",
                eoa_usdc
            ),
            steps,
            "polymarket-plugin setup-proxy".to_string(),
        );
    }

    // Case 5: low_balance — some USDC.e on EOA but below $5 minimum
    if eoa_usdc > 0.0 {
        return (
            "low_balance",
            format!(
                "You have ${:.2} USDC.e on your EOA wallet — below the $5 minimum for a first deposit. Top up to at least $5.",
                eoa_usdc
            ),
            vec![
                "1. Send at least $5 USDC.e to your EOA wallet on Polygon (chain 137):".to_string(),
                format!("   {}", eoa),
                "2. Re-run quickstart to confirm your balance:".to_string(),
                "   polymarket-plugin quickstart".to_string(),
                "3. Set up the proxy wallet for gasless trading:".to_string(),
                "   polymarket-plugin setup-proxy".to_string(),
            ],
            "polymarket-plugin balance".to_string(),
        );
    }

    // Case 6: no_funds — EOA is empty
    (
        "no_funds",
        "No USDC.e found on your EOA wallet. Send USDC.e to your EOA on Polygon (chain 137) to get started (minimum $5).".to_string(),
        vec![
            "1. Send USDC.e to your EOA wallet on Polygon (chain 137) — minimum $5:".to_string(),
            format!("   {}", eoa),
            "2. Re-run quickstart to confirm your balance:".to_string(),
            "   polymarket-plugin quickstart".to_string(),
            "3. Set up the proxy wallet for gasless trading:".to_string(),
            "   polymarket-plugin setup-proxy".to_string(),
            "4. Deposit USDC.e to the proxy, then place your first trade:".to_string(),
            "   polymarket-plugin deposit --amount 5".to_string(),
            "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
        ],
        "polymarket-plugin balance".to_string(),
    )
}
