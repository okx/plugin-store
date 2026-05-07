use clap::Args;
use serde_json::{json, Value};

use crate::api::info_post;
use crate::config::{exchange_url, info_url, now_ms, ARBITRUM_CHAIN_ID, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::submit_exchange_request;

/// Query or change Hyperliquid's abstraction mode for cross-DEX margin.
///
/// Background: Hyperliquid's HIP-3 builder DEXs each have their own clearinghouse
/// by default — your USDC on the default DEX is NOT shared with `xyz`, `flx`,
/// etc. The `dex-transfer` command moves USDC between them via `sendAsset`.
///
/// However, HL provides an OPT-IN abstraction mode that pools margin across
/// DEXs at the protocol level. When enabled, you don't need explicit
/// dex-transfer — opening a position on `xyz:CL` automatically draws from a
/// unified pool. This is what the HL web UI uses internally to make HIP-3
/// trading feel "seamless".
///
/// Modes:
///   - `disabled` (default): per-DEX clearinghouse isolation; explicit
///     dex-transfer required to fund a builder DEX before trading on it.
///     Read-side may report this as `"default"`.
///   - `unified`: single shared margin pool across all perp DEXs (default
///     + builder). One liquidation event can affect positions across DEXs.
///   - `portfolio`: shared margin with portfolio-level netting. Hedging
///     positions across DEXs reduces effective margin requirement.
///
/// Examples:
///   # Show current mode
///   hyperliquid-plugin abstraction
///
///   # Enable unified margin (one big pool)
///   hyperliquid-plugin abstraction --set unified --confirm
///
///   # Disable abstraction (back to per-DEX isolation)
///   hyperliquid-plugin abstraction --set disabled --confirm
#[derive(Args)]
pub struct AbstractionArgs {
    /// Set the abstraction mode. Without this, just queries and prints the
    /// current value. Valid: `disabled` / `unified` / `portfolio`.
    #[arg(long, value_parser = ["disabled", "unified", "portfolio"])]
    pub set: Option<String>,

    /// Show payload without signing or submitting (only relevant with --set).
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (only relevant with --set; without it, --set just previews).
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: AbstractionArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run `onchainos wallet addresses` to verify login.",
                )
            );
            return Ok(());
        }
    };

    // Always read current state first — useful both for query-only and as a
    // pre-check before set (so we can show diff and avoid no-op signing).
    let current = match info_post(info, json!({"type": "userAbstraction", "user": wallet})).await {
        Ok(v) => v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string()),
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("userAbstraction fetch failed: {:#}", e),
                    "API_ERROR",
                    "Hyperliquid info endpoint may be limited; retry shortly.",
                )
            );
            return Ok(());
        }
    };

    // Query-only mode
    let target = match &args.set {
        Some(t) => t.clone(),
        None => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "wallet": wallet,
                    "current_mode": current,
                    "note": "HIP-3 trade-without-transfer requires --set unified or --set portfolio.",
                    "modes": {
                        "disabled":  "Per-DEX clearinghouse isolation; dex-transfer needed to fund builder DEXs (also reported as 'default').",
                        "unified":   "Single shared margin pool across all perp DEXs.",
                        "portfolio": "Shared margin with portfolio-level netting (hedge offsets reduce required margin).",
                    },
                }))?
            );
            return Ok(());
        }
    };

    // Set mode
    if current.eq_ignore_ascii_case(&target) {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "action": "abstraction_noop",
                "wallet": wallet,
                "current_mode": current,
                "target_mode": target,
                "note": "Current mode already equals target; nothing to sign.",
            }))?
        );
        return Ok(());
    }

    let nonce = now_ms();
    let action = json!({
        "type": "userSetAbstraction",
        "mode": target,
    });

    let preview = json!({
        "ok": true,
        "stage": if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" },
        "preview": {
            "action": "userSetAbstraction",
            "wallet": wallet,
            "current_mode": current,
            "target_mode": target,
            "warning": match target.as_str() {
                "unified" | "portfolio" => "Enabling cross-DEX margin sharing — one liquidation can cascade across all perp DEXs you're trading. Consider risk implications before confirming.",
                "disabled" => "Disabling abstraction — future trades on builder DEXs will require explicit dex-transfer to fund them.",
                _ => "",
            },
            "nonce": nonce,
        },
    });

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[DRY RUN] No action signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[PREVIEW] Add --confirm to submit the abstraction-mode change.");
        return Ok(());
    }

    let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "SIGNING_FAILED",
                    "Retry the command. If the issue persists, check `onchainos wallet status`.",
                )
            );
            return Ok(());
        }
    };
    eprintln!("[abstraction] Submitting userSetAbstraction({})...", target);
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "TX_SUBMIT_FAILED",
                    "Retry the command.",
                )
            );
            return Ok(());
        }
    };

    let status = result["status"].as_str().unwrap_or("");
    if status != "ok" {
        println!(
            "{}",
            super::error_response(
                &format!("Hyperliquid rejected userSetAbstraction: {}", serde_json::to_string(&result).unwrap_or_default()),
                "TX_REJECTED",
                "Check `result.response`. Possible: invalid mode value, wallet has open positions that block mode change.",
            )
        );
        return Ok(());
    }

    // Re-query to verify the new value sticks
    let after: Value = info_post(info, json!({"type": "userAbstraction", "user": wallet}))
        .await
        .unwrap_or(Value::Null);
    let after_mode = after.as_str().map(|s| s.to_string()).unwrap_or_else(|| after.to_string());

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "action": "userSetAbstraction",
            "wallet": wallet,
            "previous_mode": current,
            "new_mode": after_mode,
            "result": result,
            "tip": match target.as_str() {
                "unified" | "portfolio" => "You can now trade on builder DEXs (e.g. xyz:CL) without dex-transfer; margin is drawn from the unified pool automatically.",
                "disabled" => "Builder DEX trading now requires explicit `dex-transfer` to fund the target DEX first.",
                _ => "",
            },
        }))?
    );
    Ok(())
}
