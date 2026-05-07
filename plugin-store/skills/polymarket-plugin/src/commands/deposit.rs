/// `polymarket deposit` — fund the proxy wallet.
///
/// ## Default path (Polygon, direct)
/// Sends an ERC-20 USDC.e transfer from the onchainos EOA wallet directly to
/// the proxy wallet on Polygon (chain 137). No bridge involved.
///
/// ## Bridge path (other EVM chains)
/// For chains where onchainos can sign (ETH/ARB/BASE/OP/BNB), the command:
///   1. Gets a bridge deposit address (POST /deposit with proxy wallet)
///   2. Sends tokens from the EOA to the bridge deposit address via onchainos
///   3. Polls bridge status until COMPLETED
///
/// ## Manual path (chains onchainos cannot sign for)
/// For chains like BTC, Tron, Abstract, etc.:
///   1. Gets a bridge deposit address
///   2. Displays it for the user to send manually
///   3. Polls bridge status until COMPLETED
///
/// ## List mode
/// `--list` fetches GET /supported-assets and prints all chains + tokens.
///
/// Prerequisites: `polymarket setup-proxy` must have been run first.

use anyhow::{bail, Result};
use reqwest::Client;
use std::collections::HashMap;

/// EVM chain IDs that onchainos can send transactions on (besides Polygon=137).
/// Polygon is handled separately (direct transfer, no bridge).
const BRIDGE_ONCHAINOS_CHAIN_IDS: &[&str] = &[
    "1",     // Ethereum
    "42161", // Arbitrum
    "8453",  // Base
    "10",    // Optimism
    "56",    // BNB Chain
];

/// Map common user-input chain names to the chainId used by the bridge API.
fn resolve_chain_id(chain: &str) -> Option<&'static str> {
    match chain.to_lowercase().as_str() {
        "polygon" | "matic" | "137" => Some("137"),
        "ethereum" | "eth" | "1" => Some("1"),
        "arbitrum" | "arb" | "42161" => Some("42161"),
        "base" | "8453" => Some("8453"),
        "optimism" | "op" | "10" => Some("10"),
        "bnb" | "bsc" | "56" => Some("56"),

        "bitcoin" | "btc" => Some("btc"),
        "tron" | "trx" => Some("tron"),
        "solana" | "sol" => Some("sol"),
        _ => None,
    }
}

/// Map bridge chainId → onchainos chain argument (name or numeric ID).
fn onchainos_chain_arg(chain_id: &str) -> &str {
    match chain_id {
        "1" => "ethereum",
        "42161" => "arbitrum",
        "8453" => "base",
        "10" => "optimism",
        "56" => "bnb",

        other => other,
    }
}

pub async fn run(
    amount: Option<&str>,
    chain: &str,
    token: &str,
    list: bool,
    dry_run: bool,
) -> Result<()> {
    match run_inner(amount, chain, token, list, dry_run).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("deposit"), None)); Ok(()) }
    }
}

async fn run_inner(
    amount: Option<&str>,
    chain: &str,
    token: &str,
    list: bool,
    dry_run: bool,
) -> Result<()> {
    let client = Client::new();

    // ── --list mode ─────────────────────────────────────────────────────────
    if list {
        let assets = crate::api::bridge_supported_assets(&client).await?;
        // Group by chain
        let mut by_chain: HashMap<String, Vec<&crate::api::BridgeAsset>> = HashMap::new();
        for a in &assets {
            by_chain
                .entry(format!("{} (chainId: {})", a.chain_name, a.chain_id))
                .or_default()
                .push(a);
        }
        let mut chains: Vec<_> = by_chain.keys().collect();
        chains.sort();
        let mut out = serde_json::json!({ "ok": true, "data": [] });
        let arr = out["data"].as_array_mut().unwrap();
        for chain_key in chains {
            let tokens = &by_chain[chain_key];
            let token_list: Vec<_> = tokens
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "symbol": a.token.symbol,
                        "name": a.token.name,
                        "minUsd": a.min_checkout_usd,
                        "decimals": a.token.decimals,
                        "address": a.token.address,
                    })
                })
                .collect();
            arr.push(serde_json::json!({
                "chain": chain_key,
                "tokens": token_list,
            }));
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // ── Normal deposit — amount required ────────────────────────────────────
    let signer_addr = crate::onchainos::get_wallet_address().await?;
    let creds = crate::auth::ensure_credentials(&client, &signer_addr).await?;

    // Resolve the destination wallet: deposit wallet (POLY_1271) or proxy (POLY_PROXY).
    // In DEPOSIT_WALLET mode, funds go directly to the deposit wallet address.
    // Note: the deposit wallet uses pUSD as collateral. USDC.e deposited here will need
    // to be wrapped to pUSD before trading — the `buy` command auto-wraps if needed.
    let (dest_wallet, dest_label) = if creds.mode == crate::config::TradingMode::DepositWallet {
        let dw = creds.deposit_wallet.as_ref().ok_or_else(|| {
            anyhow::anyhow!("DEPOSIT_WALLET mode set but no deposit wallet address found. Run `polymarket setup-deposit-wallet`.")
        })?.clone();
        (dw, "deposit wallet")
    } else {
        let proxy = creds.proxy_wallet.as_ref().ok_or_else(|| {
            anyhow::anyhow!("No proxy wallet configured. Run `polymarket setup-proxy` first.")
        })?.clone();
        (proxy, "proxy wallet")
    };
    // Alias for backward compat with the rest of the function.
    let proxy_wallet = &dest_wallet;

    // If amount is missing: run smart suggestion flow instead of plain error.
    if amount.is_none() {
        suggest_deposit(&client, &signer_addr).await?;
        return Ok(());
    }

    let amount_str = amount.unwrap();
    let amount_f: f64 = amount_str
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid amount: {}", amount_str))?;
    if amount_f <= 0.0 {
        bail!("amount must be positive");
    }

    // ── Resolve chain ────────────────────────────────────────────────────────
    let chain_id = resolve_chain_id(chain).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown chain '{}'. Use --list to see supported chains, or try: polygon, ethereum, arbitrum, base, optimism, bnb",
            chain
        )
    })?;

    // ── Polygon: direct ERC-20 transfer (no bridge) ─────────────────────────
    // --amount is USD. For USDC.e (6 decimals, 1:1 USD), amount_raw = amount_f × 1e6.
    if chain_id == "137" {
        // Only USDC.e is supported for direct deposit
        let token_upper = token.to_uppercase();
        if !matches!(token_upper.as_str(), "USDC.E" | "USDC" | "USDCE") {
            bail!(
                "Direct Polygon deposit only supports USDC.e. \
                 Use --chain ethereum (or arbitrum/base/op/bnb) to deposit other tokens via bridge."
            );
        }
        let amount_raw = (amount_f * 1_000_000.0).round() as u128;

        // ── Gas pre-flight: check EOA has enough POL to pay for the ERC-20 transfer.
        // Estimate dynamically from current Polygon gas price × 65,000 gas × 1.2 buffer.
        let (pol_balance, min_pol_for_gas) = tokio::join!(
            async { crate::onchainos::get_pol_balance(&signer_addr).await.unwrap_or(0.0) },
            crate::onchainos::estimate_erc20_gas_cost("polygon"),
        );
        if pol_balance < min_pol_for_gas {
            let decimals = if min_pol_for_gas < 0.0001 { 8 } else if min_pol_for_gas < 0.01 { 6 } else { 4 };
            bail!(
                "Insufficient POL for gas. Your wallet {} has {:.6} POL but ~{:.prec$} POL \
                 is needed (current gas price × 65,000 gas + 20% buffer).\n\
                 Add POL to your wallet first (e.g. bridge some MATIC/POL from Ethereum), \
                 then retry.",
                signer_addr, pol_balance, min_pol_for_gas,
                prec = decimals,
            );
        }

        if dry_run {
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "dry_run": true,
                    "data": {
                        "chain": "polygon",
                        "from": signer_addr,
                        "to": proxy_wallet,
                        "destination_type": dest_label,
                        "token": "USDC.e",
                        "amount": amount_f,
                        "amount_raw": amount_raw,
                        "pol_balance": (pol_balance * 1e6).round() / 1e6,
                        "note": "dry-run: no transaction submitted"
                    }
                })
            );
            return Ok(());
        }

        eprintln!(
            "[polymarket] Transferring {} USDC.e to proxy wallet {} on Polygon...",
            amount_f, proxy_wallet
        );
        let tx_hash = crate::onchainos::transfer_usdc_to_proxy(proxy_wallet, amount_raw).await?;
        eprintln!("[polymarket] tx submitted: {}. Waiting for on-chain confirmation...", tx_hash);

        // Wait for the tx to be mined (Polygon ~2s blocks, up to 60s).
        // This prevents returning a success response for a tx that never lands on-chain.
        crate::onchainos::wait_for_tx_receipt(&tx_hash, 60).await
            .map_err(|e| anyhow::anyhow!("Transfer tx {} failed to confirm: {}", tx_hash, e))?;

        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "data": {
                    "chain": "polygon",
                    "tx_hash": tx_hash,
                    "from": signer_addr,
                    "to": proxy_wallet,
                    "token": "USDC.e",
                    "amount": amount_f,
                    "destination_type": dest_label,
                        "note": format!("USDC.e deposited to {} ({}).", dest_label, proxy_wallet)
                }
            })
        );
        return Ok(());
    }

    // ── Non-Polygon: bridge path ─────────────────────────────────────────────
    // Fetch supported assets to find token contract + validate chain/token combo
    let assets = crate::api::bridge_supported_assets(&client).await?;
    let token_upper = token.to_uppercase();

    // Find matching asset for this chain + token
    let asset = assets.iter().find(|a| {
        a.chain_id == chain_id
            && (a.token.symbol.to_uppercase() == token_upper
                || a.token.name.to_uppercase() == token_upper)
    });

    let asset = match asset {
        Some(a) => a,
        None => {
            // Show what IS available on this chain
            let available: Vec<_> = assets
                .iter()
                .filter(|a| a.chain_id == chain_id)
                .map(|a| a.token.symbol.as_str())
                .collect();
            if available.is_empty() {
                bail!(
                    "Chain '{}' (id: {}) is not supported by the bridge. \
                     Use `polymarket deposit --list` to see all supported chains.",
                    chain, chain_id
                );
            } else {
                bail!(
                    "Token '{}' not found on chain '{}'. Available tokens: {}",
                    token,
                    chain,
                    available.join(", ")
                );
            }
        }
    };

    // ── USD minimum check (BEFORE any on-chain action) ──────────────────────
    // --amount is always in USD. Convert to token quantity using live price for
    // non-stablecoins. Hard-fail here so the user never loses funds to a
    // below-minimum deposit that the bridge silently ignores.
    let min_usd = asset.min_checkout_usd;

    // Stablecoins: 1 token ≈ $1, no price fetch needed.
    // Stablecoin detection is based on symbol only — BNB-chain stablecoins use 18 decimals
    // (e.g. USDC/USDT/DAI on BSC), so checking decimals <= 6 would incorrectly skip them.
    let is_stablecoin = matches!(
        asset.token.symbol.to_uppercase().as_str(),
        "USDC" | "USDC.E" | "USDCE" | "USDT" | "USDT0"
            | "USD\u{20AE}0" | "DAI" | "BUSD" | "USDP" | "PYUSD"
            | "USDS" | "USDE" | "USDG" | "MUSD" | "USDBC"
            | "EURC" | "EUROC" | "EUR24"
    );

    let token_price_usd: f64 = if is_stablecoin {
        1.0
    } else {
        // Fetch live price — must succeed before we touch the chain.
        eprintln!("[polymarket] Fetching {} price...", asset.token.symbol);
        match crate::api::get_token_price_usd(&client, chain_id, &asset.token.address).await {
            Some(p) => p,
            None => bail!(
                "Could not fetch USD price for {} on {}. \
                 Try depositing a stablecoin (USDC, USDT) instead, or retry.",
                asset.token.symbol, asset.chain_name
            ),
        }
    };

    // amount_f is USD → enforce minimum before going any further.
    if amount_f < min_usd {
        bail!(
            "Amount ${:.2} is below the bridge minimum ${:.2} for {} on {}. \
             Please deposit at least ${:.0}.",
            amount_f, min_usd, asset.token.symbol, asset.chain_name, min_usd
        );
    }

    // Convert USD amount → token raw units.
    let token_qty = amount_f / token_price_usd;
    let amount_raw = (token_qty * 10f64.powi(asset.token.decimals as i32)).round() as u128;
    if amount_raw == 0 {
        bail!("Computed token quantity is 0. Check the amount and token.");
    }
    let can_auto_send = BRIDGE_ONCHAINOS_CHAIN_IDS.contains(&chain_id);

    // Sentinel address means native coin (ETH, BNB, etc.).
    // The bridge backend only monitors ERC-20 Transfer events and will NOT
    // detect a plain native-value transfer to the deposit EOA — funds would
    // be lost. Block this early (before any on-chain action) and direct the
    // user to the wrapped ERC-20 equivalent.
    let is_native = asset.token.address.to_lowercase()
        == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    if is_native {
        let wrapped_sym = format!("W{}", asset.token.symbol.to_uppercase());
        let alt = assets
            .iter()
            .find(|a| a.chain_id == chain_id && a.token.symbol.to_uppercase() == wrapped_sym)
            .map(|a| a.token.symbol.as_str())
            .unwrap_or("USDC");
        bail!(
            "Native {} cannot be deposited directly — the bridge only detects ERC-20 transfers.\n\
             Use the wrapped ERC-20 version instead:\n\
               polymarket deposit --amount {} --chain {} --token {}",
            asset.token.symbol, amount_f, chain, alt
        );
    }

    // Get bridge deposit address
    eprintln!("[polymarket] Getting bridge deposit address for proxy wallet {}...", proxy_wallet);
    let bridge_deposit_addr = crate::api::bridge_get_deposit_address(&client, proxy_wallet).await?;

    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "chain": asset.chain_name,
                    "chain_id": chain_id,
                    "token": asset.token.symbol,
                    "amount_usd": amount_f,
                    "token_qty": token_qty,
                    "token_price_usd": if is_stablecoin { serde_json::Value::Null } else { serde_json::json!(token_price_usd) },
                    "amount_raw": amount_raw,
                    "bridge_deposit_address": bridge_deposit_addr,
                    "from": signer_addr,
                    "auto_send": can_auto_send,
                    "note": "dry-run: no transaction submitted"
                }
            })
        );
        return Ok(());
    }

    let tx_hash: Option<String> = if can_auto_send {
        // ── Gas pre-flight for bridge EVM chains ─────────────────────────────
        // Estimate required gas dynamically: eth_gasPrice × 65,000 gas × 1.2 buffer.
        let oc_chain = onchainos_chain_arg(chain_id);
        let native_sym = match chain_id {
            "56" => "BNB",
            _    => "ETH",
        };
        let (native_bal, min_native_gas) = tokio::join!(
            crate::onchainos::get_native_gas_balance(oc_chain),
            crate::onchainos::estimate_erc20_gas_cost(oc_chain),
        );
        if native_bal < min_native_gas {
            // Use enough decimal places so the needed amount shows at least 2 sig figs.
            let decimals = if min_native_gas < 0.0001 { 8 } else if min_native_gas < 0.01 { 6 } else { 4 };
            bail!(
                "Insufficient {} for gas on {}. Your wallet has {:.6} {} but ~{:.prec$} {} \
                 is needed (current gas price × 65,000 gas + 20% buffer).\n\
                 Add {} to your wallet on {} first, then retry.",
                native_sym, asset.chain_name,
                native_bal, native_sym,
                min_native_gas, native_sym,
                native_sym, asset.chain_name,
                prec = decimals,
            );
        }

        // onchainos can sign on this chain — send automatically
        eprintln!(
            "[polymarket] Sending {} {} on {} → bridge deposit address {}...",
            amount_f, asset.token.symbol, asset.chain_name, bridge_deposit_addr
        );
        let hash = crate::onchainos::transfer_erc20_on_chain(
            oc_chain,
            &asset.token.address,
            &bridge_deposit_addr,
            amount_raw,
        )
        .await?;
        eprintln!("[polymarket] Sent. tx_hash: {}. Waiting for on-chain confirmation...", hash);

        // Wait for source-chain tx to be mined before handing off to bridge poller.
        // BNB/ETH block times are 3–12s; 120s budget is more than sufficient.
        // (The bridge won't detect the deposit until it's confirmed on-chain anyway.)
        if let Err(e) = crate::onchainos::wait_for_receipt_on_chain(oc_chain, &hash, 120).await {
            bail!("Source-chain tx {} failed to confirm: {}", hash, e);
        }

        eprintln!("[polymarket] Source-chain tx confirmed.");
        Some(hash)
    } else {
        // Manual chain (BTC, Tron, Solana, etc.) — show address, user sends manually
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "data": {
                    "chain": asset.chain_name,
                    "token": asset.token.symbol,
                    "amount": amount_f,
                    "bridge_deposit_address": bridge_deposit_addr,
                    "note": format!(
                        "Send exactly {} {} to the bridge deposit address above, then wait for confirmation.",
                        amount_f, asset.token.symbol
                    )
                }
            })
        );
        None
    };

    // ── Poll bridge status ───────────────────────────────────────────────────
    eprintln!("[polymarket] Waiting for bridge to process deposit...");
    let mut attempts = 0u32;
    let max_attempts = 60; // 5 minutes at 5s interval
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        attempts += 1;

        match crate::api::bridge_poll_status(&client, &bridge_deposit_addr).await {
            Ok(crate::api::BridgeStatus::Completed) => {
                println!(
                    "{}",
                    serde_json::json!({
                        "ok": true,
                        "data": {
                            "status": "COMPLETED",
                            "chain": asset.chain_name,
                            "token": asset.token.symbol,
                            "amount": amount_f,
                            "bridge_deposit_address": bridge_deposit_addr,
                            "tx_hash": tx_hash,
                            "proxy_wallet": proxy_wallet,
                            "note": "Deposit completed. Funds are now in your proxy wallet as USDC."
                        }
                    })
                );
                return Ok(());
            }
            Ok(crate::api::BridgeStatus::Failed) => {
                bail!(
                    "Bridge deposit FAILED. bridge_deposit_address: {}. \
                     Check the bridge status manually.",
                    bridge_deposit_addr
                );
            }
            Ok(crate::api::BridgeStatus::Pending(state)) => {
                eprintln!(
                    "[polymarket] Bridge status: {} (attempt {}/{})",
                    state, attempts, max_attempts
                );
                if attempts >= max_attempts {
                    bail!(
                        "Bridge deposit timed out after {} attempts. Last status: {}. \
                         bridge_deposit_address: {}",
                        max_attempts, state, bridge_deposit_addr
                    );
                }
            }
            Err(e) => {
                eprintln!("[polymarket] Bridge poll error (attempt {}): {}", attempts, e);
                if attempts >= max_attempts {
                    bail!("Bridge poll failed after {} attempts: {}", max_attempts, e);
                }
            }
        }
    }
}

// ── Smart deposit suggestion (called when --amount is omitted) ────────────────
async fn suggest_deposit(client: &reqwest::Client, signer_addr: &str) -> anyhow::Result<()> {
    const BRIDGE_CHAINS: &[(&str, &str, &str)] = &[
        ("ethereum", "1",     "ethereum"),
        ("arbitrum", "42161", "arbitrum"),
        ("base",     "8453",  "base"),
        ("optimism", "10",    "optimism"),
        ("bnb",      "56",    "bnb"),

    ];

    // ── Step 1: Polygon check ────────────────────────────────────────────────
    let (pol_bal, usdc_bal) = tokio::join!(
        crate::onchainos::get_pol_balance(signer_addr),
        crate::onchainos::get_usdc_balance(signer_addr),
    );
    let pol_balance = pol_bal.unwrap_or(0.0);
    let usdc_e_usd = usdc_bal.unwrap_or(0.0);
    let has_polygon_gas = pol_balance >= 0.1;

    let polygon_info = serde_json::json!({
        "usdc_e_usd": (usdc_e_usd * 100.0).round() / 100.0,
        "pol_balance": (pol_balance * 10000.0).round() / 10000.0,
        "has_gas": has_polygon_gas,
        "command": format!("polymarket deposit --amount {:.0} --chain polygon --token USDC",
                           usdc_e_usd.floor().max(1.0)),
        "note": "Direct transfer on Polygon — no bridge fee, instant"
    });

    if usdc_e_usd >= 1.0 && has_polygon_gas {
        let hint = format!(
            "You have ${:.2} USDC.e on Polygon (recommended). \
             Transfer more USDC.e to {} on Polygon if needed, then specify --amount.",
            usdc_e_usd, signer_addr
        );
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "missing_params": ["amount"],
            "deposit_suggestions": {
                "eoa_address": signer_addr,
                "polygon": polygon_info,
                "alternatives": [],
                "recommended_command": format!(
                    "polymarket deposit --amount {:.0} --chain polygon --token USDC",
                    usdc_e_usd.floor().max(1.0)
                )
            },
            "hint": hint
        }))?);
        return Ok(());
    }

    // ── Step 2: Polygon insufficient — scan bridge chains in parallel ─────────
    eprintln!("[polymarket] Scanning balances across supported chains...");
    let assets = crate::api::bridge_supported_assets(client).await.unwrap_or_default();

    let min_usd_map: std::collections::HashMap<(String, String), f64> = assets
        .iter()
        .map(|a| ((a.chain_id.clone(), a.token.address.to_lowercase()), a.min_checkout_usd))
        .collect();

    let balance_futs: Vec<_> = BRIDGE_CHAINS
        .iter()
        .map(|(oc_chain, _, _)| crate::onchainos::get_chain_balances(oc_chain))
        .collect();
    let all_balances = futures::future::join_all(balance_futs).await;

    let sentinel = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let mut alternatives: Vec<serde_json::Value> = Vec::new();

    for (i, balances) in all_balances.iter().enumerate() {
        let (oc_chain, bridge_chain_id, chain_name) = BRIDGE_CHAINS[i];
        for b in balances {
            if b.usd_value < 1.0 { continue; }
            let tok_addr = if b.token_address.is_empty() { sentinel.to_string() } else { b.token_address.clone() };
            // Skip native tokens (bridge doesn't detect plain value transfers)
            if tok_addr == sentinel { continue; }
            let Some(&min) = min_usd_map.get(&(bridge_chain_id.to_string(), tok_addr)) else { continue };
            if b.usd_value < min { continue; }
            let deposit_amount = (b.usd_value * 0.98).floor().max(min);
            alternatives.push(serde_json::json!({
                "chain": chain_name,
                "token": b.symbol,
                "available_usd": (b.usd_value * 100.0).round() / 100.0,
                "min_deposit_usd": min,
                "command": format!(
                    "polymarket deposit --amount {:.0} --chain {} --token {}",
                    deposit_amount, oc_chain, b.symbol
                )
            }));
        }
    }

    alternatives.sort_by(|a, b| {
        b["available_usd"].as_f64().unwrap_or(0.0)
            .partial_cmp(&a["available_usd"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, alt) in alternatives.iter_mut().enumerate() {
        alt["rank"] = serde_json::json!(i + 1);
    }

    let recommended_command = alternatives
        .first()
        .and_then(|a| a["command"].as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!(
            "Transfer USDC.e to {} on Polygon, then: polymarket deposit --amount <X> --chain polygon --token USDC",
            signer_addr
        ));

    let hint = if let Some(top) = alternatives.first() {
        format!(
            "Recommended: {} (${:.2} available). Run: {}",
            top["token"].as_str().unwrap_or(""),
            top["available_usd"].as_f64().unwrap_or(0.0),
            top["command"].as_str().unwrap_or("")
        )
    } else {
        format!(
            "No eligible assets found. Transfer USDC.e to {} on Polygon (chain 137) first.",
            signer_addr
        )
    };

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "missing_params": ["amount"],
        "deposit_suggestions": {
            "eoa_address": signer_addr,
            "polygon": polygon_info,
            "alternatives": alternatives,
            "recommended_command": recommended_command
        },
        "hint": hint
    }))?);
    Ok(())
}
