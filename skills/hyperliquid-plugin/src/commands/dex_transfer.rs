use clap::Args;
use serde_json::{json, Value};

use crate::api::{
    fetch_perp_dexs, find_dex, get_clearinghouse_state_for_dex, get_meta_for_dex, BuilderDex,
};
use crate::config::{exchange_url, info_url, now_ms, ARBITRUM_CHAIN_ID, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign_send_asset, resolve_wallet};
use crate::signing::submit_exchange_request;

/// HIP-3: Move USDC between perp DEXs (default <-> builder DEX like xyz/flx/etc).
///
/// Each builder DEX has a SEPARATE clearinghouse — your USDC on the default DEX is NOT
/// shared with builder DEXs. Use this command to fund a builder DEX before placing
/// orders on its RWA / equity / commodity markets.
///
/// Examples:
///   # Move $5 USDC from default DEX to xyz (RWAs: CL, BRENTOIL, NVDA, TSLA)
///   hyperliquid-plugin dex-transfer --to-dex xyz --amount 5 --confirm
///
///   # Move $1 from xyz back to default DEX
///   hyperliquid-plugin dex-transfer --from-dex xyz --amount 1 --confirm
///
///   # Move $0.5 from xyz to flx (RWAs: OIL, GOLD, SILVER, PALLADIUM)
///   hyperliquid-plugin dex-transfer --from-dex xyz --to-dex flx --amount 0.5 --confirm
///
/// All operations require explicit `--confirm`. Sourced via onchainos
/// `wallet sign-message --type eip712` with the HIP-3 SendAsset typed-data schema
/// (8-field, signing-verified 2026-04-30 via offline ecrecover round-trip).
#[derive(Args)]
pub struct DexTransferArgs {
    /// Source DEX name. Pass empty string "" or omit for default Hyperliquid perp DEX.
    /// e.g. --from-dex xyz to move USDC OUT of the xyz builder DEX.
    #[arg(long, default_value = "")]
    pub from_dex: String,

    /// Destination DEX name. Pass empty string "" or omit for default DEX.
    /// e.g. --to-dex xyz to fund xyz builder DEX for RWA trading.
    #[arg(long, default_value = "")]
    pub to_dex: String,

    /// Amount of USDC to transfer (human-readable, e.g. 5 or 0.5).
    #[arg(long)]
    pub amount: String,

    /// Dry-run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: DexTransferArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    // Validate amount
    let amount_f: f64 = match args.amount.parse() {
        Ok(v) if v > 0.0 => v,
        _ => {
            println!("{}", super::error_response(
                &format!("Invalid --amount '{}': must be a positive number", args.amount),
                "INVALID_ARGUMENT",
                "e.g. --amount 5 (USDC)",
            ));
            return Ok(());
        }
    };
    let amount = format!("{}", amount_f); // canonical decimal string

    // sourceDex == destinationDex check
    if args.from_dex.eq_ignore_ascii_case(&args.to_dex) {
        println!("{}", super::error_response(
            &format!("--from-dex and --to-dex are both '{}'; nothing to transfer", args.from_dex),
            "INVALID_ARGUMENT",
            "Pick different source and destination DEXs.",
        ));
        return Ok(());
    }

    // Fetch dex registry to validate dex names + resolve collateral token
    let registry = match fetch_perp_dexs(info).await {
        Ok(r) => r,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("Failed to fetch perpDexs: {:#}", e),
                "API_ERROR", "Hyperliquid info endpoint may be limited; retry shortly.",
            ));
            return Ok(());
        }
    };

    // Validate from_dex / to_dex are known names (or empty for default)
    if !args.from_dex.is_empty() && find_dex(&registry, &args.from_dex).is_none() {
        return print_unknown_dex(&args.from_dex, &registry);
    }
    if !args.to_dex.is_empty() && find_dex(&registry, &args.to_dex).is_none() {
        return print_unknown_dex(&args.to_dex, &registry);
    }

    // Resolve wallet
    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("{:#}", e), "WALLET_NOT_FOUND",
                "Run `onchainos wallet addresses` to verify login.",
            ));
            return Ok(());
        }
    };

    // Pre-flight: check source DEX has enough USDC
    let source_dex_arg: Option<&str> = if args.from_dex.is_empty() { None } else { Some(&args.from_dex) };
    let source_state = get_clearinghouse_state_for_dex(info, &wallet, source_dex_arg).await
        .ok();
    let source_balance: f64 = source_state.as_ref()
        .and_then(|s| s["withdrawable"].as_str().and_then(|v| v.parse().ok()))
        .unwrap_or(0.0);
    let source_label = if args.from_dex.is_empty() { "default" } else { &args.from_dex };
    let dest_label = if args.to_dex.is_empty() { "default" } else { &args.to_dex };

    if source_balance < amount_f {
        let total_value = source_state.as_ref()
            .and_then(|s| s["marginSummary"]["accountValue"].as_str().and_then(|v| v.parse::<f64>().ok()))
            .unwrap_or(0.0);
        let tip = if total_value > source_balance {
            format!("{} DEX has ${:.4} total but only ${:.4} withdrawable (positions tying up margin). Close positions or reduce --amount.",
                source_label, total_value, source_balance)
        } else {
            format!("{} DEX has only ${:.4} USDC. Use a smaller --amount or fund the source DEX first.",
                source_label, source_balance)
        };
        println!("{}", serde_json::to_string_pretty(&json!({
            "ok": false,
            "error": format!("Insufficient USDC on {} DEX (have ${:.4}, need ${:.4})", source_label, source_balance, amount_f),
            "error_code": "DEX_INSUFFICIENT_BALANCE",
            "source_dex": source_label,
            "destination_dex": dest_label,
            "source_withdrawable_usd": format!("{:.4}", source_balance),
            "tip": tip,
        }))?);
        return Ok(());
    }

    // Resolve token field — sendAsset expects "<symbol>:<tokenId>" format. tokenId is
    // HL's internal token identifier (NOT the Arbitrum EVM contract address). For USDC
    // (the canonical collateral on every DEX), tokenId is 0x6d1e7cde53ba9467b783cb7c530ce054
    // — verified live via `info.spotMeta` 2026-04-30. If a builder DEX ever uses a
    // non-USDC collateral, this needs to be looked up dynamically from the destination
    // DEX's metaAndAssetCtxs[0].collateralToken (which is a token index pointing into
    // spotMeta.tokens[]).
    let token_str = "USDC:0x6d1e7cde53ba9467b783cb7c530ce054";

    let nonce = now_ms();
    let action = json!({
        "type":             "sendAsset",
        "hyperliquidChain": "Mainnet",
        "signatureChainId": "0x66eee",
        "destination":      wallet,           // self-transfer between own DEX accounts
        "sourceDex":        args.from_dex,
        "destinationDex":   args.to_dex,
        "token":            token_str,
        "amount":           amount,
        "fromSubAccount":   "",
        "nonce":            nonce,
    });

    // Pre-flight summary
    let preview = json!({
        "ok": true,
        "stage": if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" },
        "preview": {
            "action": "sendAsset",
            "from": wallet,
            "source_dex": source_label,
            "destination_dex": dest_label,
            "amount_usdc": amount,
            "source_withdrawable_before": format!("{:.4}", source_balance),
            "token": token_str,
            "nonce": nonce,
        }
    });

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[DRY RUN] sendAsset action built; not signed.");
        return Ok(());
    }
    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[PREVIEW] Add --confirm to sign and submit.");
        return Ok(());
    }

    // Sign + submit
    eprintln!("[dex-transfer] Signing sendAsset action via onchainos...");
    let signed = match onchainos_hl_sign_send_asset(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("{:#}", e), "SIGNING_FAILED",
                "Retry the command. If the issue persists, run `onchainos wallet status`.",
            ));
            return Ok(());
        }
    };
    eprintln!("[dex-transfer] Submitting to Hyperliquid exchange...");
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("{:#}", e), "TX_SUBMIT_FAILED",
                "Retry the command. Common: source DEX balance changed between preview and submit.",
            ));
            return Ok(());
        }
    };

    // Inspect result
    let status = result["status"].as_str().unwrap_or("");
    if status == "ok" {
        println!("{}", serde_json::to_string_pretty(&json!({
            "ok": true,
            "action": "dex_transfer",
            "from_dex": source_label,
            "to_dex": dest_label,
            "amount_usdc": amount,
            "result": result,
            "tip": format!("Run `hyperliquid-plugin positions --dex {}` to confirm balance arrived on the destination DEX.", dest_label),
        }))?);
    } else {
        println!("{}", super::error_response(
            &format!("Hyperliquid rejected sendAsset: {}", serde_json::to_string(&result).unwrap_or_default()),
            "TX_REJECTED",
            "Check `result.response` for HL's specific error reason.",
        ));
    }
    Ok(())
}

fn print_unknown_dex(name: &str, registry: &[BuilderDex]) -> anyhow::Result<()> {
    let known: Vec<String> = registry.iter().map(|d| d.name.clone()).collect();
    println!("{}", super::error_response(
        &format!("Unknown DEX '{}'. Known builder DEXs: {}", name, known.join(", ")),
        "INVALID_DEX",
        "Run `hyperliquid-plugin dex-list` to see all registered builder DEXs (or pass empty string for the default DEX).",
    ));
    Ok(())
}
