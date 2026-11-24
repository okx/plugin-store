use anyhow::Context;
use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;

/// Claim Merkl rewards for the user.
pub async fn run(
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
    confirm: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let user_string = onchainos::resolve_wallet(from, chain_id).await?;
    let user = user_string.as_str();

    // Fetch claimable rewards from Merkl API
    let merkl_data = fetch_merkl_claims(user, chain_id).await?;

    if merkl_data.tokens.is_empty() {
        let output = serde_json::json!({
            "ok": true,
            "operation": "claim-rewards",
            "user": user,
            "chainId": chain_id,
            "message": "No claimable rewards found.",
            "dryRun": dry_run,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Encode Merkl claim calldata
    let claim_calldata = calldata::encode_merkl_claim(
        user,
        &merkl_data.tokens,
        &merkl_data.claimable,
        &merkl_data.proofs,
    );

    // Confirm gate: show preview and exit if --confirm not given
    if !dry_run && !confirm {
        let preview = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "claim-rewards",
            "user": user,
            "chainId": chain_id,
            "rewardTokens": merkl_data.tokens,
            "claimable": merkl_data.claimable,
            "merklDistributor": cfg.merkl_distributor,
            "pendingTransactions": 1,
            "note": "Re-run with --confirm to execute this transaction on-chain."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    eprintln!("[morpho] Claiming {} reward token(s) from Merkl...", merkl_data.tokens.len());
    if dry_run {
        eprintln!("[morpho] [dry-run] Would claim: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, cfg.merkl_distributor, claim_calldata);
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.merkl_distributor,
        &claim_calldata,
        Some(user),
        None,
        dry_run,
        false,
    ).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "claim-rewards",
        "user": user,
        "chainId": chain_id,
        "rewardTokens": merkl_data.tokens,
        "claimable": merkl_data.claimable,
        "merklDistributor": cfg.merkl_distributor,
        "dryRun": dry_run,
        "txHash": tx_hash,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

struct MerklClaims {
    tokens: Vec<String>,
    claimable: Vec<String>,
    proofs: Vec<Vec<String>>,
}

async fn fetch_merkl_claims(user: &str, chain_id: u64) -> anyhow::Result<MerklClaims> {
    let url = format!("https://api.merkl.xyz/v4/claim?user={}&chainId={}", user, chain_id);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Merkl API request failed")?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        // 404 means no rewards for this user — treat as empty
        return Ok(MerklClaims { tokens: vec![], claimable: vec![], proofs: vec![] });
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Merkl API returned {}: {}", status, body);
    }

    let data: serde_json::Value = resp.json().await.context("Merkl API response parse failed")?;

    let mut tokens = Vec::new();
    let mut claimable = Vec::new();
    let mut proofs = Vec::new();

    // Merkl v4 claim response format: array of claim objects
    // Each: { token: "0x...", amount: "...", proofs: ["0x...", ...] }
    if let Some(claims) = data.as_array() {
        for claim in claims {
            let token = claim["token"].as_str().unwrap_or("").to_string();
            let amount = claim["amount"].as_str()
                .or_else(|| claim["claimable"].as_str())
                .unwrap_or("0")
                .to_string();
            let proof_arr: Vec<String> = claim["proofs"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|p| p.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            if !token.is_empty() && amount != "0" {
                tokens.push(token);
                claimable.push(amount);
                proofs.push(proof_arr);
            }
        }
    }

    Ok(MerklClaims { tokens, claimable, proofs })
}
