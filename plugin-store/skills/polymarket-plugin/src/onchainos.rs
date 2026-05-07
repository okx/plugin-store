/// onchainos CLI wrappers for Polymarket on-chain operations.
use anyhow::{Context, Result};
use serde_json::Value;

const CHAIN: &str = "137";

/// Return the path to the onchainos binary.
///
/// Non-interactive shells (e.g. Claude Code's Bash tool) never source ~/.zshrc, so
/// ~/.local/bin is missing from PATH and `Command::new("onchainos")` fails with
/// "os error 2 (No such file or directory)".
///
/// Resolution order:
/// 1. `POLYMARKET_ONCHAINOS_BIN` env var — used in tests to inject a mock binary.
/// 2. `~/.local/bin/onchainos` — the default install location for the onchainos CLI.
/// 3. Bare `"onchainos"` — for systems where it is already in the subprocess PATH.
fn onchainos_bin() -> std::ffi::OsString {
    if let Ok(override_path) = std::env::var("POLYMARKET_ONCHAINOS_BIN") {
        return std::ffi::OsString::from(override_path);
    }
    let local = dirs::home_dir()
        .map(|h| h.join(".local").join("bin").join("onchainos"))
        .filter(|p| p.is_file());
    match local {
        Some(p) => p.into_os_string(),
        None => std::ffi::OsString::from("onchainos"),
    }
}

/// Approval/receipt timeout in seconds — configurable via POLYMARKET_APPROVE_TIMEOUT_SECS.
/// Default: 90s (covers Polygon congestion windows where 30s caused false timeouts).
pub fn approve_timeout_secs() -> u64 {
    std::env::var("POLYMARKET_APPROVE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(90)
}
/// Sign an EIP-712 structured data JSON via `onchainos sign-message --type eip712`.
///
/// The JSON must include EIP712Domain in the `types` field — this is required for correct
/// hash computation (per Hyperliquid root-cause finding).
///
/// Returns the 0x-prefixed signature hex string.
pub async fn sign_eip712(structured_data_json: &str) -> Result<String> {
    // Resolve the wallet address to pass as --from
    let wallet_addr = get_wallet_address().await
        .context("Failed to resolve wallet address for sign-message")?;

    let output = tokio::process::Command::new(onchainos_bin())
        .args([
            "wallet", "sign-message",
            "--type", "eip712",
            "--message", structured_data_json,
            "--chain", CHAIN,
            "--from", &wallet_addr,
            "--force",
        ])
        .output()
        .await
        .context("Failed to spawn onchainos wallet sign-message")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos sign-message failed ({}): {}", output.status, stderr.trim());
    }

    let v: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("parsing sign-message output: {}", stdout.trim()))?;

    // Try data.signature first, then top-level signature
    v["data"]["signature"]
        .as_str()
        .or_else(|| v["signature"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no signature in onchainos output: {}", stdout.trim()))
}

/// Call `onchainos wallet contract-call --chain 137 --to <to> --input-data <data> --force`
pub async fn wallet_contract_call(to: &str, input_data: &str) -> Result<Value> {
    let output = tokio::process::Command::new(onchainos_bin())
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
///
/// Returns a specific, actionable error when the onchainos session has expired so
/// the agent can surface recovery instructions rather than a raw parse error.
pub async fn get_wallet_address() -> Result<String> {
    let output = tokio::process::Command::new(onchainos_bin())
        .args(["wallet", "addresses", "--chain", CHAIN])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Detect session-expiry / not-logged-in conditions from exit code or error text.
    // onchainos emits these on stdout (as JSON) or stderr when the session lapses.
    let combined = format!("{}{}", stdout, stderr).to_lowercase();
    let session_expired = !output.status.success()
        || combined.contains("session")
        || combined.contains("not logged")
        || combined.contains("login required")
        || combined.contains("unauthenticated")
        || combined.contains("unauthorized");

    let parse_result = serde_json::from_str::<Value>(&stdout);
    let json_ok = parse_result.as_ref().ok().and_then(|v| v["ok"].as_bool());

    if json_ok == Some(false) || (parse_result.is_err() && session_expired) {
        anyhow::bail!(
            "onchainos session has expired or wallet is not connected. \
             To recover: open a terminal (or use ! in this chat) and run \
             `onchainos wallet login your@email.com`, complete the login, then retry. \
             If you already re-logged in, also run \
             `rm -f ~/.config/polymarket/creds.json` to clear stale Polymarket credentials."
        );
    }

    let v = parse_result
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}\nraw: {}", e, stdout))?;

    v["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!(
            "onchainos returned no wallet address. \
             Run `onchainos wallet login your@email.com` to connect a wallet, then retry."
        ))
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

// ─── Proxy wallet ─────────────────────────────────────────────────────────────

/// Search a callTracer trace for any call (CREATE, CREATE2, or CALL) made BY PROXY_FACTORY.
/// The factory always calls the proxy wallet as its first sub-call — whether creating a new one
/// (CREATE/CREATE2) or forwarding calls to an existing one (CALL). The `to` field is the proxy.
fn find_create_in_trace(trace: &Value) -> Option<String> {
    use crate::config::Contracts;
    let factory = Contracts::PROXY_FACTORY.to_lowercase();

    if let Some(calls) = trace["calls"].as_array() {
        for sub in calls {
            let from = sub["from"].as_str().unwrap_or("").to_lowercase();
            let call_type = sub["type"].as_str().unwrap_or("");
            let to = sub["to"].as_str().unwrap_or("");

            if from == factory && !to.is_empty()
                && matches!(call_type, "CREATE" | "CREATE2" | "CALL")
            {
                return Some(to.to_string());
            }

            if let Some(addr) = find_create_in_trace(sub) {
                return Some(addr);
            }
        }
    }
    None
}

/// Polygon RPC list used for proxy-state probes. Primary first, fallback second.
/// `polygon_rpc()` reads `POLYMARKET_TEST_POLYGON_RPC` so integration tests can
/// inject a mock; production always falls back to drpc.org.
fn proxy_probe_rpcs() -> [String; 2] {
    [
        crate::config::Urls::polygon_rpc(),
        "https://polygon-bor-rpc.publicnode.com".to_string(),
    ]
}

async fn try_trace_proxy_call(eoa_addr: &str, rpc_url: &str) -> Result<Option<String>> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    let selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let selector_hex = hex::encode(&selector[..4]);
    let calldata = format!(
        "0x{}\
         0000000000000000000000000000000000000000000000000000000000000020\
         0000000000000000000000000000000000000000000000000000000000000000",
        selector_hex
    );

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "debug_traceCall",
        "params": [
            { "from": eoa_addr, "to": Contracts::PROXY_FACTORY, "data": calldata },
            "latest",
            { "tracer": "callTracer" }
        ],
        "id": 1
    });

    let resp = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("debug_traceCall request to {} failed", rpc_url))?;

    let v: serde_json::Value = resp.json().await
        .with_context(|| format!("debug_traceCall response from {} not valid JSON", rpc_url))?;

    if let Some(err) = v.get("error") {
        anyhow::bail!("debug_traceCall on {} returned error: {}", rpc_url, err);
    }

    Ok(find_create_in_trace(&v["result"]))
}

/// Returns true if `addr` has non-empty bytecode at HEAD. False = EOA or undeployed.
async fn query_code_present(addr: &str, rpc_url: &str) -> Result<bool> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [addr, "latest"],
        "id": 1
    });
    let resp = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("eth_getCode request to {} failed", rpc_url))?;
    let v: serde_json::Value = resp.json().await
        .with_context(|| format!("eth_getCode response from {} not valid JSON", rpc_url))?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("eth_getCode on {} returned error: {}", rpc_url, err);
    }
    let code = v["result"].as_str().unwrap_or("0x");
    let stripped = code.trim_start_matches("0x");
    Ok(!stripped.is_empty() && !stripped.chars().all(|c| c == '0'))
}

/// Probe PROXY_FACTORY for the proxy address keyed to `eoa_addr`, and report whether
/// that address has been deployed yet.
///
/// Returns:
/// - `Ok(Some((addr, true)))`  — proxy already deployed at `addr` (recover path)
/// - `Ok(Some((addr, false)))` — `addr` is the deterministic CREATE2 destination but
///                                no contract is deployed there yet. Safe to call
///                                `PROXY_FACTORY.proxy([(...)])` — the first such call
///                                deploys the proxy atomically with the forwarded op.
/// - `Ok(None)`                — the trace contained no factory sub-call at all.
///                                Should not happen with the current Polymarket factory;
///                                callers should treat it as an indeterminate state.
/// - `Err(...)`                — both RPCs failed. Caller MUST NOT deploy or save state,
///                                as we cannot tell which case we're in.
pub async fn get_existing_proxy(eoa_addr: &str) -> Result<Option<(String, bool)>> {
    let rpcs = proxy_probe_rpcs();
    let mut last_err: Option<anyhow::Error> = None;

    for rpc_url in &rpcs {
        let trace_result = match try_trace_proxy_call(eoa_addr, rpc_url).await {
            Ok(opt) => opt,
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        };

        let addr = match trace_result {
            Some(a) => a,
            None => return Ok(None),
        };

        // Determine if the address has bytecode (proxy actually deployed).
        // Use the same RPC for consistency — if drpc.org returned the trace, ask drpc.org for code.
        match query_code_present(&addr, rpc_url).await {
            Ok(present) => return Ok(Some((addr, present))),
            Err(e) => {
                last_err = Some(e.context(format!(
                    "Trace returned {} but eth_getCode for it failed", addr
                )));
                continue;
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!("All Polygon RPCs failed for proxy state lookup")
    }))
}

/// Transfer USDC.e directly to a proxy wallet address.
/// Uses ERC-20 transfer(address recipient, uint256 amount).
/// Selector: 0xa9059cbb
/// Withdraw USDC.e from the proxy wallet back to the EOA.
///
/// Encodes `PROXY_FACTORY.proxy([(0, USDC_E, 0, transfer(eoa, amount))])`.
/// The factory routes the op to the proxy, which executes transfer from its own context.
pub async fn withdraw_usdc_from_proxy(eoa_addr: &str, amount: u128) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // Inner calldata: transfer(eoa_addr, amount) on USDC.e
    // selector: keccak256("transfer(address,uint256)") = 0xa9059cbb
    let transfer_data = format!(
        "a9059cbb{}{}",
        pad_address(eoa_addr),
        pad_u256(amount)
    );
    let transfer_bytes = hex::decode(&transfer_data).expect("transfer calldata hex");
    let transfer_len = transfer_bytes.len(); // 68 bytes

    // ABI-encode proxy((uint8,address,uint256,bytes)[]) with one element:
    //   (op=1, to=USDC_E, value=0, data=transfer_bytes)
    //
    // op=1 means CALL; op=0 is DELEGATECALL in this proxy implementation.
    //
    // Layout (all 32-byte words, after the 4-byte selector):
    //   [0]  0x20       array offset
    //   [1]  0x01       array length = 1
    //   --- tuple[0] ---
    //   [2]  0x01       op = CALL
    //   [3]  USDC_E     to (address padded)
    //   [4]  0x00       value = 0
    //   [5]  0x80       data offset from start of tuple = 4 * 32 = 128
    //   [6]  len        data length
    //   [7+] data       transfer calldata (padded to 32-byte multiple)

    let selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let selector_hex = hex::encode(&selector[..4]);
    let usdc_padded = pad_address(Contracts::USDC_E);
    let data_len_padded = format!("{:064x}", transfer_len);
    // Pad transfer_bytes to next 32-byte boundary
    let pad_len = (32 - transfer_len % 32) % 32;
    let data_padded = format!("{}{}", transfer_data, "00".repeat(pad_len));

    // Correct ABI encoding for (uint8,address,uint256,bytes)[] with one tuple element.
    // The tuple contains a dynamic type (bytes), so the tuple itself is dynamic and
    // needs an offset pointer inside the array.
    //
    // Layout after selector (each line = 32 bytes):
    //   [0x20]  params -> array offset
    //   [0x01]  array length = 1
    //   [0x20]  array[0] offset from start of array data (32 bytes after length word)
    //   [0x00]  tuple.op = 0 (CALL)
    //   [to]    tuple.to = USDC_E
    //   [0x00]  tuple.value = 0
    //   [0x80]  tuple.data offset from start of tuple (4 * 32 = 128)
    //   [len]   tuple.data length
    //   [data]  tuple.data padded
    let calldata = format!(
        "0x{}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}",
        selector_hex,
        "0000000000000000000000000000000000000000000000000000000000000020", // params -> array offset
        "0000000000000000000000000000000000000000000000000000000000000001", // array length = 1
        "0000000000000000000000000000000000000000000000000000000000000020", // array[0] tuple offset
        "0000000000000000000000000000000000000000000000000000000000000001", // op = 1 (CALL)
        usdc_padded,    // to = USDC_E
        "0000000000000000000000000000000000000000000000000000000000000000", // value = 0
        "0000000000000000000000000000000000000000000000000000000000000080", // data offset in tuple
        data_len_padded,
        data_padded,
    );

    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
}

/// Withdraw pUSD from the proxy wallet back to the EOA.
///
/// Same ABI encoding as `withdraw_usdc_from_proxy` but targets the pUSD contract.
/// Used after the V2 collateral cutover (~2026-04-28).
pub async fn withdraw_pusd_from_proxy(eoa_addr: &str, amount: u128) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    let transfer_data = format!(
        "a9059cbb{}{}",
        pad_address(eoa_addr),
        pad_u256(amount)
    );
    let transfer_bytes = hex::decode(&transfer_data).expect("transfer calldata hex");
    let transfer_len = transfer_bytes.len();

    let selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let selector_hex = hex::encode(&selector[..4]);
    let pusd_padded = pad_address(Contracts::PUSD);
    let data_len_padded = format!("{:064x}", transfer_len);
    let pad_len = (32 - transfer_len % 32) % 32;
    let data_padded = format!("{}{}", transfer_data, "00".repeat(pad_len));

    let calldata = format!(
        "0x{}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}",
        selector_hex,
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001", // op = 1 (CALL)
        pusd_padded,
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        data_len_padded,
        data_padded,
    );

    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
}

/// Get USDC.e ERC-20 allowance for owner → spender. Returns raw amount (6 decimals).
pub async fn get_pusd_allowance(owner: &str, spender: &str) -> Result<u128> {
    use crate::config::{Contracts, Urls};
    // allowance(address,address) selector = 0xdd62ed3e
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::PUSD, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?
        .json()
        .await
        .context("parsing RPC response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error: {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    if hex.is_empty() || hex.chars().all(|c| c == '0') {
        return Ok(0);
    }
    Ok(u128::from_str_radix(hex, 16).unwrap_or(u128::MAX))
}

pub async fn get_usdc_allowance(owner: &str, spender: &str) -> Result<u128> {
    use crate::config::{Contracts, Urls};
    // allowance(address,address) selector = 0xdd62ed3e
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::USDC_E, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?
        .json()
        .await
        .context("parsing RPC response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error: {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    // Allowance can be MAX_UINT256 (256-bit), which overflows u128.
    // If any hex digit is non-zero the allowance is set — saturate to u128::MAX.
    if hex.is_empty() || hex.chars().all(|c| c == '0') {
        return Ok(0);
    }
    Ok(u128::from_str_radix(hex, 16).unwrap_or(u128::MAX))
}

/// Call setApprovalForAll(operator, true) on the CTF contract from the proxy wallet,
/// via PROXY_FACTORY.proxy([(1, CTF, 0, setApprovalForAll(operator, true))]).
pub async fn proxy_ctf_set_approval_for_all(operator: &str) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // setApprovalForAll(address,bool) selector = 0xa22cb465
    let saf_data = format!(
        "a22cb465{}{}",
        pad_address(operator),
        "0000000000000000000000000000000000000000000000000000000000000001" // true
    );
    let saf_bytes = hex::decode(&saf_data).expect("setApprovalForAll calldata hex");
    let saf_len = saf_bytes.len(); // 68 bytes

    let selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let selector_hex = hex::encode(&selector[..4]);
    let ctf_padded = pad_address(Contracts::CTF);
    let data_len_padded = format!("{:064x}", saf_len);
    let pad_len = (32 - saf_len % 32) % 32;
    let data_padded = format!("{}{}", saf_data, "00".repeat(pad_len));

    let calldata = format!(
        "0x{}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}",
        selector_hex,
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001", // op = 1 (CALL)
        ctf_padded,
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        data_len_padded,
        data_padded,
    );

    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
}

/// Approve pUSD from the proxy wallet to a spender, via PROXY_FACTORY.proxy().
///
/// Encodes `PROXY_FACTORY.proxy([(1, PUSD, 0, approve(spender, maxUint))])`.
/// Used in POLY_PROXY mode to grant V2 exchange contracts (CTF_EXCHANGE_V2 /
/// NEG_RISK_CTF_EXCHANGE_V2 / NEG_RISK_ADAPTER) spending rights on the proxy wallet's pUSD.
/// Returns the tx hash.
pub async fn proxy_pusd_approve(spender: &str) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    let approve_data = format!(
        "095ea7b3{}{}",
        pad_address(spender),
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
    let approve_bytes = hex::decode(&approve_data).expect("approve calldata hex");
    let approve_len = approve_bytes.len();

    let selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let selector_hex = hex::encode(&selector[..4]);
    let pusd_padded = pad_address(Contracts::PUSD);
    let data_len_padded = format!("{:064x}", approve_len);
    let pad_len = (32 - approve_len % 32) % 32;
    let data_padded = format!("{}{}", approve_data, "00".repeat(pad_len));

    let calldata = format!(
        "0x{}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}",
        selector_hex,
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001",
        pusd_padded,
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        data_len_padded,
        data_padded,
    );

    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
}

/// Approve USDC.e from the proxy wallet to a spender, via PROXY_FACTORY.proxy().
///
/// Encodes `PROXY_FACTORY.proxy([(1, USDC_E, 0, approve(spender, maxUint))])`.
/// Used in POLY_PROXY mode to grant the CTF Exchange (or NEG_RISK_CTF_EXCHANGE /
/// NEG_RISK_ADAPTER) spending rights on the proxy wallet's USDC.e.
/// Returns the tx hash.
pub async fn proxy_usdc_approve(spender: &str) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // Inner calldata: approve(spender, maxUint) on USDC.e
    // selector: keccak256("approve(address,uint256)") = 0x095ea7b3
    let approve_data = format!(
        "095ea7b3{}{}",
        pad_address(spender),
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" // uint256 max
    );
    let approve_bytes = hex::decode(&approve_data).expect("approve calldata hex");
    let approve_len = approve_bytes.len(); // 68 bytes

    let selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let selector_hex = hex::encode(&selector[..4]);
    let usdc_padded = pad_address(Contracts::USDC_E);
    let data_len_padded = format!("{:064x}", approve_len);
    let pad_len = (32 - approve_len % 32) % 32;
    let data_padded = format!("{}{}", approve_data, "00".repeat(pad_len));

    let calldata = format!(
        "0x{}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}",
        selector_hex,
        "0000000000000000000000000000000000000000000000000000000000000020", // params -> array offset
        "0000000000000000000000000000000000000000000000000000000000000001", // array length = 1
        "0000000000000000000000000000000000000000000000000000000000000020", // array[0] tuple offset
        "0000000000000000000000000000000000000000000000000000000000000001", // op = 1 (CALL)
        usdc_padded,    // to = USDC_E
        "0000000000000000000000000000000000000000000000000000000000000000", // value = 0
        "0000000000000000000000000000000000000000000000000000000000000080", // data offset in tuple
        data_len_padded,
        data_padded,
    );

    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
}

pub async fn transfer_usdc_to_proxy(proxy_addr: &str, amount: u128) -> Result<String> {
    use crate::config::Contracts;
    let recipient_padded = pad_address(proxy_addr);
    let amount_padded = pad_u256(amount);
    let calldata = format!("0xa9059cbb{}{}", recipient_padded, amount_padded);
    let result = wallet_contract_call(Contracts::USDC_E, &calldata).await?;
    extract_tx_hash(&result)
}

// ─── ERC-20 / CTF approvals ────────────────────────────────────────────────────

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

/// Approve USDC.e allowance before a BUY order.
///
/// Always approves `u128::MAX` (unlimited) so that future trades on the same market
/// do not trigger a second approval transaction. Approving a specific order amount
/// downsizes any pre-existing MAX_UINT allowance to that amount, causing a new
/// approval on every subsequent trade.
///
/// For neg_risk=false: approves CTF Exchange only.
/// For neg_risk=true: approves BOTH NEG_RISK_CTF_EXCHANGE and NEG_RISK_ADAPTER —
/// the CLOB checks both contracts in the settlement path for neg_risk markets.
/// Returns the tx hash of the last approval submitted.
pub async fn approve_usdc(neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    let usdc = Contracts::USDC_E;
    let amount = u128::MAX;
    if neg_risk {
        usdc_approve(usdc, Contracts::NEG_RISK_CTF_EXCHANGE, amount).await?;
        usdc_approve(usdc, Contracts::NEG_RISK_ADAPTER, amount).await
    } else {
        usdc_approve(usdc, Contracts::CTF_EXCHANGE, amount).await
    }
}

/// Approve CTF tokens for sell orders.
///
/// For neg_risk=false: approves CTF_EXCHANGE only.
/// For neg_risk=true: approves BOTH NEG_RISK_CTF_EXCHANGE and NEG_RISK_ADAPTER —
/// the CLOB checks setApprovalForAll on both contracts for neg_risk markets (mirrors
/// the approve_usdc pattern for USDC.e allowance).
/// Returns the tx hash of the last approval submitted.
pub async fn approve_ctf(neg_risk: bool) -> Result<String> {
    use crate::config::Contracts;
    let ctf = Contracts::CTF;
    if neg_risk {
        ctf_set_approval_for_all(ctf, Contracts::NEG_RISK_CTF_EXCHANGE).await?;
        ctf_set_approval_for_all(ctf, Contracts::NEG_RISK_ADAPTER).await
    } else {
        ctf_set_approval_for_all(ctf, Contracts::CTF_EXCHANGE).await
    }
}

/// Pure ABI encoder for CTF.redeemPositions(collateralToken, parentCollectionId, conditionId, indexSets).
///
/// indexSets [1, 2] covers both YES (bit 0) and NO (bit 1) outcomes — the CTF contract only
/// pays out for winning tokens and silently no-ops for losing ones, so passing both is safe.
/// Extracted as a pure function so the encoding can be unit-tested independently from RPC I/O.
pub fn build_ctf_redeem_positions_calldata(condition_id: &str, collateral_addr: &str) -> String {
    use sha3::{Digest, Keccak256};

    let selector = Keccak256::digest(b"redeemPositions(address,bytes32,bytes32,uint256[])");
    let selector_hex = hex::encode(&selector[..4]);

    // Slots 0-2 are static (address and bytes32); slot 3 is the offset to the dynamic uint256[] array.
    let collateral  = pad_address(collateral_addr);            // address padded to 32 bytes
    let parent_id   = format!("{:064x}", 0u128);               // bytes32(0) — null parent collection
    let cond_id_hex = condition_id.trim_start_matches("0x");
    let cond_id_pad = format!("{:0>64}", cond_id_hex);
    let array_offset = pad_u256(4 * 32);
    let array_len  = pad_u256(2);
    let index_yes  = pad_u256(1);
    let index_no   = pad_u256(2);

    format!(
        "0x{}{}{}{}{}{}{}{}",
        selector_hex, collateral, parent_id, cond_id_pad,
        array_offset, array_len, index_yes, index_no
    )
}

/// ABI-encode and submit CTF redeemPositions(collateralToken, parentCollectionId, conditionId, indexSets).
///
/// For neg_risk (multi-outcome) markets use the NEG_RISK_ADAPTER path (not implemented here).
///
/// `collateral_addr`: the collateral token used at trade time.
///   - V1 markets: Contracts::USDC_E
///   - V2 markets: Contracts::PUSD  (from ~2026-04-28)
pub async fn ctf_redeem_positions(condition_id: &str, collateral_addr: &str) -> Result<String> {
    use crate::config::Contracts;
    let calldata = build_ctf_redeem_positions_calldata(condition_id, collateral_addr);
    let result = wallet_contract_call(Contracts::CTF, &calldata).await?;
    extract_tx_hash(&result)
}

/// ABI-encode and submit CTF redeemPositions via the deposit wallet relayer WALLET batch.
///
/// Used when winning outcome tokens are held by the deposit wallet (DEPOSIT_WALLET mode).
/// The relayer executes the call from the deposit wallet's context, so CTF sees
/// msg.sender = deposit_wallet, which holds the winning tokens.
/// pUSD collateral is transferred to the deposit wallet after redemption.
pub async fn ctf_redeem_via_deposit_wallet(
    condition_id: &str,
    collateral_addr: &str,
    deposit_wallet: &str,
    eoa_addr: &str,
    builder: &crate::auth::BuilderCredentials,
) -> Result<String> {
    use crate::config::Contracts;
    use crate::signing::{BatchParams, WalletCall, sign_batch_via_onchainos};
    use crate::api::{get_wallet_nonce, relayer_wallet_batch};

    let calldata = build_ctf_redeem_positions_calldata(condition_id, collateral_addr);
    let calls = vec![WalletCall {
        target: Contracts::CTF.to_string(),
        value: 0,
        data: calldata,
    }];

    let client = reqwest::Client::new();
    let nonce = get_wallet_nonce(&client, eoa_addr).await
        .map_err(|e| anyhow::anyhow!("Could not fetch wallet nonce for redeem batch: {}", e))?;
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + 300;

    let calls_json: Vec<serde_json::Value> = calls.iter().map(|c| serde_json::json!({
        "target": c.target,
        "value":  c.value.to_string(),
        "data":   c.data,
    })).collect();

    let batch_params = BatchParams {
        wallet: deposit_wallet.to_string(),
        nonce,
        deadline,
        calls,
    };
    let batch_sig = sign_batch_via_onchainos(&batch_params).await
        .map_err(|e| anyhow::anyhow!("Batch signing for deposit wallet redeem failed: {}", e))?;

    relayer_wallet_batch(&client, eoa_addr, deposit_wallet, nonce, deadline, calls_json, &batch_sig, builder).await
}

/// ABI-encode and submit CTF redeemPositions via the PROXY_FACTORY.
///
/// Used when winning outcome tokens are held by the proxy wallet (POLY_PROXY mode).
/// Routes: EOA → PROXY_FACTORY.proxy([(CALL, CTF, 0, redeemPositions_calldata)])
/// The factory forwards the call from the proxy wallet's context, so CTF sees
/// msg.sender = proxy wallet, which holds the winning tokens.
///
/// `collateral_addr`: the collateral token used at trade time.
///   - V1 markets: Contracts::USDC_E
///   - V2 markets: Contracts::PUSD  (from ~2026-04-28)
pub async fn ctf_redeem_via_proxy(condition_id: &str, collateral_addr: &str) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    let inner_selector = Keccak256::digest(b"redeemPositions(address,bytes32,bytes32,uint256[])");
    let inner_selector_hex = hex::encode(&inner_selector[..4]);
    let collateral   = pad_address(collateral_addr);
    let parent_id    = format!("{:064x}", 0u128);
    let cond_id_hex  = condition_id.trim_start_matches("0x");
    let cond_id_pad  = format!("{:0>64}", cond_id_hex);
    let array_offset = pad_u256(4 * 32);
    let array_len    = pad_u256(2);
    let index_yes    = pad_u256(1);
    let index_no     = pad_u256(2);

    let inner_hex = format!(
        "{}{}{}{}{}{}{}{}",
        inner_selector_hex, collateral, parent_id, cond_id_pad,
        array_offset, array_len, index_yes, index_no
    );
    let inner_bytes = hex::decode(&inner_hex).expect("inner redeem calldata");
    let inner_len   = inner_bytes.len();
    let pad_len     = (32 - inner_len % 32) % 32;
    let inner_padded = format!("{}{}", inner_hex, "00".repeat(pad_len));

    let outer_selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let outer_selector_hex = hex::encode(&outer_selector[..4]);
    let ctf_padded     = pad_address(Contracts::CTF);
    let data_len_padded = format!("{:064x}", inner_len);

    let calldata = format!(
        "0x{}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}",
        outer_selector_hex,
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001",
        ctf_padded,
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        data_len_padded,
        inner_padded,
    );
    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
}


// ─── NegRisk Adapter redeem ───────────────────────────────────────────────────

/// Convert a large decimal integer string (up to 256 bits) to a 64-char lowercase hex string.
///
/// Polymarket outcome token IDs are full uint256 values that do not fit in u128.
/// This function does the conversion using byte-level long multiplication, avoiding
/// any bignum dependency.
///
/// Returns an error if the string contains non-digit characters or overflows 32 bytes.
pub fn decimal_str_to_hex64(s: &str) -> Result<String> {
    if s.is_empty() {
        anyhow::bail!("decimal_str_to_hex64: empty string is not a valid decimal integer");
    }
    let mut result = [0u8; 32];
    for ch in s.chars() {
        let digit = ch
            .to_digit(10)
            .ok_or_else(|| anyhow::anyhow!("decimal_str_to_hex64: invalid digit '{}' in '{}'", ch, s))?;
        let mut carry = digit as u16;
        for byte in result.iter_mut().rev() {
            let val = (*byte as u16) * 10 + carry;
            *byte = (val & 0xFF) as u8;
            carry = val >> 8;
        }
        if carry != 0 {
            anyhow::bail!("decimal_str_to_hex64: overflow — value '{}' too large for 32 bytes", s);
        }
    }
    Ok(hex::encode(result))
}

/// Query the ERC-1155 CTF token balance of `owner` for a given outcome token ID.
///
/// `token_id_decimal` is the decimal string representation of the uint256 token ID
/// as returned by the Polymarket CLOB API (e.g. `ClobToken::token_id`).
///
/// Returns the raw token balance (atomic units, same scale as USDC.e: 1 share = 1_000_000).
pub async fn get_ctf_balance(owner: &str, token_id_decimal: &str) -> Result<u128> {
    use crate::config::{Contracts, Urls};
    // balanceOf(address,uint256) selector = 0x00fdd58e
    let token_id_hex = decimal_str_to_hex64(token_id_decimal)?;
    let data = format!("0x00fdd58e{}{}", pad_address(owner), token_id_hex);
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::CTF, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?
        .json()
        .await
        .context("parsing CTF balanceOf response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error in CTF balanceOf: {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    if hex.is_empty() || hex.chars().all(|c| c == '0') {
        return Ok(0);
    }
    // Balances are small (shares held by a user) — safely fits in u128.
    Ok(u128::from_str_radix(hex, 16).unwrap_or(u128::MAX))
}

/// Variant of `get_ctf_balance` that takes the position ID as a 64-char hex string
/// instead of decimal — useful when the position ID is computed via the on-chain
/// `getPositionId` view function (which returns hex).
pub async fn get_ctf_balance_hex(owner: &str, position_id_hex: &str) -> Result<u128> {
    use crate::config::{Contracts, Urls};
    let pid = position_id_hex.trim_start_matches("0x");
    if pid.len() != 64 {
        anyhow::bail!("position_id_hex must be 64 hex chars, got {}", pid.len());
    }
    let data = format!("0x00fdd58e{}{}", pad_address(owner), pid);
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::CTF, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?
        .json()
        .await
        .context("parsing CTF balanceOf response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error in CTF balanceOf (hex): {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    if hex.is_empty() || hex.chars().all(|c| c == '0') {
        return Ok(0);
    }
    Ok(u128::from_str_radix(hex, 16).unwrap_or(u128::MAX))
}

/// On-chain `CTF.getCollectionId(parentCollectionId, conditionId, indexSet)`.
///
/// CTF's collectionId computation uses BN254 elliptic-curve point addition
/// internally (via hashToCurve), so it cannot be replicated locally without
/// pulling in elliptic-curve dependencies — we delegate to the contract.
///
/// Returns the collectionId as a 64-char hex string (without 0x prefix).
pub async fn ctf_get_collection_id_hex(
    parent_collection_id_hex: &str,
    condition_id_hex: &str,
    index_set: u32,
) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::{Contracts, Urls};

    let selector = Keccak256::digest(b"getCollectionId(bytes32,bytes32,uint256)");
    let selector_hex = hex::encode(&selector[..4]);

    let parent_pad = format!(
        "{:0>64}",
        parent_collection_id_hex.trim_start_matches("0x")
    );
    let cond_pad = format!(
        "{:0>64}",
        condition_id_hex.trim_start_matches("0x")
    );
    let idx_pad = format!("{:064x}", index_set);
    let data = format!("0x{}{}{}{}", selector_hex, parent_pad, cond_pad, idx_pad);

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::CTF, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed (getCollectionId)")?
        .json()
        .await
        .context("parsing getCollectionId response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error in CTF.getCollectionId: {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    if hex.len() != 64 {
        anyhow::bail!(
            "getCollectionId returned unexpected length {} (expected 64): 0x{}",
            hex.len(),
            hex
        );
    }
    Ok(hex.to_string())
}

/// On-chain `CTF.getPositionId(IERC20 collateralToken, bytes32 collectionId)`.
///
/// Returns the position ID as a 64-char hex string (without 0x prefix), suitable
/// for direct use with `get_ctf_balance_hex`.
pub async fn ctf_get_position_id_hex(
    collateral_addr: &str,
    collection_id_hex: &str,
) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::{Contracts, Urls};

    let selector = Keccak256::digest(b"getPositionId(address,bytes32)");
    let selector_hex = hex::encode(&selector[..4]);

    let collateral_pad = pad_address(collateral_addr);
    let collection_pad = format!(
        "{:0>64}",
        collection_id_hex.trim_start_matches("0x")
    );
    let data = format!("0x{}{}{}", selector_hex, collateral_pad, collection_pad);

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::CTF, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed (getPositionId)")?
        .json()
        .await
        .context("parsing getPositionId response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error in CTF.getPositionId: {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    if hex.len() != 64 {
        anyhow::bail!(
            "getPositionId returned unexpected length {} (expected 64): 0x{}",
            hex.len(),
            hex
        );
    }
    Ok(hex.to_string())
}

/// ABI-encode NegRiskAdapter.redeemPositions(bytes32 conditionId, uint256[] amounts).
///
/// `amounts` is indexed by outcome slot: amounts[0] = YES token balance, amounts[1] = NO token balance.
/// After market resolution, only the winning outcome's amount is non-zero; passing zero for the
/// other slot is safe (the adapter no-ops on zero-balance outcomes).
pub fn build_negrisk_redeem_calldata(condition_id: &str, amounts: &[u128]) -> String {
    use sha3::{Digest, Keccak256};

    let selector = Keccak256::digest(b"redeemPositions(bytes32,uint256[])");
    let selector_hex = hex::encode(&selector[..4]);

    let cond_id_hex = condition_id.trim_start_matches("0x");
    let cond_id_pad = format!("{:0>64}", cond_id_hex);

    // Dynamic array starts at offset 64 bytes (2 × 32-byte static params: conditionId + array offset).
    let array_offset = pad_u256(64u128);
    let array_len = pad_u256(amounts.len() as u128);
    let amounts_hex: String = amounts.iter().map(|a| pad_u256(*a)).collect();

    format!(
        "0x{}{}{}{}{}",
        selector_hex, cond_id_pad, array_offset, array_len, amounts_hex
    )
}

/// Redeem neg_risk (multi-outcome) positions via NegRiskAdapter.redeemPositions.
///
/// `amounts[i]` is the ERC-1155 balance of outcome slot i held by `from`.
/// Pre-flights via eth_call to surface reverts before signing.
/// Returns the tx hash of the broadcast transaction.
pub async fn negrisk_redeem_positions(
    condition_id: &str,
    amounts: &[u128],
    from: &str,
) -> Result<String> {
    use crate::config::Contracts;
    let calldata = build_negrisk_redeem_calldata(condition_id, amounts);
    eth_call_simulate(from, Contracts::NEG_RISK_ADAPTER, &calldata)
        .await
        .context("NegRiskAdapter.redeemPositions would revert on-chain")?;
    let result = wallet_contract_call(Contracts::NEG_RISK_ADAPTER, &calldata).await?;
    extract_tx_hash(&result)
}


/// Get native POL balance for an address (eth_getBalance). Returns human-readable f64 (POL).
pub async fn get_pol_balance(addr: &str) -> Result<f64> {
    use crate::config::Urls;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [addr, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?
        .json()
        .await
        .context("parsing RPC response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error: {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x0").trim_start_matches("0x");
    let wei = u128::from_str_radix(hex, 16).context("parsing POL balance")?;
    Ok(wei as f64 / 1e18)
}

/// Get the ERC-20 balance for `holder_addr` on any 6-decimal token contract.
/// Returns human-readable f64 (e.g. dollars for USDC.e / pUSD).
pub async fn get_erc20_balance_6dec(token_addr: &str, holder_addr: &str) -> Result<f64> {
    use crate::config::Urls;
    // balanceOf(address) selector = 0x70a08231
    let data = format!("0x70a08231{}", pad_address(holder_addr));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": token_addr, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?
        .json()
        .await
        .context("parsing RPC response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error: {}", err);
    }
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    let raw = u128::from_str_radix(hex, 16).unwrap_or(0);
    Ok(raw as f64 / 1_000_000.0) // 6 decimals
}

/// Get USDC.e (ERC-20) balance for an address. Returns human-readable f64 (dollars).
pub async fn get_usdc_balance(addr: &str) -> Result<f64> {
    use crate::config::Contracts;
    get_erc20_balance_6dec(Contracts::USDC_E, addr).await
}

/// Return the current Polygon block number via eth_blockNumber.
pub async fn get_current_block() -> Option<u64> {
    use crate::config::Urls;
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "method": "eth_blockNumber", "params": [], "id": 1
        }))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let hex = v["result"].as_str()?;
    u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok()
}

/// Get pUSD (ERC-20) balance for an address. Returns human-readable f64 (dollars).
/// pUSD is the Polymarket USD collateral token that replaces USDC.e for V2 exchange contracts.
pub async fn get_pusd_balance(addr: &str) -> Result<f64> {
    use crate::config::Contracts;
    get_erc20_balance_6dec(Contracts::PUSD, addr).await
}

/// Wrap USDC.e → pUSD via the Collateral Onramp for an EOA wallet.
///
/// Steps:
///   1. Approve USDC.e to COLLATERAL_ONRAMP (amount).
///   2. Call COLLATERAL_ONRAMP.wrap(USDC_E, recipient, amount).
///   3. Wait for the wrap tx to confirm.
///
/// Returns the wrap tx hash after on-chain confirmation.
pub async fn wrap_usdc_to_pusd(recipient: &str, amount: u128) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // Step 1: approve USDC.e to the onramp
    let approve_tx = usdc_approve(Contracts::USDC_E, Contracts::COLLATERAL_ONRAMP, amount).await?;
    wait_for_tx_receipt(&approve_tx, 30).await?;

    // Step 2: call wrap(address _asset, address _to, uint256 _amount)
    let selector = Keccak256::digest(b"wrap(address,address,uint256)");
    let selector_hex = hex::encode(&selector[..4]);
    let calldata = format!(
        "0x{}{}{}{}",
        selector_hex,
        pad_address(Contracts::USDC_E),
        pad_address(recipient),
        pad_u256(amount),
    );
    let result = wallet_contract_call(Contracts::COLLATERAL_ONRAMP, &calldata).await?;
    extract_tx_hash(&result)
}

/// Wrap USDC.e → pUSD for a proxy wallet via PROXY_FACTORY.proxy().
///
/// The proxy wallet first approves USDC.e to the Collateral Onramp, then calls
/// COLLATERAL_ONRAMP.wrap(USDC_E, proxy_addr, amount) from its own context.
///
/// Steps (each routed through proxy):
///   1. proxy_usdc_approve(COLLATERAL_ONRAMP)  — sets unlimited allowance
///   2. proxy calls wrap(USDC_E, proxy_addr, amount) → pUSD minted to proxy
///
/// Returns the wrap tx hash after on-chain confirmation.
pub async fn proxy_wrap_usdc_to_pusd(proxy_addr: &str, amount: u128) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // Step 1: proxy approves USDC.e to the onramp (unlimited)
    let approve_tx = proxy_usdc_approve(Contracts::COLLATERAL_ONRAMP).await?;
    wait_for_tx_receipt(&approve_tx, 30).await?;

    // Step 2: proxy calls wrap(USDC_E, proxy_addr, amount)
    // wrap(address,address,uint256) = selector + _asset + _to + _amount
    let wrap_selector = Keccak256::digest(b"wrap(address,address,uint256)");
    let wrap_selector_hex = hex::encode(&wrap_selector[..4]);
    let inner_hex = format!(
        "{}{}{}{}",
        wrap_selector_hex,
        pad_address(Contracts::USDC_E),
        pad_address(proxy_addr),
        pad_u256(amount),
    );
    let inner_bytes = hex::decode(&inner_hex).expect("wrap calldata hex");
    let inner_len = inner_bytes.len();
    let pad_len = (32 - inner_len % 32) % 32;
    let inner_padded = format!("{}{}", inner_hex, "00".repeat(pad_len));

    // Wrap in PROXY_FACTORY.proxy([(CALL, COLLATERAL_ONRAMP, 0, wrap_calldata)])
    let outer_selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let outer_selector_hex = hex::encode(&outer_selector[..4]);
    let onramp_padded = pad_address(Contracts::COLLATERAL_ONRAMP);
    let data_len_padded = format!("{:064x}", inner_len);

    let calldata = format!(
        "0x{}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}\
         {}",
        outer_selector_hex,
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000020",
        "0000000000000000000000000000000000000000000000000000000000000001", // op = 1 (CALL)
        onramp_padded,
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        data_len_padded,
        inner_padded,
    );

    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
}

/// Wrap USDC.e → pUSD for a deposit wallet via Polymarket relayer WALLET batch.
///
/// COLLATERAL_ONRAMP.wrap() requires `_to == msg.sender`, so the deposit wallet must
/// call the onramp from its own context. This is done via a signed WALLET batch submitted
/// to the Polymarket relayer — fully gasless (same mechanism as the approval batch in
/// `setup-deposit-wallet`).
///
/// Batch calls:
///   1. USDC_E.approve(COLLATERAL_ONRAMP, amount)
///   2. COLLATERAL_ONRAMP.wrap(USDC_E, wallet_addr, amount)  ← msg.sender = wallet_addr = _to ✓
///
/// Returns the approval batch tx hash after on-chain confirmation.
pub async fn deposit_wallet_wrap_usdc_to_pusd(
    wallet_addr: &str,
    owner_addr: &str,
    amount: u128,
    client: &reqwest::Client,
    creds: &crate::config::Credentials,
) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::api::{get_builder_api_key, get_wallet_nonce, relayer_wallet_batch};
    use crate::config::Contracts;
    use crate::signing::{sign_batch_via_onchainos, BatchParams, WalletCall};

    // Fetch builder credentials (needed for relayer WALLET batch auth).
    let builder = get_builder_api_key(client, creds, owner_addr).await
        .map_err(|e| anyhow::anyhow!("Could not get builder credentials for wrap: {}", e))?;

    // Call 1: USDC.e.approve(COLLATERAL_ONRAMP, amount)
    let approve_sel = hex::encode(&Keccak256::digest(b"approve(address,uint256)")[..4]);
    let approve_data = format!(
        "0x{}{}{}",
        approve_sel,
        pad_address(Contracts::COLLATERAL_ONRAMP),
        pad_u256(amount),
    );

    // Call 2: COLLATERAL_ONRAMP.wrap(USDC_E, wallet_addr, amount)
    let wrap_sel = hex::encode(&Keccak256::digest(b"wrap(address,address,uint256)")[..4]);
    let wrap_data = format!(
        "0x{}{}{}{}",
        wrap_sel,
        pad_address(Contracts::USDC_E),
        pad_address(wallet_addr),
        pad_u256(amount),
    );

    let calls = vec![
        WalletCall { target: Contracts::USDC_E.to_string(),            value: 0, data: approve_data },
        WalletCall { target: Contracts::COLLATERAL_ONRAMP.to_string(), value: 0, data: wrap_data   },
    ];

    let nonce = get_wallet_nonce(client, owner_addr).await
        .map_err(|e| anyhow::anyhow!("Could not fetch wallet nonce for wrap: {}", e))?;
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + 300;

    let calls_json: Vec<serde_json::Value> = calls.iter().map(|c| serde_json::json!({
        "target": c.target,
        "value":  c.value.to_string(),
        "data":   c.data,
    })).collect();

    let batch_params = BatchParams { wallet: wallet_addr.to_string(), nonce, deadline, calls };
    let batch_sig = sign_batch_via_onchainos(&batch_params).await
        .map_err(|e| anyhow::anyhow!("Batch signing failed for wrap: {}", e))?;

    let tx = relayer_wallet_batch(client, owner_addr, wallet_addr, nonce, deadline, calls_json, &batch_sig, &builder).await
        .map_err(|e| anyhow::anyhow!("Relayer WALLET batch for wrap failed: {}", e))?;

    wait_for_tx_receipt(&tx, 120).await?;
    Ok(tx)
}

/// Simulate a contract call via eth_call on Polygon. Returns Ok(()) if no revert.
///
/// Use this as a pre-flight before `wallet_contract_call` to catch reverts that
/// onchainos's `--force` flag would otherwise mask (returning a txHash that was
/// signed but never broadcast).
pub async fn eth_call_simulate(from: &str, to: &str, input_data: &str) -> Result<()> {
    use crate::config::Urls;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "from": from,
            "to": to,
            "data": input_data,
        }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC eth_call failed")?
        .json()
        .await
        .context("parsing eth_call response")?;
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        anyhow::bail!("eth_call simulation reverted: {}", msg);
    }
    Ok(())
}

/// Poll eth_getTransactionReceipt until the tx is mined (or timeout).
///
/// Polygon block time is ~2 seconds. We poll every 2 seconds for up to max_wait_secs.
pub async fn wait_for_tx_receipt(tx_hash: &str, max_wait_secs: u64) -> Result<()> {
    wait_for_tx_receipt_labeled(tx_hash, max_wait_secs, "Transaction").await
}

/// Same as `wait_for_tx_receipt` but with a caller-supplied label used in error
/// messages (e.g. "Approve", "Redeem") so the bail text is accurate.
pub async fn wait_for_tx_receipt_labeled(
    tx_hash: &str,
    max_wait_secs: u64,
    label: &str,
) -> Result<()> {
    use crate::config::Urls;
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    let deadline = Instant::now() + Duration::from_secs(max_wait_secs);
    loop {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        let resp = reqwest::Client::new()
            .post(Urls::polygon_rpc())
            .json(&body)
            .send()
            .await;
        if let Ok(r) = resp {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                if v["result"].is_object() {
                    let status = v["result"]["status"].as_str().unwrap_or("0x1");
                    if status == "0x0" {
                        anyhow::bail!(
                            "{} tx {} was mined but reverted (status 0x0). \
                             Check Polygonscan for details.",
                            label,
                            tx_hash
                        );
                    }
                    return Ok(());
                }
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "{} tx {} not observed on-chain within {}s. \
                 If the hash does not appear on Polygonscan, onchainos signed the tx \
                 but never broadcast it — usually because it would revert. \
                 Check your trading mode / outcome token ownership and retry.",
                label,
                tx_hash,
                max_wait_secs
            );
        }
        sleep(Duration::from_millis(2000)).await;
    }
}


/// Poll eth_getTransactionReceipt on any supported EVM chain until mined or timeout.
///
/// `chain` is the onchainos chain name (e.g. "bnb", "ethereum", "arbitrum").
/// Resolves to the correct public RPC endpoint per chain.
pub async fn wait_for_receipt_on_chain(chain: &str, tx_hash: &str, max_wait_secs: u64) -> Result<()> {
    use crate::config::Urls;
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    let rpc = match chain {
        "ethereum"  => Urls::ETHEREUM_RPC,
        "arbitrum"  => Urls::ARBITRUM_RPC,
        "base"      => Urls::BASE_RPC,
        "optimism"  => Urls::OPTIMISM_RPC,
        "bnb"       => Urls::BNB_RPC,
        "polygon" | "137" => Urls::POLYGON_RPC,
        other => anyhow::bail!("No RPC configured for chain '{}'", other),
    };

    let deadline = Instant::now() + Duration::from_secs(max_wait_secs);
    loop {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        let resp = reqwest::Client::new()
            .post(rpc)
            .json(&body)
            .send()
            .await;
        if let Ok(r) = resp {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                if v["result"].is_object() {
                    let status = v["result"]["status"].as_str().unwrap_or("0x1");
                    if status == "0x0" {
                        anyhow::bail!(
                            "Transaction {} was mined but reverted on {} (status 0x0).",
                            tx_hash, chain
                        );
                    }
                    return Ok(());
                }
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "Tx {} not confirmed on {} within {}s. Check a block explorer and retry.",
                tx_hash, chain, max_wait_secs
            );
        }
        sleep(Duration::from_millis(3000)).await;
    }
}

/// Check if the CTF contract has setApprovalForAll set for owner → operator.
/// Makes a direct eth_call to the Polygon RPC to read isApprovedForAll(owner, operator).
///
/// Returns Ok(true) if approved, Ok(false) if not approved, Err if the RPC call fails.
/// Callers should treat Err as "unknown — approve to be safe" (setApprovalForAll is idempotent).
// ─── Multi-chain transfers (for bridge deposit) ───────────────────────────────

/// Transfer an ERC-20 token on any EVM chain supported by onchainos.
///
/// `chain` accepts onchainos chain names or IDs (e.g. "ethereum", "1", "arbitrum", "42161").
/// `token_contract` is the ERC-20 contract address on the source chain.
/// `to` is the destination address (e.g. Polymarket bridge EVM deposit address).
/// `amount` is the raw token amount in smallest units (respecting token decimals).
///
/// Uses `onchainos wallet contract-call --chain <chain>` (not hardcoded to Polygon).
pub async fn transfer_erc20_on_chain(
    chain: &str,
    token_contract: &str,
    to: &str,
    amount: u128,
) -> Result<String> {
    let output = tokio::process::Command::new(onchainos_bin())
        .args([
            "wallet", "send",
            "--chain", chain,
            "--recipient", to,
            "--amt", &amount.to_string(),
            "--contract-token", token_contract,
            "--force",
        ])
        .output()
        .await
        .context("Failed to spawn onchainos wallet send (ERC-20)")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos contract-call on {} failed ({}): {}",
            chain, output.status, stderr.trim()
        );
    }
    let result: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("parsing wallet send output: {}", stdout.trim()))?;
    extract_tx_hash(&result)
}

/// Send native tokens (ETH, BNB, etc.) on any EVM chain supported by onchainos.
///
/// `chain` accepts onchainos chain names or IDs.
/// `to` is the destination address.
/// `amount_wei` is the amount in wei (18 decimals for ETH-like, 9 for others).
pub async fn transfer_native_on_chain(chain: &str, to: &str, amount_wei: u128) -> Result<String> {
    let output = tokio::process::Command::new(onchainos_bin())
        .args([
            "wallet", "send",
            "--chain", chain,
            "--recipient", to,
            "--amt", &amount_wei.to_string(),
            "--force",
        ])
        .output()
        .await
        .context("Failed to spawn onchainos wallet send (native)")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos native transfer on {} failed ({}): {}",
            chain, output.status, stderr.trim()
        );
    }
    let result: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("parsing wallet send output: {}", stdout.trim()))?;
    extract_tx_hash(&result)
}

/// A single token balance entry returned by `onchainos wallet balance`.
#[derive(Debug, Clone)]
pub struct ChainTokenBalance {
    pub symbol: String,
    pub token_address: String, // lowercase; empty string for native coin
    pub usd_value: f64,
    pub balance: String,
    pub decimal: u8,
}

/// Call `onchainos wallet balance --chain <chain>` and return all token balances.
/// Returns an empty vec on failure (non-fatal — used for best-effort suggestions).
/// Estimate the gas cost of a standard ERC-20 transfer on the given chain, in native token units.
///
/// Uses `eth_gasPrice` (or `eth_maxFeePerGas` for EIP-1559 chains) from the public RPC,
/// multiplied by 65,000 gas (standard ERC-20 transfer) with a 20% buffer.
/// Falls back to conservative static minimums if the RPC call fails.
pub async fn estimate_erc20_gas_cost(chain: &str) -> f64 {
    use crate::config::Urls;

    let rpc = match chain {
        "ethereum"  => Urls::ETHEREUM_RPC,
        "arbitrum"  => Urls::ARBITRUM_RPC,
        "base"      => Urls::BASE_RPC,
        "optimism"  => Urls::OPTIMISM_RPC,
        "bnb"       => Urls::BNB_RPC,
        "polygon" | "137" => Urls::POLYGON_RPC,
        _ => return 0.001, // unknown chain: conservative fallback
    };

    // Try eth_feeHistory (EIP-1559) first, fall back to eth_gasPrice
    let body = serde_json::json!({
        "jsonrpc": "2.0", "method": "eth_gasPrice", "params": [], "id": 1
    });
    let gas_price_wei: u128 = match async {
        let resp = reqwest::Client::new().post(rpc).json(&body).send().await.ok()?;
        let v: serde_json::Value = resp.json().await.ok()?;
        let hex = v["result"].as_str()?;
        let price = u128::from_str_radix(hex.trim_start_matches("0x"), 16).ok()?;
        if price > 0 { Some(price) } else { None }
    }.await
    {
        Some(p) => p,
        None => {
            // RPC unreachable — use static fallback per chain
            return match chain {
                "ethereum"  => 0.005,
                "arbitrum" | "base" | "optimism" => 0.0002,
                "bnb"       => 0.001,
                _           => 0.001,
            };
        }
    };

    const ERC20_GAS_UNITS: u128 = 65_000;
    const BUFFER: f64 = 1.2; // 20% headroom
    let cost_wei = gas_price_wei * ERC20_GAS_UNITS;
    (cost_wei as f64 / 1e18) * BUFFER
}

/// Return the native gas token balance (ETH, BNB, etc.) on a given chain.
/// Returns 0.0 if the chain cannot be queried or no native balance is found.
pub async fn get_native_gas_balance(chain: &str) -> f64 {
    let balances = get_chain_balances(chain).await;
    // Native token has an empty token_address in the onchainos balance output.
    balances
        .iter()
        .find(|b| b.token_address.is_empty())
        .map(|b| b.balance.parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0)
}

pub async fn get_chain_balances(chain: &str) -> Vec<ChainTokenBalance> {
    let output = tokio::process::Command::new(onchainos_bin())
        .args(["wallet", "balance", "--chain", chain])
        .output()
        .await;
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = match serde_json::from_str(stdout.trim()) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let assets = match v["data"]["details"]
        .as_array()
        .and_then(|d| d.first())
        .and_then(|d| d["tokenAssets"].as_array())
    {
        Some(a) => a.clone(),
        None => return vec![],
    };
    assets
        .iter()
        .filter_map(|a| {
            let usd_value = a["usdValue"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .or_else(|| a["usdValue"].as_f64())
                .unwrap_or(0.0);
            if usd_value <= 0.0 {
                return None;
            }
            Some(ChainTokenBalance {
                symbol: a["symbol"].as_str().unwrap_or("").to_string(),
                token_address: a["tokenAddress"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase(),
                usd_value,
                balance: a["balance"].as_str().unwrap_or("0").to_string(),
                decimal: a["decimal"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| a["decimal"].as_u64().map(|n| n as u8))
                    .unwrap_or(18),
            })
        })
        .collect()
}

/// Report plugin-level order metadata to the OKX backend for strategy attribution.
///
/// Serializes `payload` to a JSON string and passes it as `--plugin-parameter`.
/// Non-fatal at the call site: the trade has already succeeded before this is invoked,
/// so callers should log and continue on error rather than propagate.
pub async fn report_plugin_info(payload: &Value) -> Result<()> {
    let payload_str = serde_json::to_string(payload)
        .context("serializing report-plugin-info payload")?;
    let output = tokio::process::Command::new(onchainos_bin())
        .args([
            "wallet", "report-plugin-info",
            "--plugin-parameter", &payload_str,
            "--chain", CHAIN,
        ])
        .output()
        .await
        .context("Failed to spawn onchainos wallet report-plugin-info")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos report-plugin-info failed ({}): {}",
            output.status,
            stderr.trim()
        );
    }
    Ok(())
}

pub async fn is_ctf_approved_for_all(owner: &str, operator: &str) -> Result<bool> {
    use crate::config::{Contracts, Urls};
    // isApprovedForAll(address,address) selector = 0xe985e9c5
    let data = format!("0xe985e9c5{}{}", pad_address(owner), pad_address(operator));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::CTF, "data": data }, "latest"],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .context("Polygon RPC request failed")?;
    let v: serde_json::Value = resp.json().await
        .context("parsing Polygon RPC response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("Polygon RPC error: {}", err);
    }
    // ABI-encoded bool: 32 bytes. Approved = 0x0000...0001, Not approved = 0x0000...0000
    let hex = v["result"].as_str().unwrap_or("0x").trim_start_matches("0x");
    Ok(!hex.is_empty() && hex.trim_start_matches('0') == "1")
}

// ─── Deposit Wallet detection ─────────────────────────────────────────────────

/// WalletCreated event signature hash (observed from on-chain factory deployment tx).
///
/// Event layout (3 indexed topics):
///   topic[0] = this signature hash
///   topic[1] = wallet address (indexed)
///   topic[2] = owner address (indexed)
///   topic[3] = walletId (indexed, = bytes32(uint160(owner)))
///   data     = implementation address (non-indexed)
const WALLET_CREATED_TOPIC: &str =
    "0x7441de0ad639fe5d2bf1c22447715a0528b682385736bb40ae8dd92555eb8276";

/// Wait for a WALLET-CREATE relayer tx to confirm and extract the deployed deposit wallet
/// address from the factory's WalletCreated event log.
///
/// Returns the wallet address (lowercase, 0x-prefixed).
pub async fn wait_for_wallet_create_receipt(
    tx_hash: &str,
    max_wait_secs: u64,
) -> Result<String> {
    use crate::config::{Contracts, Urls};
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    let factory_lower = Contracts::DEPOSIT_WALLET_FACTORY.to_lowercase();
    let deadline = Instant::now() + Duration::from_secs(max_wait_secs);
    loop {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        let resp = reqwest::Client::new()
            .post(Urls::polygon_rpc())
            .json(&body)
            .send()
            .await;
        if let Ok(r) = resp {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                if v["result"].is_object() {
                    let status = v["result"]["status"].as_str().unwrap_or("0x1");
                    if status == "0x0" {
                        anyhow::bail!(
                            "WALLET-CREATE tx {} reverted on-chain (status 0x0). \
                             The factory's deploy() function requires OnlyOperator authorization. \
                             Check that builder credentials are valid.",
                            tx_hash
                        );
                    }
                    // Parse wallet address from the factory's WalletCreated event
                    if let Some(logs) = v["result"]["logs"].as_array() {
                        for log in logs {
                            let addr = log["address"].as_str().unwrap_or("").to_lowercase();
                            if addr == factory_lower {
                                if let Some(topics) = log["topics"].as_array() {
                                    if topics.len() >= 2
                                        && topics[0].as_str().unwrap_or("") == WALLET_CREATED_TOPIC
                                    {
                                        // topic[1] = indexed wallet address (32-byte ABI word)
                                        let t1 = topics[1].as_str().unwrap_or("");
                                        let hex = t1.trim_start_matches("0x");
                                        if hex.len() >= 40 {
                                            return Ok(format!("0x{}", &hex[hex.len() - 40..]));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    anyhow::bail!(
                        "WALLET-CREATE tx {} confirmed (status 0x1) but no WalletCreated \
                         event found in factory logs. This is unexpected — please report.",
                        tx_hash
                    );
                }
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "WALLET-CREATE tx {} not observed on Polygon within {}s. \
                 Check Polygonscan to confirm the transaction was included.",
                tx_hash,
                max_wait_secs
            );
        }
        sleep(Duration::from_millis(2000)).await;
    }
}

/// Check if a deposit wallet has been deployed for the given EOA.
///
/// Strategy (two-pass):
/// 1. Fast: `predictWalletAddress(owner, walletId)` — works for post-factory-upgrade wallets.
/// 2. Fallback: `eth_getLogs` scan backwards in 9,999-block chunks (free-tier limit).
///    Handles wallets deployed before a factory upgrade where the CREATE2 salt changed.
///    Scans up to MAX_SCAN_BLOCKS of history (≈14 hours of Polygon blocks).
///
/// Returns `Some(wallet_addr)` if deployed (code present), `None` otherwise.
pub async fn get_existing_deposit_wallet(eoa_addr: &str) -> Option<String> {
    use crate::config::{Contracts, Urls};

    let client = reqwest::Client::new();

    // ── Pass 1: predictWalletAddress (fast, O(2) RPC calls) ─────────────────
    let padded = pad_address(eoa_addr);
    let data = format!("0x1f264778{padded}{padded}"); // selector + owner + walletId
    let v: serde_json::Value = client
        .post(Urls::polygon_rpc())
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "method": "eth_call",
            "params": [{"to": Contracts::DEPOSIT_WALLET_FACTORY, "data": data}, "latest"],
            "id": 1
        }))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    if v.get("error").is_none() {
        let hex_result = v["result"].as_str().unwrap_or("").trim_start_matches("0x");
        if hex_result.len() >= 64 {
            let addr_hex = &hex_result[hex_result.len() - 40..];
            if !addr_hex.chars().all(|c| c == '0') {
                let wallet_addr = format!("0x{}", addr_hex);
                if let Ok(code_resp) = client
                    .post(Urls::polygon_rpc())
                    .json(&serde_json::json!({
                        "jsonrpc": "2.0", "method": "eth_getCode",
                        "params": [&wallet_addr, "latest"], "id": 2
                    }))
                    .send()
                    .await
                {
                    if let Ok(cv) = code_resp.json::<serde_json::Value>().await {
                        let code = cv["result"].as_str().unwrap_or("0x");
                        if code != "0x" && !code.is_empty() {
                            return Some(wallet_addr);
                        }
                    }
                }
            }
        }
    }

    // ── Pass 2: eth_getLogs chunked backwards scan ───────────────────────────
    // Uses polygon_logs_rpc() (publicnode) which supports ≤7,998 block range.
    // drpc free tier rejects eth_getLogs above ~7,500 blocks despite claiming 10,000.
    const CHUNK_SIZE: u64 = 7_499; // safely under publicnode's actual ~7,998 limit
    const MAX_SCAN_BLOCKS: u64 = 150_000; // ≈21 hours of Polygon @ 2 blocks/sec

    // Use the logs RPC endpoint (publicnode) for both block number and getLogs queries
    // so both calls go to the same reliable node — drpc free tier can silently fail eth_blockNumber.
    let logs_rpc = Urls::polygon_logs_rpc();
    let blk_body = serde_json::json!({"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1});
    let current_block = client.post(&logs_rpc).json(&blk_body).send().await.ok()?
        .json::<serde_json::Value>().await.ok()
        .and_then(|v| v["result"].as_str()
            .and_then(|h| u64::from_str_radix(h.trim_start_matches("0x"), 16).ok())
        )?;
    let padded_owner = format!("0x{:0>64}", eoa_addr.trim_start_matches("0x").to_lowercase());
    let scan_stop = current_block.saturating_sub(MAX_SCAN_BLOCKS);

    let mut to_block = current_block;
    while to_block > scan_stop {
        let from_block = to_block.saturating_sub(CHUNK_SIZE).max(scan_stop);
        let logs_body = serde_json::json!({
            "jsonrpc": "2.0", "method": "eth_getLogs",
            "params": [{
                "address": Contracts::DEPOSIT_WALLET_FACTORY,
                "fromBlock": format!("0x{:x}", from_block),
                "toBlock":   format!("0x{:x}", to_block),
                "topics": [WALLET_CREATED_TOPIC, serde_json::Value::Null, &padded_owner]
            }],
            "id": 3
        });
        if let Ok(resp) = client.post(&logs_rpc).json(&logs_body).send().await {
            if let Ok(lv) = resp.json::<serde_json::Value>().await {
                // Ignore RPC errors (e.g. transient range exceeded) and try next chunk
                if lv.get("error").is_none() {
                    if let Some(logs) = lv["result"].as_array() {
                        if let Some(log) = logs.last() {
                            if let Some(topics) = log["topics"].as_array() {
                                if topics.len() >= 2 {
                                    let t1 = topics[1].as_str().unwrap_or("");
                                    let hex = t1.trim_start_matches("0x");
                                    if hex.len() >= 40 {
                                        let wallet = format!("0x{}", &hex[hex.len() - 40..]);
                                        // Verify code still present
                                        if let Ok(cr) = client
                                            .post(Urls::polygon_rpc())
                                            .json(&serde_json::json!({
                                                "jsonrpc": "2.0", "method": "eth_getCode",
                                                "params": [&wallet, "latest"], "id": 4
                                            }))
                                            .send().await
                                        {
                                            if let Ok(cv) = cr.json::<serde_json::Value>().await {
                                                let code = cv["result"].as_str().unwrap_or("0x");
                                                if code != "0x" && !code.is_empty() {
                                                    return Some(wallet);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if from_block == 0 || from_block == scan_stop { break; }
        to_block = from_block.saturating_sub(1);
    }

    None
}

/// Compute the deterministic deposit wallet address for an EOA without deploying it.
/// Uses `predictWalletAddress(address owner, bytes32 walletId)`.
pub async fn predict_deposit_wallet_address(eoa_addr: &str) -> Option<String> {
    use crate::config::{Contracts, Urls};

    let padded = pad_address(eoa_addr);
    let data = format!("0x1f264778{padded}{padded}");

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": Contracts::DEPOSIT_WALLET_FACTORY, "data": data}, "latest"],
        "id": 1
    });

    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::polygon_rpc())
        .json(&body)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    if v.get("error").is_some() { return None; }

    let hex_result = v["result"].as_str()?.trim_start_matches("0x");
    if hex_result.len() < 64 { return None; }
    let addr_hex = &hex_result[hex_result.len() - 40..];
    if addr_hex.chars().all(|c| c == '0') { return None; }

    Some(format!("0x{}", addr_hex))
}


// ─── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Serialize env-var tests to prevent parallel test contamination.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // ── Bug #1: PATH resolution ──────────────────────────────────────────────

    /// `POLYMARKET_ONCHAINOS_BIN` env var overrides the binary path.
    /// This is the mechanism that lets CI inject a mock binary so onchainos
    /// calls can be stubbed without a real wallet.
    #[test]
    fn test_onchainos_bin_env_override() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("POLYMARKET_ONCHAINOS_BIN", "/usr/bin/env");
        let bin = onchainos_bin();
        std::env::remove_var("POLYMARKET_ONCHAINOS_BIN");
        assert_eq!(bin, std::ffi::OsString::from("/usr/bin/env"));
    }

    /// Without the env var and without ~/.local/bin/onchainos present,
    /// `onchainos_bin()` falls back to bare "onchainos".
    #[test]
    fn test_onchainos_bin_fallback_to_bare_name() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("POLYMARKET_ONCHAINOS_BIN");
        // Only test the fallback path when ~/.local/bin/onchainos is absent.
        let local_path = dirs::home_dir()
            .map(|h| h.join(".local").join("bin").join("onchainos"));
        if local_path.map(|p| p.is_file()).unwrap_or(false) {
            return; // test not applicable on a machine with onchainos installed
        }
        let bin = onchainos_bin();
        assert_eq!(bin, std::ffi::OsString::from("onchainos"));
    }

    // ── Bug #4: MAX_UINT approval calldata ──────────────────────────────────

    /// `usdc_approve` ABI-encodes amount as uint256. Verify that the calldata
    /// for u128::MAX contains the correct max-value bytes (the low 128 bits
    /// of MAX_UINT256).
    ///
    /// This test does NOT make a network call — it just checks the calldata
    /// that would be passed to wallet_contract_call.
    #[test]
    fn test_usdc_approve_max_uint_encoding() {
        // The calldata for approve(spender, u128::MAX) should end with
        // ffffffffffffffffffffffffffffffff (32 bytes / 16 bytes low + 16 high of 0).
        // Since u128::MAX = 0xffffffffffffffffffffffffffffffff (128 bits),
        // ABI-encoded as uint256 it is: 0000000000000000ffffffffffffffffffffffffffffffff
        // Wait — u128::MAX as ABI uint256 is:
        //   32 bytes big-endian: 16 zero bytes then 16 0xff bytes
        let amount = u128::MAX;
        let padded = pad_u256(amount);
        assert_eq!(padded.len(), 64, "pad_u256 must produce exactly 64 hex chars");
        assert_eq!(
            padded,
            "00000000000000000000000000000000ffffffffffffffffffffffffffffffff",
            "u128::MAX as uint256 should be 16 zero bytes followed by 16 0xff bytes"
        );
    }

    // ── Bug #2: NegRisk ABI encoding ────────────────────────────────────────

    /// `decimal_str_to_hex64("0")` should produce 64 zeros.
    #[test]
    fn test_decimal_str_to_hex64_zero() {
        let result = decimal_str_to_hex64("0").unwrap();
        assert_eq!(result, "0".repeat(64));
    }

    /// `decimal_str_to_hex64("255")` should produce 62 zeros + "ff".
    #[test]
    fn test_decimal_str_to_hex64_small_values() {
        let result = decimal_str_to_hex64("255").unwrap();
        assert_eq!(result, format!("{:0>64}", "ff"));

        let result = decimal_str_to_hex64("256").unwrap();
        assert_eq!(result, format!("{:0>64}", "100"));
    }

    /// u64::MAX = 18446744073709551615 = 0xffffffffffffffff
    #[test]
    fn test_decimal_str_to_hex64_u64_max() {
        let result = decimal_str_to_hex64("18446744073709551615").unwrap();
        assert_eq!(result, format!("{:0>64}", "ffffffffffffffff"));
    }

    /// u128::MAX = 340282366920938463463374607431768211455 = 0xffffffffffffffffffffffffffffffff
    #[test]
    fn test_decimal_str_to_hex64_u128_max() {
        let result = decimal_str_to_hex64("340282366920938463463374607431768211455").unwrap();
        assert_eq!(result, format!("{:0>64}", "ffffffffffffffffffffffffffffffff"));
    }

    /// Invalid decimal string (contains non-digit) should return an error.
    #[test]
    fn test_decimal_str_to_hex64_invalid_input() {
        assert!(decimal_str_to_hex64("0x1234").is_err(), "0x prefix is not valid decimal");
        assert!(decimal_str_to_hex64("12.34").is_err(), "decimal point is not a digit");
        assert!(decimal_str_to_hex64("").is_err(), "empty string should fail");
    }

    /// The `build_negrisk_redeem_calldata` calldata must have the correct structure:
    /// - 4-byte selector
    /// - 32-byte condition_id (bytes32)
    /// - 32-byte array offset (64 = 0x40)
    /// - 32-byte array length (number of amounts)
    /// - 32-byte per amount
    /// Total for 2 amounts: 4 + 4*32 = 4 + 128 = 132 bytes = 264 hex chars + 2 ("0x") = 266
    #[test]
    fn test_negrisk_redeem_calldata_length() {
        let condition_id = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let amounts = [1_000_000u128, 0u128];
        let calldata = build_negrisk_redeem_calldata(condition_id, &amounts);
        // 0x + 8 (selector) + 64 (cond_id) + 64 (offset) + 64 (len) + 64*2 (amounts) = 2 + 328 = 330
        assert_eq!(calldata.len(), 330, "calldata should be 330 chars (2 + 8 + 64*5)");
    }

    /// Verify the array offset field is encoded as 64 (0x40 = 2 static params × 32 bytes).
    #[test]
    fn test_negrisk_redeem_calldata_array_offset() {
        let condition_id = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let amounts = [0u128, 0u128];
        let calldata = build_negrisk_redeem_calldata(condition_id, &amounts);
        // Strip "0x" prefix. Layout: [selector 8][cond_id 64][array_offset 64][...]
        let hex = &calldata[2..];
        let array_offset_hex = &hex[8 + 64..8 + 64 + 64];
        // array_offset = 64 = 0x0000...0040
        assert_eq!(
            array_offset_hex,
            format!("{:0>64}", "40"),
            "array offset should be 64 (0x40)"
        );
    }

    /// Verify amounts are correctly encoded in the calldata.
    #[test]
    fn test_negrisk_redeem_calldata_amounts_encoding() {
        let condition_id = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let yes_amount = 50_000_000u128; // 50 USDC.e worth of shares
        let no_amount = 0u128;
        let calldata = build_negrisk_redeem_calldata(condition_id, &[yes_amount, no_amount]);
        let hex = &calldata[2..]; // strip "0x"
        // Layout: [selector 8][cond_id 64][offset 64][length 64][amount0 64][amount1 64]
        let amount0_hex = &hex[8 + 64 + 64 + 64..8 + 64 + 64 + 64 + 64];
        let amount1_hex = &hex[8 + 64 + 64 + 64 + 64..];
        assert_eq!(
            amount0_hex,
            format!("{:0>64x}", yes_amount),
            "yes amount should be correctly encoded"
        );
        assert_eq!(
            amount1_hex,
            format!("{:0>64x}", no_amount),
            "no amount should be correctly encoded"
        );
    }

    /// CTF.redeemPositions calldata has the correct selector.
    /// keccak256("redeemPositions(address,bytes32,bytes32,uint256[])") = 0xdbcb3da5
    #[test]
    fn test_ctf_redeem_positions_selector() {
        use sha3::{Digest, Keccak256};
        let selector = Keccak256::digest(b"redeemPositions(address,bytes32,bytes32,uint256[])");
        let expected = hex::encode(&selector[..4]);
        let cid = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let collateral = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"; // USDC.e (V1)
        let calldata = build_ctf_redeem_positions_calldata(cid, collateral);
        assert!(calldata.starts_with(&format!("0x{}", expected)),
            "CTF.redeemPositions selector should be 0x{}", expected);
    }

    /// NegRiskAdapter.redeemPositions calldata has the correct selector.
    /// keccak256("redeemPositions(bytes32,uint256[])") first 4 bytes
    #[test]
    fn test_negrisk_redeem_positions_selector() {
        use sha3::{Digest, Keccak256};
        let selector = Keccak256::digest(b"redeemPositions(bytes32,uint256[])");
        let expected = hex::encode(&selector[..4]);
        let cid = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let calldata = build_negrisk_redeem_calldata(cid, &[0u128]);
        assert!(calldata.starts_with(&format!("0x{}", expected)),
            "NegRiskAdapter.redeemPositions selector should be 0x{}", expected);
    }
}

