use clap::Args;
use tokio::time::{sleep, Duration};
use crate::api::{get_quote, get_eth_balance, explorer_tx_url, resolve_token, token_symbol, QuoteRequest, NATIVE_ETH};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};

#[derive(Args)]
pub struct BridgeArgs {
    /// Source chain ID (e.g. 1 for Ethereum, 42161 for Arbitrum)
    #[arg(long)]
    pub from_chain: u64,
    /// Destination chain ID
    #[arg(long)]
    pub to_chain: u64,
    /// Token to send (symbol or address, e.g. ETH, USDC, 0x...)
    #[arg(long, default_value = "ETH")]
    pub token: String,
    /// Amount to send in human-readable form (e.g. 0.01 for 0.01 ETH)
    #[arg(long)]
    pub amount: String,
    /// Destination token (defaults to same as --token)
    #[arg(long)]
    pub to_token: Option<String>,
    /// Recipient address on destination chain (defaults to your wallet)
    #[arg(long)]
    pub recipient: Option<String>,
    /// Broadcast the bridge transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Build calldata without calling onchainos (dry-run)
    #[arg(long, conflicts_with = "confirm")]
    pub dry_run: bool,
}

pub fn parse_amount(s: &str, decimals: u8) -> anyhow::Result<u128> {
    if s == "0" || s.is_empty() {
        anyhow::bail!("Amount must be greater than 0");
    }
    let (whole, frac) = if let Some(dot) = s.find('.') {
        let w: u128 = s[..dot].parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        let frac_str = &s[dot + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!("Amount '{}' has {} decimal places but token supports only {}", s, frac_str.len(), decimals);
        }
        let padded = format!("{:0<width$}", frac_str, width = decimals as usize);
        let f: u128 = padded.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        (w, f)
    } else {
        let w: u128 = s.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        (w, 0u128)
    };
    let scale = 10u128.pow(decimals as u32);
    Ok(whole * scale + frac)
}

fn format_value(raw: u128, decimals: u8) -> String {
    let scale = 10u128.pow(decimals as u32);
    let whole = raw / scale;
    let frac = raw % scale;
    if frac == 0 {
        whole.to_string()
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        format!("{}.{}", whole, frac_str.trim_end_matches('0'))
    }
}

pub async fn run(args: BridgeArgs) -> anyhow::Result<()> {
    let origin_token = resolve_token(&args.token, args.from_chain);
    let dest_token_input = args.to_token.as_deref().unwrap_or(&args.token);
    let dest_token = resolve_token(dest_token_input, args.to_chain);

    let decimals: u8 = if origin_token == NATIVE_ETH { 18 }
        else { match token_symbol(&origin_token, args.from_chain) {
            "USDC" | "USDT" => 6,
            _ => 18,
        }};

    let amount_raw = parse_amount(&args.amount, decimals)?;
    if amount_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let wallet = if args.dry_run {
        // Resolve for display only; fall back to placeholder string
        resolve_wallet(args.from_chain)
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    } else if args.confirm {
        // Wallet required for actual broadcast
        resolve_wallet(args.from_chain)?
    } else {
        // Preview only — try to resolve but fall back to zero address for quote API
        resolve_wallet(args.from_chain)
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    };

    let recipient = args.recipient.as_deref().unwrap_or(&wallet);

    // Validate recipient is a well-formed EVM address
    if !recipient.starts_with("0x") || recipient.len() != 42
        || !recipient[2..].chars().all(|c| c.is_ascii_hexdigit())
    {
        anyhow::bail!("Invalid recipient address '{}': must be a 42-character hex address (0x...)", recipient);
    }

    let sym_in = if token_symbol(&origin_token, args.from_chain) != "UNKNOWN" {
        token_symbol(&origin_token, args.from_chain).to_string()
    } else { args.token.clone() };
    let sym_out = if token_symbol(&dest_token, args.to_chain) != "UNKNOWN" {
        token_symbol(&dest_token, args.to_chain).to_string()
    } else { dest_token_input.to_string() };

    let quote = get_quote(QuoteRequest {
        user: wallet.clone(),
        recipient: recipient.to_string(),
        origin_chain_id: args.from_chain,
        destination_chain_id: args.to_chain,
        origin_currency: origin_token.clone(),
        destination_currency: dest_token.clone(),
        amount: amount_raw.to_string(),
        trade_type: "EXACT_INPUT".to_string(),
    }).await.map_err(|e| {
        let msg = e.to_string();
        // API returns 500 for unrecognised chain IDs — surface a helpful hint
        if msg.contains("500") || msg.contains("validating") {
            anyhow::anyhow!("{}\nTip: run `relay chains` to see all supported chain IDs.", e)
        } else {
            e
        }
    })?;

    let request_id = quote.steps.first()
        .and_then(|s| s.request_id.as_deref())
        .unwrap_or("unknown")
        .to_string();

    let amount_out_fmt = quote.details.as_ref()
        .and_then(|d| d.currency_out.as_ref())
        .and_then(|c| c.amount_formatted.as_deref())
        .unwrap_or(&format_value(amount_raw, decimals))
        .to_string();
    let amount_out_usd = quote.details.as_ref()
        .and_then(|d| d.currency_out.as_ref())
        .and_then(|c| c.amount_usd.as_deref())
        .unwrap_or("unknown")
        .to_string();
    let time_secs = quote.details.as_ref()
        .and_then(|d| d.time_estimate)
        .unwrap_or(0);

    let steps_summary: Vec<&str> = quote.steps.iter().map(|s| s.id.as_str()).collect();
    let preview_fee_usd = quote.fees.as_ref()
        .and_then(|f| f.pointer("/relayer/amountUsd"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Balance warning — fetch on-chain balance and warn if amount exceeds it.
    // Only for native ETH (ERC-20 balance check would require a separate call per token).
    // A failed RPC returns 0 so this never blocks the preview.
    let balance_warning: Option<String> = if origin_token == NATIVE_ETH {
        let balance = get_eth_balance(args.from_chain, &wallet).await;
        if balance > 0 && amount_raw > balance {
            let bal_eth = format_value(balance, 18);
            Some(format!(
                "Requested {} ETH exceeds wallet balance of {} ETH. \
                 The transaction will revert on-chain.",
                args.amount, bal_eth
            ))
        } else {
            None
        }
    } else {
        None
    };

    let mut preview = serde_json::json!({
        "preview":     true,
        "action":      "bridge",
        "token_in":    sym_in,
        "token_out":   sym_out,
        "amount_in":   args.amount,
        "amount_out":  amount_out_fmt,
        "amount_out_usd": amount_out_usd,
        "fee_usd":     preview_fee_usd,
        "from_chain":  args.from_chain,
        "to_chain":    args.to_chain,
        "recipient":   recipient,
        "wallet":      wallet,
        "estimated_time_secs": time_secs,
        "steps":       steps_summary,
        "request_id":  request_id,
    });
    if let Some(ref warn) = balance_warning {
        preview["balance_warning"] = serde_json::json!(warn);
    }

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast this bridge transaction.");
        eprintln!("Track status with: relay status --request-id {}", request_id);
        return Ok(());
    }

    // Block --confirm execution if the balance check detected insufficient funds.
    // A failed RPC (balance == 0) skips the check, so this never false-positives
    // on an unreachable node.
    if args.confirm {
        if let Some(ref warn) = balance_warning {
            anyhow::bail!("{} Refusing to broadcast.", warn);
        }
    }

    // Execute each step in order
    let mut tx_hashes: Vec<serde_json::Value> = Vec::new();

    for step in &quote.steps {
        let step_id = step.id.as_str();
        for item in &step.items {
            let data = &item.data;
            let to = &data.to;
            let calldata = &data.data;
            let value = data.value.as_deref().unwrap_or("0");
            let step_chain = data.chain_id.unwrap_or(args.from_chain);

            if step_id == "approve" {
                eprintln!("[relay] Approving unlimited {} for bridge...", sym_in);
            } else {
                eprintln!("[relay] Sending {} {} from chain {} to chain {}...",
                    args.amount, sym_in, args.from_chain, args.to_chain);
            }

            let result = wallet_contract_call(
                step_chain, to, calldata, value,
                true, args.dry_run, Some(&wallet),
            )?;

            let tx_hash = extract_tx_hash(&result);
            eprintln!("[relay] {} tx: {}", step_id, tx_hash);

            let explorer = if !args.dry_run && !tx_hash.starts_with("0x000000000000") {
                explorer_tx_url(step_chain, &tx_hash)
            } else {
                String::new()
            };
            let mut tx_entry = serde_json::json!({
                "step":    step_id,
                "tx_hash": tx_hash,
                "chain":   step_chain,
            });
            if !explorer.is_empty() {
                tx_entry["explorer"] = serde_json::json!(explorer);
            }
            tx_hashes.push(tx_entry);

            // Wait between steps so approval confirms before deposit
            if step_id == "approve" && !args.dry_run {
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    let fee_usd = quote.fees.as_ref()
        .and_then(|f| f.pointer("/relayer/amountUsd"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut out = serde_json::json!({
        "ok":          true,
        "action":      "bridge",
        "token_in":    sym_in,
        "token_out":   sym_out,
        "amount_in":   args.amount,
        "amount_out":  amount_out_fmt,
        "fee_usd":     fee_usd,
        "from_chain":  args.from_chain,
        "to_chain":    args.to_chain,
        "wallet":      wallet,
        "recipient":   recipient,
        "request_id":  request_id,
        "txs":         tx_hashes,
        "track":       format!("relay status --request-id {}", request_id),
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
