use serde_json::json;

const ABOUT: &str = "Morpho is a permissionless lending protocol on Ethereum and Base — \
    supply assets to earn yield in MetaMorpho vaults, or borrow against collateral \
    in Morpho Blue markets with no KYC and $5B+ TVL.";

// USDC addresses per chain
const USDC_ETHEREUM: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDC_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

// WETH addresses per chain
const WETH_ETHEREUM: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const WETH_BASE: &str = "0x4200000000000000000000000000000000000006";

// Minimum ETH needed for approve + main tx
const MIN_GAS_ETHEREUM_WEI: u128 = 3_000_000_000_000_000; // 0.003 ETH (L1)
const MIN_GAS_BASE_WEI: u128 = 200_000_000_000_000;       // 0.0002 ETH (L2)

// Minimum meaningful token balance for "has funds" check
const MIN_USDC_RAW: u128 = 1_000_000;          // 1 USDC (6 decimals)
const MIN_WETH_RAW: u128 = 1_000_000_000_000_000; // 0.001 WETH (18 decimals)

pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain_id)?;
    let wallet = crate::onchainos::resolve_wallet(from, chain_id).await?;

    let chain_str = if chain_id == 8453 { "base" } else { "ethereum" };
    eprintln!(
        "Checking assets for {} on {}...",
        &wallet[..std::cmp::min(10, wallet.len())],
        chain_str
    );

    let usdc_addr = if chain_id == 8453 { USDC_BASE } else { USDC_ETHEREUM };
    let weth_addr = if chain_id == 8453 { WETH_BASE } else { WETH_ETHEREUM };
    let min_gas_wei = if chain_id == 8453 { MIN_GAS_BASE_WEI } else { MIN_GAS_ETHEREUM_WEI };

    // Fetch everything in parallel
    let (eth_res, usdc_res, weth_res, blue_res, vault_res) = tokio::join!(
        crate::rpc::eth_balance(&wallet, cfg.rpc_url),
        crate::rpc::erc20_balance_of(usdc_addr, &wallet, cfg.rpc_url),
        crate::rpc::erc20_balance_of(weth_addr, &wallet, cfg.rpc_url),
        crate::api::get_user_positions(&wallet, chain_id),
        crate::api::get_vault_positions(&wallet, chain_id),
    );

    let eth_wei = eth_res.unwrap_or(0);
    let usdc_raw = usdc_res.unwrap_or(0);
    let weth_raw = weth_res.unwrap_or(0);

    let eth_balance = eth_wei as f64 / 1e18;
    let usdc_balance = usdc_raw as f64 / 1_000_000.0;
    let weth_balance = weth_raw as f64 / 1e18;

    // Count non-empty Blue market positions
    let blue_count = blue_res.as_ref()
        .map(|positions| {
            positions.iter().filter(|p| {
                let borrow: u128 = p.state.borrow_assets.as_deref().unwrap_or("0").parse().unwrap_or(0);
                let supply: u128 = p.state.supply_assets.as_deref().unwrap_or("0").parse().unwrap_or(0);
                let coll: u128 = p.state.collateral.as_deref().unwrap_or("0").parse().unwrap_or(0);
                borrow > 0 || supply > 0 || coll > 0
            }).count()
        })
        .unwrap_or(0);

    // Count non-empty vault positions
    let vault_count = vault_res.as_ref()
        .map(|positions| {
            positions.iter().filter(|p| {
                p.assets.as_deref().unwrap_or("0").parse::<u128>().unwrap_or(0) > 0
            }).count()
        })
        .unwrap_or(0);

    let has_positions = blue_count > 0 || vault_count > 0;
    let has_tokens = usdc_raw >= MIN_USDC_RAW || weth_raw >= MIN_WETH_RAW;
    let has_gas = eth_wei >= min_gas_wei;

    let (status, suggestion, onboarding_steps, next_command) = build_suggestion(
        chain_id,
        &wallet,
        eth_balance,
        min_gas_wei,
        usdc_balance,
        weth_balance,
        has_positions,
        has_tokens,
        has_gas,
    );

    let mut out = json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "chain": chain_str,
        "chainId": chain_id,
        "assets": {
            "eth_balance": format!("{:.6}", eth_balance),
            "usdc_balance": format!("{:.2}", usdc_balance),
            "weth_balance": format!("{:.6}", weth_balance),
            "blue_positions": blue_count,
            "vault_positions": vault_count,
        },
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command,
    });

    if !onboarding_steps.is_empty() {
        out["onboarding_steps"] = json!(onboarding_steps);
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn build_suggestion(
    chain_id: u64,
    wallet: &str,
    eth_balance: f64,
    min_gas_wei: u128,
    usdc_balance: f64,
    weth_balance: f64,
    has_positions: bool,
    has_tokens: bool,
    has_gas: bool,
) -> (&'static str, &'static str, Vec<String>, String) {
    let chain_flag = if chain_id == 8453 { "--chain 8453 " } else { "" };
    let min_gas_eth = min_gas_wei as f64 / 1e18;

    // Case 1: active — open positions
    if has_positions {
        return (
            "active",
            "You have open Morpho positions. Review your balances and health factors.",
            vec![],
            format!("morpho-plugin {}positions", chain_flag),
        );
    }

    // Case 2: ready — has gas + tokens
    if has_gas && has_tokens {
        let (asset, example_amount) = if usdc_balance >= 1.0 {
            ("USDC", format!("{:.2}", (usdc_balance * 0.9_f64).max(1.0_f64).min(usdc_balance)))
        } else {
            ("WETH", format!("{:.4}", (weth_balance * 0.9_f64).max(0.001_f64).min(weth_balance)))
        };
        return (
            "ready",
            "Your wallet is funded. Browse MetaMorpho vaults and start earning yield.",
            vec![
                "1. Browse available vaults to earn yield:".to_string(),
                format!("   morpho-plugin {}vaults --asset {}", chain_flag, asset),
                "2. Supply to a vault (ERC-4626 deposit):".to_string(),
                format!("   morpho-plugin {}supply --vault <vault-address> --asset {} --amount {} --confirm",
                    chain_flag, asset, example_amount),
                "3. Or browse Morpho Blue markets to borrow with collateral:".to_string(),
                format!("   morpho-plugin {}markets --asset {}", chain_flag, asset),
            ],
            format!("morpho-plugin {}vaults --asset {}", chain_flag, asset),
        );
    }

    // Case 3: has tokens but no gas
    if has_tokens && !has_gas {
        let asset = if usdc_balance >= 1.0 { "USDC" } else { "WETH" };
        return (
            "needs_gas",
            "You have tokens but need ETH for gas fees. Send ETH to your wallet to proceed.",
            vec![
                format!("1. Send at least {:.4} ETH (gas) to your wallet:", min_gas_eth),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm:".to_string(),
                format!("   morpho-plugin {}quickstart", chain_flag),
                format!("3. Browse {} vaults:", asset),
                format!("   morpho-plugin {}vaults --asset {}", chain_flag, asset),
            ],
            format!("morpho-plugin {}quickstart", chain_flag),
        );
    }

    // Case 4: has gas but no meaningful tokens
    if has_gas && !has_tokens {
        return (
            "needs_funds",
            "You have ETH for gas but no USDC or WETH to supply. Transfer tokens to your wallet.",
            vec![
                "1. Send USDC or WETH to your wallet:".to_string(),
                format!("   {}", wallet),
                "2. Run quickstart again to confirm balance:".to_string(),
                format!("   morpho-plugin {}quickstart", chain_flag),
                "3. Browse MetaMorpho vaults to pick the best yield:".to_string(),
                format!("   morpho-plugin {}vaults --asset USDC", chain_flag),
            ],
            format!("morpho-plugin {}vaults --asset USDC", chain_flag),
        );
    }

    // Case 5: no funds at all
    (
        "no_funds",
        "No ETH or tokens found. Send ETH (for gas) and USDC/WETH to your wallet to get started.",
        vec![
            "1. Send ETH (for gas) and USDC or WETH to your wallet:".to_string(),
            format!("   {}", wallet),
            format!("   Minimum gas: {:.4} ETH", min_gas_eth),
            "2. Run quickstart again to confirm:".to_string(),
            format!("   morpho-plugin {}quickstart", chain_flag),
            "3. Browse MetaMorpho vaults to earn yield:".to_string(),
            format!("   morpho-plugin {}vaults --asset USDC", chain_flag),
            "4. Supply to start earning:".to_string(),
            "   morpho-plugin supply --vault <vault-address> --asset USDC --amount <amount> --confirm".to_string(),
        ],
        format!("morpho-plugin {}quickstart", chain_flag),
    )
}
