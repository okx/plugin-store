use clap::Args;
use reqwest::Client;

use crate::api::{check_clob_access, get_positions, Position};
use crate::onchainos::{get_existing_proxy, get_pol_balance, get_usdc_balance, get_wallet_address};

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
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("quickstart"), None)); Ok(()) }
    }
}

async fn run_inner(args: QuickstartArgs) -> anyhow::Result<()> {
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

    // 2. Read local creds for this specific wallet address.
    //    load_credentials_for only returns an entry when signing_address matches exactly,
    //    so account-switching (different address) automatically returns None and triggers
    //    fresh on-chain detection — no manual creds.json clear required.
    let saved_creds = crate::config::load_credentials_for(&eoa).ok().flatten();
    let saved_mode  = saved_creds.as_ref().map(|c| c.mode.clone());

    let proxy_from_creds: Option<String> = saved_creds.as_ref().and_then(|c| c.proxy_wallet.clone());
    let deposit_wallet_from_creds: Option<String> = saved_creds.as_ref().and_then(|c| c.deposit_wallet.clone());

    // On-chain fallback only when creds.json has no mode set (fresh install or wiped creds).
    let proxy: Option<String> = match proxy_from_creds {
        Some(p) => Some(p),
        None if saved_mode.is_none() => get_existing_proxy(&eoa).await.ok().flatten()
            .filter(|(_, exists)| *exists)
            .map(|(addr, _)| addr),
        None => None,
    };
    let deposit_wallet: Option<String> = match deposit_wallet_from_creds {
        Some(d) => Some(d),
        None if saved_mode.is_none() && proxy.is_none() =>
            crate::onchainos::get_existing_deposit_wallet(&eoa).await,
        None => None,
    };

    // 3. Positions belong to the maker wallet — deposit wallet > proxy > EOA
    let primary_wallet = deposit_wallet.clone()
        .or_else(|| proxy.clone())
        .unwrap_or_else(|| eoa.clone());

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

    // 5b. Optional: deposit wallet pUSD balance (only if deposit wallet initialized)
    let deposit_wallet_pusd: Option<f64> = match &deposit_wallet {
        Some(dw) => crate::onchainos::get_pusd_balance(dw).await.ok(),
        None => None,
    };

    // 6. Build state-machine guidance
    let (status, suggestion, onboarding_steps, next_command) = build_suggestion(
        &eoa,
        accessible,
        access_warning.as_deref(),
        proxy.as_deref(),
        deposit_wallet.as_deref(),
        eoa_pol,
        eoa_usdc,
        proxy_usdc,
        deposit_wallet_pusd,
        open_positions_count,
    );

    let mut assets = serde_json::json!({
        "eoa_pol":    format!("{:.4}", eoa_pol),
        "eoa_usdc_e": format!("{:.2}", eoa_usdc),
    });
    if let Some(u) = proxy_usdc {
        assets["proxy_usdc_e"] = serde_json::json!(format!("{:.2}", u));
    }
    if let Some(u) = deposit_wallet_pusd {
        assets["deposit_wallet_pusd"] = serde_json::json!(format!("{:.2}", u));
    }

    let mut out = serde_json::json!({
        "ok":    true,
        "about": ABOUT,
        "wallet": {
            "eoa":            eoa,
            "proxy":          proxy,
            "deposit_wallet": deposit_wallet,
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
    deposit_wallet: Option<&str>,
    eoa_pol: f64,
    eoa_usdc: f64,
    proxy_usdc: Option<f64>,
    deposit_wallet_pusd: Option<f64>,
    open_positions: usize,
) -> (&'static str, String, Vec<String>, String) {
    // Case 1: region-locked
    if !accessible {
        let base = "Polymarket is not accessible from this network. \
                    The CLOB may be blocking your IP (US/OFAC jurisdictions are blocked). \
                    Switch to an accessible region before continuing.";
        let msg = match access_warning {
            Some(w) => format!("{base} Detail: {w}"),
            None => base.to_string(),
        };
        return ("restricted", msg, vec![], "polymarket-plugin check-access".to_string());
    }

    // Case 2: active trader — has open positions on the maker wallet
    if open_positions > 0 {
        return (
            "active",
            format!("You have {} open position(s) on Polymarket. Review them below.", open_positions),
            vec![],
            "polymarket-plugin get-positions".to_string(),
        );
    }

    // Case 3: deposit wallet exists (new-user migration path)
    if deposit_wallet.is_some() {
        let dw_addr = deposit_wallet.unwrap();
        let dw_pusd = deposit_wallet_pusd.unwrap_or(0.0);

        // 3a. deposit_wallet_ready — funded and ready to trade
        if dw_pusd >= MIN_FUND {
            return (
                "deposit_wallet_ready",
                format!("Deposit wallet is funded (${:.2} pUSD). Place your first gasless trade.", dw_pusd),
                vec![
                    "1. Browse active markets:".to_string(),
                    "   polymarket-plugin list-markets".to_string(),
                    "2. Place your first trade (gasless — relayer pays gas):".to_string(),
                    "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
                ],
                "polymarket-plugin list-markets".to_string(),
            );
        }

        // 3b. deposit_wallet_needs_funding — deployed but empty
        return (
            "deposit_wallet_needs_funding",
            format!(
                "Deposit wallet deployed ({}). Send pUSD or USDC.e to it on Polygon (chain 137) to start trading.",
                &dw_addr[..std::cmp::min(20, dw_addr.len())]
            ),
            vec![
                "1. Send pUSD or USDC.e to your deposit wallet on Polygon (chain 137):".to_string(),
                format!("   {}", dw_addr),
                "2. Re-run quickstart to confirm your balance:".to_string(),
                "   polymarket-plugin quickstart".to_string(),
                "3. Place your first gasless trade:".to_string(),
                "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
            ],
            "polymarket-plugin balance".to_string(),
        );
    }

    // Case 4: proxy wallet exists (existing POLY_PROXY users)
    if proxy.is_some() {
        let pu = proxy_usdc.unwrap_or(0.0);

        // 4a. proxy_ready — gasless trading fully set up
        if pu >= MIN_FUND {
            return (
                "proxy_ready",
                format!("Proxy wallet is funded (${:.2} USDC.e). Place your first gasless trade.", pu),
                vec![
                    "1. Browse active markets:".to_string(),
                    "   polymarket-plugin list-markets".to_string(),
                    "2. (Optional) Pick a 5-minute crypto up/down market:".to_string(),
                    "   polymarket-plugin list-5m".to_string(),
                    "3. Preview a buy with --dry-run:".to_string(),
                    "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5 --dry-run".to_string(),
                    "4. When ready, remove --dry-run to submit the order:".to_string(),
                    "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
                ],
                "polymarket-plugin list-markets".to_string(),
            );
        }

        // 4b. needs_deposit — proxy exists but under-funded; EOA has enough
        if eoa_usdc >= MIN_FUND {
            let suggest = suggest_deposit(eoa_usdc);
            return (
                "needs_deposit",
                format!(
                    "Proxy wallet has ${:.2} USDC.e — below the $5 minimum. Deposit from your EOA wallet (${:.2} available).",
                    pu, eoa_usdc
                ),
                vec![
                    "1. Deposit USDC.e from EOA into the proxy wallet:".to_string(),
                    format!("   polymarket-plugin deposit --amount {:.2}", suggest),
                    "2. Re-run quickstart to confirm the proxy is funded:".to_string(),
                    "   polymarket-plugin quickstart".to_string(),
                    "3. Place your first gasless trade:".to_string(),
                    "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
                ],
                format!("polymarket-plugin deposit --amount {:.2}", suggest),
            );
        }
        // 4c. proxy exists but neither proxy nor EOA has enough — fall through
    }

    // Case 5: no wallet setup yet — new user
    // Route to setup-deposit-wallet (gasless, relayer-paid, no POL required).
    // setup-deposit-wallet is safe to run with zero balance — deployment is free.
    let ready_to_setup = proxy.is_none() && deposit_wallet.is_none();

    if ready_to_setup {
        let mut steps = vec![
            "1. Deploy your deposit wallet (gasless — no POL needed):".to_string(),
            "   polymarket-plugin setup-deposit-wallet".to_string(),
            "2. Send pUSD or USDC.e to your deposit wallet on Polygon (chain 137):".to_string(),
            "   (address shown after step 1)".to_string(),
            "3. Re-run quickstart to confirm your balance:".to_string(),
            "   polymarket-plugin quickstart".to_string(),
            "4. Place your first gasless trade:".to_string(),
            "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
        ];
        // If EOA has funds: also mention they can skip deposit wallet and use EOA directly
        if eoa_usdc >= MIN_FUND {
            steps.push("   -- Or trade directly from EOA (requires POL for gas):".to_string());
            steps.push("   polymarket-plugin switch-mode --mode eoa".to_string());
        }
        let summary = if eoa_usdc >= MIN_FUND {
            format!(
                "You have ${:.2} USDC.e on your EOA. Set up a deposit wallet for gasless trading (recommended), or switch to EOA mode to trade directly.",
                eoa_usdc
            )
        } else {
            "New user: deploy your deposit wallet (free, relayer-paid), then fund it with pUSD or USDC.e on Polygon to start trading.".to_string()
        };
        return (
            "needs_deposit_wallet_setup",
            summary,
            steps,
            "polymarket-plugin setup-deposit-wallet".to_string(),
        );
    }

    // Case 6: low_balance — some USDC.e but below minimum
    if eoa_usdc > 0.0 {
        return (
            "low_balance",
            format!(
                "You have ${:.2} USDC.e — below the $5 minimum. Top up to at least $5.",
                eoa_usdc
            ),
            vec![
                "1. Send at least $5 USDC.e or pUSD to your wallet on Polygon (chain 137):".to_string(),
                format!("   {}", eoa),
                "2. Re-run quickstart to confirm your balance:".to_string(),
                "   polymarket-plugin quickstart".to_string(),
                "3. Deploy your deposit wallet (gasless):".to_string(),
                "   polymarket-plugin setup-deposit-wallet".to_string(),
            ],
            "polymarket-plugin balance".to_string(),
        );
    }

    // Case 7: no_funds — EOA is empty, no wallet set up
    (
        "no_funds",
        "New to Polymarket? Deploy your deposit wallet first (free, relayer-paid), then fund it with pUSD or USDC.e on Polygon.".to_string(),
        vec![
            "1. Deploy your deposit wallet (gasless — no POL or funds needed):".to_string(),
            "   polymarket-plugin setup-deposit-wallet".to_string(),
            "2. Send pUSD or USDC.e to your deposit wallet on Polygon (chain 137):".to_string(),
            "   (address shown after step 1)".to_string(),
            "3. Re-run quickstart to confirm your balance:".to_string(),
            "   polymarket-plugin quickstart".to_string(),
            "4. Place your first gasless trade:".to_string(),
            "   polymarket-plugin buy --market-id <SLUG> --outcome yes --amount 5".to_string(),
        ],
        "polymarket-plugin setup-deposit-wallet".to_string(),
    )
}
