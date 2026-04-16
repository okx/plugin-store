use clap::Args;
use crate::config::{ARBITRUM_CHAIN_ID, CHAIN_ID, HYPER_EVM_RPC, USDC_ARBITRUM};
use crate::onchainos::{resolve_wallet, wallet_contract_call};
use crate::rpc::{ARBITRUM_RPC, erc20_balance, erc20_allowance, parse_wei, wait_tx_mined};

const RELAY_API: &str = "https://api.relay.link";
/// native HYPE on HyperEVM (address zero = native gas token)
const HYPE_HYPER_EVM: &str = "0x0000000000000000000000000000000000000000";

#[derive(Args)]
pub struct GetGasArgs {
    /// USDC amount to swap for HYPE on HyperEVM (e.g. 2 for $2 USDC → ~0.047 HYPE)
    #[arg(long)]
    pub amount: f64,

    /// Dry run — show quote without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and execute (without this flag, shows a preview quote)
    #[arg(long)]
    pub confirm: bool,
}

async fn fetch_quote(wallet: &str, usdc_units: u64) -> anyhow::Result<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let resp = client
        .post(format!("{}/quote", RELAY_API))
        .json(&serde_json::json!({
            "user":                wallet,
            "originChainId":       ARBITRUM_CHAIN_ID,
            "destinationChainId":  CHAIN_ID,
            "originCurrency":      USDC_ARBITRUM,
            "destinationCurrency": HYPE_HYPER_EVM,
            "amount":              usdc_units.to_string(),
            "tradeType":           "EXACT_INPUT"
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    if resp.get("steps").is_none() {
        anyhow::bail!("relay.link quote failed: {}", resp);
    }
    Ok(resp)
}

/// Native ETH/HYPE balance on HyperEVM via eth_getBalance
async fn hype_balance(address: &str) -> anyhow::Result<f64> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp: serde_json::Value = client
        .post(HYPER_EVM_RPC)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBalance",
            "params": [address, "latest"]
        }))
        .send()
        .await?
        .json()
        .await?;
    let hex = resp["result"].as_str().unwrap_or("0x0");
    let wei = u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0);
    Ok(wei as f64 / 1e18)
}

pub async fn run(args: GetGasArgs) -> anyhow::Result<()> {
    if args.amount <= 0.0 {
        anyhow::bail!("--amount must be positive");
    }

    let usdc_units = (args.amount * 1_000_000.0).round() as u64;
    let wallet = resolve_wallet(ARBITRUM_CHAIN_ID)?;

    // Check USDC balance on Arbitrum
    let balance = erc20_balance(USDC_ARBITRUM, &wallet, ARBITRUM_RPC).await?;
    let balance_usd = balance as f64 / 1_000_000.0;
    if balance < usdc_units as u128 {
        anyhow::bail!(
            "Insufficient USDC on Arbitrum: have {:.4} USDC, need {:.4} USDC",
            balance_usd, args.amount
        );
    }

    // Fetch quote from relay.link
    let quote = fetch_quote(&wallet, usdc_units).await?;

    // Parse key fields from quote
    let hype_out = quote["details"]["currencyOut"]["amountFormatted"]
        .as_str().unwrap_or("?");
    let hype_usd = quote["details"]["currencyOut"]["amountUsd"]
        .as_str().unwrap_or("?");
    let fee_usdc = quote["fees"]["relayer"]["amountFormatted"]
        .as_str().unwrap_or("?");
    let time_est = quote["details"]["timeEstimate"]
        .as_u64().unwrap_or(3);

    let steps = quote["steps"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No steps in relay.link response"))?;

    if args.dry_run || !args.confirm {
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": !args.confirm,
            "dry_run": args.dry_run,
            "wallet": wallet,
            "spend": format!("{} USDC", args.amount),
            "receive": format!("{} HYPE (~${})", hype_out, hype_usd),
            "relayer_fee": format!("{} USDC", fee_usdc),
            "estimated_seconds": time_est,
            "route": "Arbitrum USDC → relay.link → HyperEVM HYPE",
            "note": if args.confirm { "" } else { "Add --confirm to execute" }
        }));
        return Ok(());
    }

    // ── Separate approve vs deposit steps by id ───────────────────────────
    let mut approve_step: Option<&serde_json::Value> = None;
    let mut deposit_step: Option<&serde_json::Value> = None;

    for step in steps.iter() {
        match step["id"].as_str().unwrap_or("") {
            "approve" => approve_step = Some(step),
            _ => deposit_step = Some(step),   // "deposit", "send", etc.
        }
    }

    // If only one step, treat it as the deposit
    if deposit_step.is_none() {
        deposit_step = steps.first();
    }

    let deposit_step = deposit_step
        .ok_or_else(|| anyhow::anyhow!("relay.link returned no deposit step"))?;

    let request_id = deposit_step["requestId"].as_str().unwrap_or("");

    // ── Step 1 (optional): Approve USDC ──────────────────────────────────
    if let Some(approve) = approve_step {
        let data = &approve["items"][0]["data"];
        let approve_to = data["to"].as_str().unwrap_or("");
        let approve_calldata = data["data"].as_str().unwrap_or("");
        let approve_value = parse_wei(data["value"].as_str().unwrap_or("0x0"));

        // Skip if allowance is already sufficient
        let existing = erc20_allowance(USDC_ARBITRUM, &wallet, approve_to, ARBITRUM_RPC)
            .await
            .unwrap_or(0);

        if existing < usdc_units as u128 {
            eprintln!("Approving USDC to relay solver...");
            let approve_value_opt = if approve_value > 0 { Some(approve_value) } else { None };
            let result = wallet_contract_call(
                ARBITRUM_CHAIN_ID, approve_to, approve_calldata, approve_value_opt, false
            )?;

            // Wait for the approve tx to be mined so deposit simulation succeeds
            let tx_hash = result["data"]["txHash"].as_str().unwrap_or("");
            if !tx_hash.is_empty() {
                eprint!("  Waiting for approve tx {} to confirm...", tx_hash);
                let confirmed = wait_tx_mined(tx_hash, ARBITRUM_RPC).await;
                eprintln!(" {}", if confirmed { "confirmed" } else { "timed out (proceeding anyway)" });
            }
        } else {
            eprintln!("USDC allowance already sufficient, skipping approve.");
        }
    }

    // ── Step 2: Deposit (triggers cross-chain swap) ───────────────────────
    // Fetch a fresh quote right before deposit so requestId is not stale
    let fresh_quote = fetch_quote(&wallet, usdc_units).await?;
    let fresh_steps = fresh_quote["steps"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No steps in fresh relay.link quote"))?;

    let fresh_deposit = fresh_steps.iter()
        .find(|s| s["id"].as_str().unwrap_or("") != "approve")
        .or_else(|| fresh_steps.last())
        .ok_or_else(|| anyhow::anyhow!("No deposit step in fresh quote"))?;

    let dep_data = &fresh_deposit["items"][0]["data"];
    let deposit_to = dep_data["to"].as_str().unwrap_or("");
    let deposit_calldata = dep_data["data"].as_str().unwrap_or("");
    let deposit_value = parse_wei(dep_data["value"].as_str().unwrap_or("0x0"));
    let fresh_request_id = fresh_deposit["requestId"]
        .as_str()
        .unwrap_or(request_id);

    eprintln!("Depositing {} USDC to relay solver...", args.amount);
    let deposit_value_opt = if deposit_value > 0 { Some(deposit_value) } else { None };
    wallet_contract_call(ARBITRUM_CHAIN_ID, deposit_to, deposit_calldata, deposit_value_opt, false)?;

    // Poll relay.link status until HYPE arrives (max ~40s)
    eprintln!("Waiting for HYPE to arrive on HyperEVM (~{} seconds)...", time_est);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut arrived = false;
    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        if let Ok(resp) = client
            .get(format!("{}/intents/status?requestId={}", RELAY_API, fresh_request_id))
            .send().await
        {
            if let Ok(status) = resp.json::<serde_json::Value>().await {
                if status["status"].as_str() == Some("success") {
                    arrived = true;
                    break;
                }
            }
        }
    }

    let hype_now = hype_balance(&wallet).await.unwrap_or(0.0);

    println!("{}", serde_json::json!({
        "ok": true,
        "action": "get-gas",
        "wallet": wallet,
        "spent_usdc": args.amount,
        "hype_received": hype_out,
        "hype_balance_now": format!("{:.6}", hype_now),
        "confirmed": arrived,
        "note": if arrived {
            "HYPE arrived. You can now use CoreWriter operations on HyperEVM."
        } else {
            "Transactions submitted. Check 'hyperliquid address' in ~30s to verify HYPE balance."
        }
    }));

    Ok(())
}
