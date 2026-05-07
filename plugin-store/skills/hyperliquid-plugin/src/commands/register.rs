// commands/register.rs — Detect and set up the onchainos signing address on Hyperliquid
//
// onchainos uses AA wallets. The EVM address shown by `wallet addresses` may differ
// from the underlying EOA signing key that Hyperliquid recovers from EIP-712 signatures.
//
// This command discovers the actual signing address by submitting a signed test request
// to HL exchange (which returns the recovered signer in the error response), then outputs
// clear instructions for how to fund and set up that address.

use clap::Args;
use serde_json::json;

use crate::api::get_asset_index;
use crate::config::{exchange_url, info_url, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_market_order_action, submit_exchange_request};

#[derive(Args)]
pub struct RegisterArgs {
    /// Skip the signing test; just show wallet address info
    #[arg(long)]
    dry_run: bool,
}

pub async fn run(args: RegisterArgs) -> anyhow::Result<()> {
    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };

    if args.dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "onchainos_wallet": wallet,
                "note": "Run without --dry-run to detect your Hyperliquid signing address via a test signature."
            }))?
        );
        return Ok(());
    }

    eprintln!("Detecting your Hyperliquid signing address via onchainos...");

    // Build a minimal action — 0-size orders are rejected by HL but still reveal
    // the recovered signer address in the error response.
    let nonce = now_ms();
    let asset_idx = get_asset_index(info_url(), "ETH").await.unwrap_or(1);
    // Price "0" is intentionally invalid — we want HL to reject this but reveal the signer.
    let dummy_action = build_market_order_action(asset_idx, true, "0", false, "0");

    // Sign through onchainos (real signing required to get HL to reveal signer)
    let signed = match onchainos_hl_sign(&dummy_action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("onchainos signing failed: {}. Ensure onchainos is installed and a wallet is configured.", e),
                "SIGNING_FAILED",
                "Retry the command. If the issue persists, check onchainos status."
            ));
            return Ok(());
        }
    };

    // Submit to HL — expect an error response containing the recovered signer address
    let response = submit_exchange_request(exchange_url(), signed).await;

    let signer = match &response {
        Ok(v) => extract_0x_address(
            v["response"].as_str().unwrap_or("")
        ),
        Err(e) => extract_0x_address(&e.to_string()),
    };

    let signer = match signer {
        Some(addr) => addr,
        None => {
            // If HL returned ok (shouldn't happen for size=0), signer == wallet
            if response.as_ref().map(|v| v["status"].as_str() == Some("ok")).unwrap_or(false) {
                wallet.clone()
            } else {
                println!("{}", super::error_response(
                    &format!("Could not detect signing address from HL response: {:?}", response),
                    "API_ERROR",
                    "Check your connection and retry."
                ));
                return Ok(());
            }
        }
    };

    let addresses_match = signer.to_lowercase() == wallet.to_lowercase();

    if addresses_match {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "status": "ready",
                "hl_address": signer,
                "message": "Your onchainos wallet address matches your Hyperliquid signing address. No extra setup needed — orders will work once your account has USDC."
            }))?
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "status": "setup_required",
                "onchainos_wallet": wallet,
                "hl_signing_address": signer,
                "explanation": "onchainos uses an AA (account abstraction) wallet. Hyperliquid recovers the underlying EOA signing key, not the AA wallet address. These are two different addresses.",
                "options": {
                    "option_1_recommended": {
                        "description": "Deposit USDC directly to your signing address to create a fresh Hyperliquid account tied to your onchainos signing key.",
                        "command": format!("hyperliquid deposit --amount <USDC_AMOUNT> --to {}", signer),
                        "note": "This keeps everything in onchainos — no web UI required."
                    },
                    "option_2_existing_account": {
                        "description": format!(
                            "If you already have funds at {} on Hyperliquid, register {} as an API wallet via the Hyperliquid web UI.",
                            wallet, signer
                        ),
                        "url": "https://app.hyperliquid.xyz/settings/api-wallets",
                        "steps": [
                            format!("1. Go to https://app.hyperliquid.xyz/settings/api-wallets"),
                            format!("2. Click 'Add API Wallet'"),
                            format!("3. Enter your signing address: {}", signer),
                            "4. Sign with your connected wallet"
                        ]
                    }
                }
            }))?
        );
    }

    Ok(())
}

/// Extract the first `0x` + 40 hex char address from a string.
fn extract_0x_address(s: &str) -> Option<String> {
    let lower = s.to_lowercase();
    let start = lower.find("0x")?;
    let rest = &s[start..];
    let end = rest
        .char_indices()
        .skip(2) // skip "0x"
        .take_while(|(_, c)| c.is_ascii_hexdigit())
        .last()
        .map(|(i, _)| i + 1)
        .unwrap_or(2);
    if end == 42 {
        // Exactly 40 hex chars = valid EVM address
        Some(rest[..42].to_lowercase())
    } else {
        None
    }
}
