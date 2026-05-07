use anyhow::Result;

/// Fetch SOL balance in lamports for the given wallet via Solana JSON-RPC.
async fn sol_balance_lamports(wallet: &str) -> u64 {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [wallet]
    });

    let resp = match client
        .post("https://api.mainnet-beta.solana.com")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return 0,
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return 0,
    };

    json["result"]["value"].as_u64().unwrap_or(0)
}

pub async fn run() -> Result<()> {
    let wallet = crate::onchainos::resolve_wallet_solana()?;

    eprintln!(
        "Checking assets for {}... on Solana...",
        &wallet[..8.min(wallet.len())]
    );

    let lamports = sol_balance_lamports(&wallet).await;
    // 1 SOL = 1_000_000_000 lamports
    let sol_balance = lamports as f64 / 1_000_000_000.0;
    let sol_balance_str = format!("{:.6}", sol_balance);

    let threshold = 0.05_f64;
    let (status, suggestion, next_command, onboarding_steps) = if sol_balance >= threshold {
        (
            "ready",
            "Your wallet has SOL. Find a token mint and start trading on pump.fun.",
            "pump-fun-plugin get-token-info --mint <TOKEN_MINT>",
            serde_json::json!([
                "1. Get token info (replace with your token mint):",
                "   pump-fun-plugin get-token-info --mint <TOKEN_MINT>",
                "2. Check buy price (--amount is in lamports; 10000000 = 0.01 SOL):",
                "   pump-fun-plugin get-price --mint <TOKEN_MINT> --direction buy --amount 10000000",
                "3. Preview a buy (no transaction):",
                "   pump-fun-plugin buy --mint <TOKEN_MINT> --sol-amount 0.01",
                "4. Execute when ready:",
                "   pump-fun-plugin buy --mint <TOKEN_MINT> --sol-amount 0.01 --confirm",
                "5. Preview a sell (no transaction):",
                "   pump-fun-plugin sell --mint <TOKEN_MINT> --token-amount <AMOUNT>",
                "6. Execute sell when ready:",
                "   pump-fun-plugin sell --mint <TOKEN_MINT> --token-amount <AMOUNT> --confirm",
                "Note: Find token mints at pump.fun or via search"
            ]),
        )
    } else {
        (
            "no_funds",
            "Your wallet has insufficient SOL. Send SOL to your wallet to start trading.",
            "pump-fun-plugin quickstart",
            serde_json::json!([
                "1. Send SOL to your wallet on Solana mainnet:",
                format!("   {}", wallet),
                "   Minimum recommended: 0.05 SOL (covers fees + minimum buy of 0.01 SOL)",
                "2. Run quickstart again:",
                "   pump-fun-plugin quickstart"
            ]),
        )
    };

    let output = serde_json::json!({
        "ok": true,
        "about": "pump.fun plugin — buy and sell tokens on Solana bonding curves before and after DEX graduation.",
        "wallet": wallet,
        "chain": "Solana",
        "assets": {
            "sol_balance": sol_balance_str
        },
        "status": status,
        "suggestion": suggestion,
        "next_command": next_command,
        "onboarding_steps": onboarding_steps
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
