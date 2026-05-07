pub mod balance;
pub mod buy;
pub mod quickstart;
pub mod cancel;
pub mod check_access;
pub mod create_readonly_key;
pub mod deposit;
pub mod get_market;
pub mod get_positions;
pub mod get_series;
pub mod list_5m;
pub mod list_markets;
pub mod orders;
pub mod redeem;
pub mod rfq;
pub mod sell;
pub mod setup_proxy;
pub mod setup_deposit_wallet;
pub mod switch_mode;
pub mod watch;
pub mod withdraw;


/// Build a structured error JSON string for stdout output (per GEN-001).
///
/// Use when a command hits a business-logic failure (insufficient gas, tx never
/// broadcast, revert, missing positions, etc.) — the caller should `println!` this
/// and `return Ok(())` so external agents can parse the error instead of seeing
/// only exit code 1 + stderr.
///
/// `extra_hint`, when present, is appended to `suggestion` — useful for attaching
/// context the classifier cannot derive from the error message alone (e.g. a proxy
/// wallet address discovered on-chain for this specific EOA).
pub fn error_response(
    err: &anyhow::Error,
    context: Option<&str>,
    extra_hint: Option<&str>,
) -> String {
    let msg = format!("{:#}", err);
    let (error_code, mut suggestion) = classify_error(&msg, context);
    if let Some(h) = extra_hint {
        let h = h.trim();
        if !h.is_empty() {
            suggestion.push(' ');
            suggestion.push_str(h);
        }
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": error_code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}

fn classify_error(msg: &str, ctx: Option<&str>) -> (&'static str, String) {
    let m = msg.to_lowercase();

    // ── Network / RPC ───────────────────────────────────────────────────────
    if m.contains("polygon rpc") || m.contains("rpc request failed") || m.contains("rpc error") {
        return (
            "RPC_UNAVAILABLE",
            "Polygon RPC is unavailable or rate-limited. Wait a few seconds and retry. \
             If it persists, the public RPC may be congested — Polymarket data is unaffected.".into(),
        );
    }
    if m.contains("error sending request") || m.contains("connection refused")
        || m.contains("dns error") || m.contains("certificate") {
        return (
            "NETWORK_UNREACHABLE",
            "Network request failed. Check your internet connection and that polymarket.com / \
             gamma-api.polymarket.com are reachable from your IP. If you see TLS / certificate \
             errors, your ISP or network may be intercepting Polymarket traffic.".into(),
        );
    }
    if m.contains("trading restricted in your region") || m.contains("clob blocked")
        || m.contains("region") {
        return (
            "REGION_RESTRICTED",
            "Polymarket CLOB is blocking this IP (US / OFAC). Switch region (VPN) and re-run \
             `check-access` to verify before any trading commands.".into(),
        );
    }

    // ── Auth / wallet ───────────────────────────────────────────────────────
    if m.contains("credentials are stale") || m.contains("invalid credential")
        || m.contains("api key") {
        return (
            "STALE_CREDENTIALS",
            "Polymarket CLOB credentials are stale. Delete `~/.config/polymarket/creds.json` \
             and re-run the command — credentials will be re-derived automatically.".into(),
        );
    }
    if m.contains("no wallet") || m.contains("wallet not found")
        || m.contains("onchainos wallet") && m.contains("failed") {
        return (
            "NO_WALLET",
            "No active onchainos wallet found. Run `onchainos wallet status` to inspect, or \
             `onchainos wallet add` to create one. Polymarket needs a wallet on Polygon (chain 137).".into(),
        );
    }

    // ── Balance / allowance ─────────────────────────────────────────────────
    if m.contains("insufficient pol") {
        return (
            "INSUFFICIENT_POL_GAS",
            "Top up POL on your EOA wallet (Polygon). Redeem costs ~0.015 POL per market; \
             setup-proxy needs ~0.05 POL for the V1+V2 approval txs. Trading via POLY_PROXY \
             is gasless after setup.".into(),
        );
    }
    if m.contains("insufficient usdc") || m.contains("insufficient pusd")
        || m.contains("insufficient balance") {
        return (
            "INSUFFICIENT_BALANCE",
            "Wallet does not hold enough collateral for this order. Check `balance` for the \
             active wallet (EOA vs proxy), and `deposit --amount <N>` if the proxy needs funding.".into(),
        );
    }
    if m.contains("insufficient allowance") || m.contains("erc20: insufficient allowance") {
        return (
            "INSUFFICIENT_ALLOWANCE",
            "Token allowance too low. Re-run `setup-proxy` to ensure all 10 V1+V2 approvals \
             are in place — the per-pair idempotency check will only resubmit the missing ones.".into(),
        );
    }

    // ── Order placement / sizing ────────────────────────────────────────────
    if m.contains("rounds to 0 shares") || m.contains("divisibility") {
        return (
            "ORDER_TOO_SMALL_DIVISIBILITY",
            "Order amount rounds to 0 shares at this price. Increase `--amount` or pass \
             `--round-up` to snap to the minimum valid amount.".into(),
        );
    }
    if m.contains("below this market's minimum") || m.contains("min_order_size") {
        return (
            "ORDER_BELOW_SHARE_MINIMUM",
            "Order size is below the market's share minimum (typically 5 shares for resting \
             GTC limits). Pass `--round-up` or use `--order-type FOK` (subject to a separate \
             ~$1 CLOB execution floor).".into(),
        );
    }
    if m.contains("price slippage") || m.contains("not enough liquidity") {
        return (
            "SLIPPAGE_OR_LIQUIDITY",
            "Order would not fill at the requested price (insufficient liquidity or price \
             moved). Inspect `get-market` order book and retry with a worse price or smaller \
             size.".into(),
        );
    }

    // ── Tx lifecycle ────────────────────────────────────────────────────────
    if m.contains("simulation reverted") || m.contains("eth_call reverted") {
        return (
            "SIMULATION_REVERTED",
            "Pre-flight eth_call simulation reverted. For redeem: the calling wallet doesn't \
             hold the winning tokens — check trading mode. For buy/sell: insufficient balance \
             or allowance — run `setup-proxy` and `balance`.".into(),
        );
    }
    if m.contains("not observed on-chain") || m.contains("not confirmed within")
        || m.contains("did not confirm") {
        return (
            "TX_NOT_CONFIRMED",
            "Tx returned a hash but never appeared on-chain within the timeout. Usually means \
             onchainos signed a tx that would revert and dropped it silently. Check Polygonscan \
             for the hash; if missing, re-run with --dry-run.".into(),
        );
    }
    if m.contains("mined but reverted") || m.contains("status 0x0") {
        return (
            "TX_REVERTED",
            "Tx mined but reverted on-chain. Check Polygonscan for the failure reason. \
             For redeem this usually means the calling wallet doesn't hold the winning tokens.".into(),
        );
    }

    // ── Domain-specific (redeem) ────────────────────────────────────────────
    if m.contains("no redeemable positions") {
        return (
            "NO_REDEEMABLE_POSITIONS",
            "Data API shows no redeemable positions on either the EOA or the proxy wallet. \
             If you traded in POLY_PROXY mode, run `setup-proxy` first so the plugin knows \
             your proxy address; otherwise verify your trading mode with `balance`.".into(),
        );
    }
    if m.contains("neg_risk") && m.contains("not yet supported") {
        return (
            "NEG_RISK_PROXY_NOT_SUPPORTED",
            "Multi-outcome (neg_risk) redeem from a proxy wallet is not yet supported by this \
             plugin — use the Polymarket web UI. EOA redeem via NegRiskAdapter is fully supported.".into(),
        );
    }

    // ── Setup-proxy specific ────────────────────────────────────────────────
    if m.contains("on-chain proxy check failed") || m.contains("could not retrieve proxy address") {
        return (
            "PROXY_RPC_INDETERMINATE",
            "Could not determine on-chain proxy state from RPC. Refusing to deploy in case a \
             proxy already exists. Wait for RPC recovery and re-run setup-proxy.".into(),
        );
    }
    if m.contains("not an eip-1167 proxy") {
        return (
            "PROXY_ADDRESS_INVALID",
            "Resolved proxy address is not a valid EIP-1167 proxy contract — refusing to use \
             it to protect funds. This usually indicates an RPC trace error; re-run setup-proxy \
             from a different RPC.".into(),
        );
    }
    if m.contains("could not verify") && m.contains("allowance on-chain") {
        return (
            "ALLOWANCE_CHECK_FAILED",
            "Could not verify proxy approval state on-chain. Polygon RPC may be unavailable. \
             Wait a few seconds and re-run setup-proxy.".into(),
        );
    }

    // ── Generic fallback (per command context) ─────────────────────────────
    let default_code: &'static str = match ctx {
        Some("buy")          => "BUY_FAILED",
        Some("sell")         => "SELL_FAILED",
        Some("redeem")       => "REDEEM_FAILED",
        Some("cancel")       => "CANCEL_FAILED",
        Some("rfq")          => "RFQ_FAILED",
        Some("setup-proxy")          => "SETUP_PROXY_FAILED",
        Some("setup-deposit-wallet") => "SETUP_DEPOSIT_WALLET_FAILED",
        Some("deposit")      => "DEPOSIT_FAILED",
        Some("withdraw")     => "WITHDRAW_FAILED",
        Some("quickstart")   => "QUICKSTART_FAILED",
        Some("balance")      => "BALANCE_FAILED",
        Some("orders")       => "ORDERS_FAILED",
        Some("watch")        => "WATCH_FAILED",
        Some("get-market")   => "GET_MARKET_FAILED",
        Some("get-positions")=> "GET_POSITIONS_FAILED",
        Some("get-series")   => "GET_SERIES_FAILED",
        Some("list-markets") => "LIST_MARKETS_FAILED",
        Some("list-5m")      => "LIST_5M_FAILED",
        Some("switch-mode")  => "SWITCH_MODE_FAILED",
        Some("create-readonly-key") => "CREATE_READONLY_KEY_FAILED",
        _                    => "UNKNOWN_ERROR",
    };
    (default_code, "See error field for details. Retry the command, or run with --dry-run to inspect parameters.".into())
}
