/// `fourmeme-plugin create-token` — launch a new memecoin on Four.meme.
///
/// Flow:
///   1. POST four.meme/meme-api/v1/private/token/create with the user's
///      `meme-web-access` cookie (issued at four.meme login, ~30-day TTL).
///      Backend returns `createArg` (ABI-encoded metadata) + `signature`
///      (65-byte ECDSA) + the deterministic CREATE2 token address.
///   2. ABI-encode `createToken(bytes,bytes)` calldata locally.
///   3. Send tx to TokenManager V2. msg.value = 0 (deployCost is "0" for both
///      BNB- and USDT-quoted tokens; first-buy is deferred to v0.2).
///   4. Wait for receipt via direct RPC (TX-001).
///
/// Auth note: the cookie is bound to a specific wallet (the one logged into
/// four.meme). The on-chain tx must be signed by **that same wallet** — the
/// signature blob is wallet-tied. If `--from` differs from the four.meme login
/// wallet, the contract will revert.

use anyhow::{Context, Result};
use clap::Args;

use crate::api::{create_token as api_create_token, CreateTokenRequest, QuoteToken};
use crate::config::{chain_name, is_supported_chain, addresses};
use crate::rpc::{eth_call, eth_get_balance_wei, estimate_native_gas_cost_wei, parse_uint256_to_u128, wei_to_bnb};

const GAS_LIMIT_CREATE: u64 = 4_000_000;

#[derive(Args)]
pub struct CreateTokenArgs {
    /// Display name (≤32 chars typically)
    #[arg(long)]
    pub name: String,

    /// Symbol (also stored as on-chain `name`/`symbol` in the deployed contract)
    #[arg(long)]
    pub symbol: String,

    /// Free-form description shown on four.meme
    #[arg(long, default_value = "")]
    pub desc: String,

    /// Path to a local logo file (.png/.jpg/.gif/.webp). The CLI uploads it via
    /// four.meme's image API and uses the returned CDN URL automatically.
    /// Mutually exclusive with --image-url.
    #[arg(long, conflicts_with = "image_url")]
    pub image_file: Option<std::path::PathBuf>,

    /// Pre-existing four.meme CDN URL (`https://static.four.meme/market/...`).
    /// Use this if you already uploaded the logo via the web UI or in a prior
    /// run. Mutually exclusive with --image-file.
    #[arg(long, conflicts_with = "image_file")]
    pub image_url: Option<String>,

    /// Quote token: `bnb` or `usdt`
    #[arg(long, default_value = "bnb")]
    pub quote: String,

    /// Total supply (default 1 billion — Four.meme standard)
    #[arg(long, default_value_t = 1_000_000_000)]
    pub total_supply: u64,

    /// Raised target in quote-token whole units. Defaults: 18 (BNB) / 12000 (USDT).
    #[arg(long)]
    pub raised_amount: Option<u64>,

    /// Seconds-from-now to launch on chain (≥ 5 recommended so the backend
    /// has time to receive and sign).
    #[arg(long, default_value_t = 5)]
    pub launch_delay_secs: u64,

    /// Token category. One of: Meme, AI, Defi, Games, Infra, De-Sci, Social, Depin, Charity, Others.
    #[arg(long, default_value = "Meme")]
    pub label: String,

    #[arg(long)]
    pub web_url: Option<String>,
    #[arg(long)]
    pub twitter_url: Option<String>,
    #[arg(long)]
    pub telegram_url: Option<String>,

    /// Optional presale amount in whole quote-token units (BNB or USDT). 0.001 = 0.001 BNB.
    /// When > 0 and quote=BNB, msg.value is `launch_fee + presale_wei + trading_fee`.
    #[arg(long, default_value_t = 0.0)]
    pub presale: f64,

    #[arg(long, default_value_t = false)]
    pub fee_plan: bool,

    /// Path to JSON file with `{"tokenTaxInfo": {...}}` (for TaxToken creates)
    #[arg(long)]
    pub tax_options: Option<std::path::PathBuf>,

    /// Override auth token. By default the CLI loads the token saved by
    /// `fourmeme-plugin login` from ~/.fourmeme-plugin/auth.json. Pass this
    /// only if you want to use a hand-pasted browser cookie instead.
    #[arg(long)]
    pub auth_token: Option<String>,

    #[arg(long, default_value_t = 56)]
    pub chain: u64,

    /// Pass --confirm to actually submit the on-chain tx. Default is preview-only
    /// (prints the planned tx without spending gas) so accidental invocation is safe.
    #[arg(long, default_value_t = false)]
    pub confirm: bool,
}

pub async fn run(args: CreateTokenArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("create-token"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: CreateTokenArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    const VALID_LABELS: &[&str] = &[
        "Meme","AI","Defi","Games","Infra","De-Sci","Social","Depin","Charity","Others",
    ];
    if !VALID_LABELS.contains(&args.label.as_str()) {
        anyhow::bail!(
            "--label '{}' not supported. Choose one of: {}",
            args.label, VALID_LABELS.join(", ")
        );
    }

    // Resolve auth token: explicit flag wins, otherwise load from auth.json.
    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;
    let auth_token = crate::auth::resolve_token(args.auth_token.as_deref(), &wallet)?;

    let quote = QuoteToken::parse(&args.quote)?;
    let raised_amount = args.raised_amount.unwrap_or_else(|| quote.default_raised_amount());

    // Optional tax token config — load JSON once into a Value the API helper clones in.
    let tax_value: Option<serde_json::Value> = match args.tax_options.as_ref() {
        None => None,
        Some(path) => {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("reading tax-options file {}", path.display()))?;
            let v: serde_json::Value = serde_json::from_str(&raw)
                .with_context(|| format!("parsing tax-options JSON {}", path.display()))?;
            Some(v.get("tokenTaxInfo").cloned()
                .ok_or_else(|| anyhow::anyhow!("tax-options must have top-level `tokenTaxInfo` object"))?)
        }
    };

    // Resolve image: upload local file, or use pre-existing CDN URL.
    let image_url = match (&args.image_file, &args.image_url) {
        (Some(path), None) => {
            eprintln!("[fourmeme] uploading {} to four.meme CDN...", path.display());
            crate::api::upload_image(&auth_token, path).await
                .context("image upload step failed")?
        }
        (None, Some(url)) => url.clone(),
        (Some(_), Some(_)) => unreachable!("clap conflicts_with prevents this"),
        (None, None) => anyhow::bail!(
            "Provide either --image-file <path> (CLI uploads it) or --image-url <https://static.four.meme/...>."
        ),
    };

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let launch_time_ms = now_ms + (args.launch_delay_secs as i64) * 1000;

    eprintln!(
        "[fourmeme] requesting createArg from four.meme backend (quote={}, raised={})...",
        quote.symbol(), raised_amount
    );
    let api_req = CreateTokenRequest {
        auth_token:    &auth_token,
        name:          &args.name,
        symbol:        &args.symbol,
        desc:          &args.desc,
        img_url:       &image_url,
        total_supply:  args.total_supply,
        raised_amount,
        quote,
        launch_time_ms,
        label:         &args.label,
        web_url:       args.web_url.as_deref(),
        twitter_url:   args.twitter_url.as_deref(),
        telegram_url:  args.telegram_url.as_deref(),
        presale_ether: args.presale,
        fee_plan:      args.fee_plan,
        tax_token:     tax_value.as_ref(),
    };
    let api_resp = api_create_token(&api_req).await
        .context("four.meme backend rejected the create request")?;

    let calldata = crate::calldata::build_create_token(
        &api_resp.create_arg,
        &api_resp.signature,
    );

    // Read TM2._launchFee() — base required msg.value.
    let launch_fee_data = crate::calldata::build_no_args(crate::calldata::SEL_LAUNCH_FEE);
    let launch_fee_hex = eth_call(args.chain, addresses::TOKEN_MANAGER_V2, &launch_fee_data).await
        .context("reading TM2._launchFee()")?;
    let launch_fee_wei = parse_uint256_to_u128(&launch_fee_hex);

    // If presale > 0 and quote == BNB, msg.value = launch_fee + presale_wei + trading_fee
    let mut creation_fee_wei = launch_fee_wei;
    if args.presale > 0.0 && matches!(quote, QuoteToken::Bnb) {
        let presale_wei = (args.presale * 1e18).round() as u128;
        let fee_rate_data = crate::calldata::build_no_args(crate::calldata::SEL_TRADING_FEE_RATE);
        let fee_rate_hex = eth_call(args.chain, addresses::TOKEN_MANAGER_V2, &fee_rate_data).await
            .context("reading TM2._tradingFeeRate()")?;
        let fee_rate_bps = parse_uint256_to_u128(&fee_rate_hex);
        let trading_fee_wei = presale_wei.saturating_mul(fee_rate_bps) / 10_000;
        creation_fee_wei = launch_fee_wei
            .saturating_add(presale_wei)
            .saturating_add(trading_fee_wei);
        eprintln!(
            "[fourmeme] _launchFee={} + presale={} + tradingFee={} → msg.value={} wei",
            launch_fee_wei, presale_wei, trading_fee_wei, creation_fee_wei
        );
    } else {
        eprintln!("[fourmeme] _launchFee() = {} wei ({:.4} BNB)",
            launch_fee_wei, launch_fee_wei as f64 / 1e18);
    }
    let launch_fee_wei = creation_fee_wei; // rename for downstream sites

    if !args.confirm {
        let resp = serde_json::json!({
            "ok": true,
            "preview_only": true,
            "data": {
                "action": "create-token",
                "chain": chain_name(args.chain),
                "chain_id": args.chain,
                "wallet": wallet,
                "input": {
                    "name": args.name, "symbol": args.symbol,
                    "desc": args.desc, "image_url": image_url,
                    "quote": quote.symbol(),
                    "total_supply": args.total_supply,
                    "raised_amount": raised_amount,
                    "launch_time_ms": launch_time_ms,
                },
                "backend_response": {
                    "token_id":      api_resp.token_id,
                    "token_address": api_resp.token_address,
                    "template":      api_resp.template,
                    "launch_time":   api_resp.launch_time,
                    "creator_base_amount": api_resp.bamount,
                    "minted_token_amount": api_resp.tamount,
                    "create_arg_length_bytes": api_resp.create_arg.trim_start_matches("0x").len() / 2,
                    "signature_length_bytes":  api_resp.signature.trim_start_matches("0x").len() / 2,
                },
                "creation_fee_wei": launch_fee_wei.to_string(),
                "creation_fee_bnb": format!("{:.6}", launch_fee_wei as f64 / 1e18),
                "tx_plan": [
                    format!(
                        "TokenManager.createToken(createArg, signature) on {} with msg.value={} wei",
                        addresses::TOKEN_MANAGER_V2, launch_fee_wei
                    )
                ],
                "note": "preview only (--confirm omitted): no transaction submitted. Backend ALREADY recorded \
                         this createArg + signature, valid until launchTime expires.",
            }
        });
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    // Pre-flight: BNB needs to cover msg.value (launch fee) + gas
    let need_gas = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_CREATE).await?;
    let need_total = launch_fee_wei.saturating_add(need_gas);
    let have = eth_get_balance_wei(args.chain, &wallet).await?;
    if have < need_total {
        anyhow::bail!(
            "Insufficient BNB: have {:.6}, need ~{:.6} ({:.6} for launch fee + {:.6} for gas).",
            wei_to_bnb(have), wei_to_bnb(need_total),
            wei_to_bnb(launch_fee_wei), wei_to_bnb(need_gas),
        );
    }

    eprintln!(
        "[fourmeme] submitting createToken on-chain — predicted token address: {}",
        api_resp.token_address
    );
    let resp = crate::onchainos::wallet_contract_call(
        args.chain,
        addresses::TOKEN_MANAGER_V2,
        &calldata,
        Some(&wallet),
        Some(launch_fee_wei),
        false, // user-facing tx
    ).await?;
    let tx_hash = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[fourmeme] create tx: {} (waiting for confirmation...)", tx_hash);
    crate::onchainos::wait_for_tx_receipt(&tx_hash, args.chain, 180).await?;

    // Fetch the live state of the just-deployed token so the JSON payload tells the
    // caller "your token exists, here is its current curve state" without a follow-up
    // get-token call. Best-effort — Helper3 indexing can lag a few blocks, so a
    // failure here is downgraded to a stub object instead of failing the command.
    let live = match super::fetch_token_info(args.chain, &api_resp.token_address).await {
        Ok(info) => serde_json::json!({
            "version":          info.version.to_string(),
            "token_manager":    info.token_manager,
            "is_bnb_quoted":    info.is_bnb_quoted(),
            "trading_fee_bps":  info.trading_fee_rate.to_string(),
            "min_trading_fee_raw": info.min_trading_fee.to_string(),
            "offers":      super::fmt_decimal(info.offers, crate::config::TOKEN_DECIMALS),
            "max_offers":  super::fmt_decimal(info.max_offers, crate::config::TOKEN_DECIMALS),
            "funds_bnb":   format!("{:.6}", info.funds as f64 / 1e18),
            "max_funds_bnb": format!("{:.6}", info.max_funds as f64 / 1e18),
            "progress_by_offers_pct": format!("{:.2}", info.progress_by_offers_pct()),
            "progress_by_funds_pct":  format!("{:.2}", info.progress_by_funds_pct()),
            "graduated":   info.liquidity_added,
        }),
        Err(e) => serde_json::json!({
            "note": format!("Helper3 read pending — token created but indexer hasn't caught up yet ({}). Re-run `get-token --address {}` in a few seconds.", e, api_resp.token_address)
        }),
    };

    // Read creator's initial token balance (only non-zero if --presale was set).
    // EVM-012: post-tx read is display-only (the create_token already
    // confirmed). Keep soft fallback but expose query error.
    let (initial_balance, initial_balance_query_error) =
        match super::erc20_balance(args.chain, &api_resp.token_address, &wallet).await {
            Ok(v) => (v, None::<String>),
            Err(e) => (0u128, Some(format!("{:#}", e))),
        };

    let out = serde_json::json!({
        "ok": true,
        "data": {
            "action": "create-token",
            "chain": chain_name(args.chain),
            "chain_id": args.chain,
            "creator_wallet": wallet,
            "token_address": api_resp.token_address,
            "token_id":      api_resp.token_id,
            "template":      api_resp.template,
            "name":   args.name,
            "symbol": args.symbol,
            "label":  args.label,
            "quote":  quote.symbol(),
            "total_supply": args.total_supply,
            "raised_amount_target": raised_amount,
            "presale_ether": args.presale,
            "launch_time_unix": api_resp.launch_time,
            "create_tx": tx_hash,
            "on_chain_status": "0x1",
            "creation_fee_wei":   launch_fee_wei.to_string(),
            "creation_fee_bnb":   format!("{:.6}", launch_fee_wei as f64 / 1e18),
            "creator_initial_balance":     super::fmt_decimal(initial_balance, crate::config::TOKEN_DECIMALS),
            "creator_initial_balance_raw": initial_balance.to_string(),
            "creator_initial_balance_query_error": initial_balance_query_error,
            "live_state": live,
            "tip": format!(
                "Token live at {}. View on four.meme: https://four.meme/token/{}. \
                 Buy via `fourmeme-plugin buy --token {} --funds 0.005`.",
                api_resp.token_address, api_resp.token_address, api_resp.token_address
            ),
        }
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
