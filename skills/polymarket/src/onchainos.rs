/// onchainos CLI wrappers for Polymarket on-chain operations.
use anyhow::Result;
use serde_json::Value;

const CHAIN: &str = "137";

/// Call `onchainos wallet contract-call --chain 137 --to <to> --input-data <data> --force`
pub async fn wallet_contract_call(to: &str, input_data: &str) -> Result<Value> {
    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            CHAIN,
            "--to",
            to,
            "--input-data",
            input_data,
            "--force",
        ])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("wallet contract-call parse error: {}\nraw: {}", e, stdout))
}

/// Extract txHash from wallet contract-call response.
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let msg = result["error"]
            .as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        return Err(anyhow::anyhow!("contract-call failed: {}", msg));
    }
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no txHash in contract-call response"))
}

/// Get the wallet address from `onchainos wallet addresses --chain 137`.
/// Parses: data.evm[0].address
pub async fn get_wallet_address() -> Result<String> {
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", CHAIN])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}\nraw: {}", e, stdout))?;
    v["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not determine wallet address from onchainos output"))
}

/// Pad a hex address to 32 bytes (64 hex chars), no 0x prefix.
fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u256 value to 32 bytes (64 hex chars), no 0x prefix.
fn pad_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// ABI-encode and submit USDC.e approve(spender, amount).
/// Selector: 0x095ea7b3
/// To: USDC.e contract
pub async fn usdc_approve(usdc_addr: &str, spender: &str, amount: u128) -> Result<String> {
    let spender_padded = pad_address(spender);
    let amount_padded = pad_u256(amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_padded);
    let result = wallet_contract_call(usdc_addr, &calldata).await?;
    extract_tx_hash(&result)
}

/// ABI-encode and submit CTF setApprovalForAll(operator, true).
/// Selector: 0xa22cb465
/// To: CTF contract
pub async fn ctf_set_approval_for_all(ctf_addr: &str, operator: &str) -> Result<String> {
    let operator_padded = pad_address(operator);
    // approved = true = 1
    let approved_padded = pad_u256(1);
    let calldata = format!("0xa22cb465{}{}", operator_padded, approved_padded);
    let result = wallet_contract_call(ctf_addr, &calldata).await?;
    extract_tx_hash(&result)
}

/// Approve max USDC.e to CTF Exchange. Used before BUY orders.
pub async fn approve_usdc_max(neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    let usdc = Contracts::USDC_E;
    let exchange = Contracts::exchange_for(neg_risk);
    // For true max uint256, we encode manually
    let spender_padded = pad_address(exchange);
    let amount_padded = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_padded);
    let result = wallet_contract_call(usdc, &calldata).await?;
    extract_tx_hash(&result)
}

/// Approve CTF tokens for CTF Exchange. Used before SELL orders.
pub async fn approve_ctf(neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    let ctf = Contracts::CTF;
    let exchange = Contracts::exchange_for(neg_risk);
    ctf_set_approval_for_all(ctf, exchange).await
}

/// ABI-encode and submit CTFExchange.setOperatorApproval(operator, true).
/// Selector: keccak256("setOperatorApproval(address,bool)")[0:4] = 0xa63a1098
pub async fn set_operator_approval(exchange_addr: &str, operator: &str) -> Result<String> {
    // setOperatorApproval(address operator, bool approved)
    // selector = keccak256("setOperatorApproval(address,bool)")[0:4] = 0xa63a1098
    let operator_padded = pad_address(operator);
    let approved_padded = pad_u256(1); // true
    let calldata = format!("0xa63a1098{}{}", operator_padded, approved_padded);
    let result = wallet_contract_call(exchange_addr, &calldata).await?;
    extract_tx_hash(&result)
}

/// Path for caching operator approval state.
fn operator_approval_cache_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".config")
        .join("polymarket")
        .join("operator_approved.json")
}

/// Ensure the local signing key is registered as an operator on CTF Exchange.
/// Calls setOperatorApproval once and caches the result.
pub async fn ensure_operator_approval(
    wallet_addr: &str,
    signer_addr: &str,
    neg_risk: bool,
) -> Result<()> {
    use crate::config::Contracts;

    // If maker == signer, no operator approval needed
    if wallet_addr.to_lowercase() == signer_addr.to_lowercase() {
        return Ok(());
    }

    let cache_path = operator_approval_cache_path();
    if cache_path.exists() {
        let data = std::fs::read_to_string(&cache_path).unwrap_or_default();
        let cached: serde_json::Value = serde_json::from_str(&data).unwrap_or_default();
        let key = format!("{}-{}", wallet_addr.to_lowercase(), signer_addr.to_lowercase());
        if cached.get(&key).and_then(|v| v.as_bool()) == Some(true) {
            return Ok(());
        }
    }

    eprintln!(
        "[polymarket] Setting up operator approval: {} can sign for {}",
        signer_addr, wallet_addr
    );
    let exchange = Contracts::exchange_for(neg_risk);
    match set_operator_approval(exchange, signer_addr).await {
        Ok(tx_hash) => eprintln!("[polymarket] Operator approval tx: {}", tx_hash),
        Err(e) => {
            // setOperatorApproval may revert if already set or contract version differs;
            // warn but proceed — the CLOB API validates signatures independently.
            eprintln!("[polymarket] Warning: operator approval failed ({}). Proceeding anyway.", e);
            return Ok(());
        }
    }

    // Cache approval
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let key = format!("{}-{}", wallet_addr.to_lowercase(), signer_addr.to_lowercase());
    let mut cached: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    cached.insert(key, serde_json::Value::Bool(true));
    let _ = std::fs::write(&cache_path, serde_json::to_string(&cached).unwrap_or_default());

    Ok(()
    )
}
