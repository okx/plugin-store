/// `pancakeswap-v3 quickstart` — onboarding status and suggested first command.

use anyhow::Result;

const ABOUT: &str = "PancakeSwap V3 is the leading DEX on BNB Chain — swap tokens and provide \
    concentrated liquidity across BNB Chain, Base, Arbitrum, Ethereum, and Linea \
    with industry-low fees and deep liquidity.";

// BSC token addresses (default chain)
const USDT_BSC: &str = "0x55d398326f99059fF775485246999027B3197955"; // 18 dec
const USDC_BSC: &str = "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"; // 18 dec

// Minimum thresholds
const MIN_BNB_GAS_WEI: u128 = 2_000_000_000_000_000; // 0.002 BNB — covers a swap tx
const MIN_TOKEN_RAW: u128 = 5_000_000_000_000_000_000; // 5 USDT/USDC (18 dec)

pub async fn run(wallet_override: Option<&str>) -> Result<()> {
    let wallet = match wallet_override {
        Some(w) => w.to_string(),
        None => crate::onchainos::get_wallet_address().await?,
    };

    eprintln!("Checking assets for {}...", &wallet[..wallet.len().min(10)]);

    let cfg = crate::config::get_chain_config(56)?; // BSC default

    // Fetch in parallel: native BNB, USDT, USDC, LP position count
    let (bnb_result, usdt_result, usdc_result, lp_result) = tokio::join!(
        crate::rpc::get_native_balance(&wallet, cfg.rpc_url),
        crate::rpc::get_balance(USDT_BSC, &wallet, cfg.rpc_url),
        crate::rpc::get_balance(USDC_BSC, &wallet, cfg.rpc_url),
        crate::rpc::get_lp_position_count(cfg.npm, &wallet, cfg.rpc_url),
    );

    let bnb_wei = bnb_result.unwrap_or(0);
    let usdt_raw = usdt_result.unwrap_or(0);
    let usdc_raw = usdc_result.unwrap_or(0);
    let lp_count = lp_result.unwrap_or(0);

    let bnb = bnb_wei as f64 / 1e18;
    let usdt = usdt_raw as f64 / 1e18;
    let usdc = usdc_raw as f64 / 1e18;
    let token_usd = usdt + usdc;

    let (status, suggestion, onboarding_steps, next_command) =
        build_suggestion(&wallet, bnb_wei, token_usd, lp_count);

    let mut out = serde_json::json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "assets": {
            "bnb_balance":       format!("{:.6}", bnb),
            "usdt_balance":      format!("{:.4}", usdt),
            "usdc_balance":      format!("{:.4}", usdc),
            "lp_positions_bsc":  lp_count,
        },
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

fn build_suggestion(
    wallet: &str,
    bnb_wei: u128,
    token_usd: f64,
    lp_count: usize,
) -> (&'static str, &'static str, Vec<String>, String) {
    // Case 1: active LP — has positions on BSC
    if lp_count > 0 {
        return (
            "active",
            "You have active V3 LP positions on BNB Chain.",
            vec![],
            format!("pancakeswap-v3 positions --owner {} --chain 56", wallet),
        );
    }

    // Case 2: ready — has gas + tokens
    if bnb_wei >= MIN_BNB_GAS_WEI && token_usd >= 5.0 {
        let swap_amount = (token_usd * 0.9 * 100.0).floor() / 100.0;
        return (
            "ready",
            "Your wallet is funded. You can swap tokens or add liquidity on BNB Chain.",
            vec![
                "1. Check available pools for USDT/USDC:".to_string(),
                "   pancakeswap-v3 pools --token0 USDT --token1 USDC --chain 56".to_string(),
                "2. Preview a swap (no --confirm = preview only):".to_string(),
                format!("   pancakeswap-v3 swap --from USDT --to WBNB --amount {:.2} --chain 56", swap_amount.min(token_usd)),
                "3. Add --confirm to execute:".to_string(),
                format!("   pancakeswap-v3 swap --from USDT --to WBNB --amount {:.2} --chain 56 --confirm", swap_amount.min(token_usd)),
            ],
            "pancakeswap-v3 pools --token0 USDT --token1 USDC --chain 56".to_string(),
        );
    }

    // Case 3: has tokens but no gas
    if token_usd >= 5.0 {
        return (
            "needs_gas",
            "You have tokens but need BNB for gas fees. Send at least 0.002 BNB to your BSC wallet.",
            vec![
                "1. Send at least 0.002 BNB to your BSC wallet:".to_string(),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm:".to_string(),
                "   pancakeswap-v3 quickstart".to_string(),
            ],
            "pancakeswap-v3 quickstart".to_string(),
        );
    }

    // Case 4: has gas but no tokens
    if bnb_wei >= MIN_BNB_GAS_WEI {
        return (
            "needs_funds",
            "You have BNB for gas but need tokens to swap or add liquidity. Send at least 5 USDT or USDC to your BSC wallet.",
            vec![
                "1. Send at least 5 USDT or USDC to your BSC wallet:".to_string(),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm:".to_string(),
                "   pancakeswap-v3 quickstart".to_string(),
                "3. Then swap or add liquidity:".to_string(),
                "   pancakeswap-v3 swap --from USDT --to WBNB --amount 5 --chain 56 --confirm".to_string(),
            ],
            "pancakeswap-v3 quickstart".to_string(),
        );
    }

    // Case 5: no funds at all
    (
        "no_funds",
        "No BNB or tokens found. Send BNB (for gas) and USDT/USDC to your BSC wallet to get started.",
        vec![
            "1. Send BNB (at least 0.002) and USDT/USDC (at least 5) to your BSC wallet:".to_string(),
            format!("   {}", wallet),
            "2. Run quickstart again to confirm:".to_string(),
            "   pancakeswap-v3 quickstart".to_string(),
            "3. Preview a swap:".to_string(),
            "   pancakeswap-v3 swap --from USDT --to WBNB --amount 5 --chain 56".to_string(),
            "4. Execute with --confirm when ready.".to_string(),
        ],
        "pancakeswap-v3 quickstart".to_string(),
    )
}
