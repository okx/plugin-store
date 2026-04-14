/// onchainos CLI wrappers for Polymarket on-chain operations.
use anyhow::{Context, Result};
use serde_json::Value;

const CHAIN: &str = "137";

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

    let output = tokio::process::Command::new("onchainos")
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

// ─── Proxy wallet ─────────────────────────────────────────────────────────────

/// Resolve the proxy wallet address created in `tx_hash` by inspecting the call trace.
///
/// Uses `debug_traceTransaction` with the callTracer to find the CREATE/CREATE2 sub-call
/// emitted by PROXY_FACTORY and extract the resulting contract address.
///
/// Resolve the proxy wallet address from `tx_hash` by inspecting the call trace.
///
/// Uses `debug_traceTransaction` with the callTracer on two RPCs (drpc + publicnode).
/// If neither RPC returns a verifiable EIP-1167 proxy address, returns an error — the
/// caller must NOT proceed with an unverified address, as depositing to a wrong EOA
/// will permanently lose user funds.
pub async fn get_proxy_address_from_tx(tx_hash: &str) -> Result<String> {
    use crate::config::Urls;

    let rpcs = [Urls::POLYGON_RPC, "https://polygon-bor-rpc.publicnode.com"];
    for rpc_url in &rpcs {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "debug_traceTransaction",
            "params": [tx_hash, {"tracer": "callTracer"}],
            "id": 1
        });
        let resp = reqwest::Client::new()
            .post(*rpc_url)
            .json(&body)
            .send()
            .await;
        if let Ok(r) = resp {
            if let Ok(v) = r.json::<Value>().await {
                if v.get("error").is_none() {
                    if let Some(addr) = find_create_in_trace(&v["result"]) {
                        // Mandatory: verify the resolved address is an EIP-1167 proxy.
                        // A wrong address (e.g. an EOA) would silently accept deposits and
                        // permanently lock user funds.
                        if verify_eip1167_proxy(&addr).await {
                            return Ok(addr);
                        }
                        anyhow::bail!(
                            "Resolved address {} from tx {} is not an EIP-1167 proxy contract. \
                             Refusing to proceed to protect funds. \
                             Check: https://polygonscan.com/tx/{}",
                            addr, tx_hash, tx_hash
                        );
                    }
                }
            }
        }
    }

    // No nonce-based fallback: guessing an address without on-chain verification risks
    // sending funds to a random EOA. Fail loudly instead.
    anyhow::bail!(
        "Could not retrieve proxy address from tx {} via debug_traceTransaction on any RPC. \
         This may be a temporary RPC outage — wait a few seconds and retry setup-proxy. \
         Do NOT deposit until the proxy address is confirmed on-chain. \
         Check: https://polygonscan.com/tx/{}",
        tx_hash, tx_hash
    )
}

/// Search a callTracer trace for any call (CREATE, CREATE2, or CALL) made BY PROXY_FACTORY.
/// The factory always calls the proxy wallet as its first sub-call — whether creating a new one
/// (CREATE/CREATE2) or forwarding calls to an existing one (CALL). The `to` field is the proxy.
fn find_create_in_trace(trace: &Value) -> Option<String> {
    use crate::config::Contracts;
    let factory = Contracts::PROXY_FACTORY.to_lowercase();

    // Check direct sub-calls of the current frame
    if let Some(calls) = trace["calls"].as_array() {
        for sub in calls {
            let from = sub["from"].as_str().unwrap_or("").to_lowercase();
            let call_type = sub["type"].as_str().unwrap_or("");
            let to = sub["to"].as_str().unwrap_or("");

            // Any call FROM the factory is to the proxy (new or existing)
            if from == factory && !to.is_empty()
                && matches!(call_type, "CREATE" | "CREATE2" | "CALL")
            {
                return Some(to.to_string());
            }

            // Recurse deeper
            if let Some(addr) = find_create_in_trace(sub) {
                return Some(addr);
            }
        }
    }
    None
}

/// Compute the CREATE address for a deployment from `deployer` at `nonce`.
/// Formula: keccak256(rlp([deployer, nonce]))[12:]
fn compute_create_address(deployer: &str, nonce: u64) -> Result<String> {
    use sha3::{Digest, Keccak256};

    let addr_bytes = hex::decode(deployer.trim_start_matches("0x"))
        .context("decoding deployer address")?;
    anyhow::ensure!(addr_bytes.len() == 20, "deployer must be 20 bytes");

    // RLP-encode address (20 bytes): 0x94 prefix (0x80 + 20)
    let rlp_addr: Vec<u8> = [&[0x94u8][..], &addr_bytes].concat();

    // RLP-encode nonce
    let rlp_nonce: Vec<u8> = if nonce == 0 {
        vec![0x80]
    } else {
        let b = {
            let mut tmp = nonce;
            let mut bytes = Vec::new();
            while tmp > 0 {
                bytes.push((tmp & 0xFF) as u8);
                tmp >>= 8;
            }
            bytes.reverse();
            bytes
        };
        if b.len() == 1 && b[0] < 0x80 {
            b
        } else {
            [[0x80 + b.len() as u8].as_slice(), &b].concat()
        }
    };

    // RLP-encode list: payload = rlp_addr + rlp_nonce
    let payload: Vec<u8> = [rlp_addr, rlp_nonce].concat();
    let list_prefix: Vec<u8> = if payload.len() < 56 {
        vec![0xC0 + payload.len() as u8]
    } else {
        let len_bytes = {
            let l = payload.len();
            let mut tmp = l;
            let mut bytes = Vec::new();
            while tmp > 0 {
                bytes.push((tmp & 0xFF) as u8);
                tmp >>= 8;
            }
            bytes.reverse();
            bytes
        };
        [[0xF7 + len_bytes.len() as u8].as_slice(), &len_bytes].concat()
    };
    let encoded: Vec<u8> = [list_prefix, payload].concat();

    let hash = Keccak256::digest(&encoded);
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

/// Check whether an address has EIP-1167 minimal proxy bytecode deployed.
async fn verify_eip1167_proxy(addr: &str) -> bool {
    use crate::config::Urls;
    const EIP1167_PREFIX: &str = "363d3d373d3d3d363d73";
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [addr, "latest"],
        "id": 1
    });
    if let Ok(r) = reqwest::Client::new()
        .post(Urls::POLYGON_RPC)
        .json(&body)
        .send()
        .await
    {
        if let Ok(v) = r.json::<Value>().await {
            if let Some(code) = v["result"].as_str() {
                return code.trim_start_matches("0x").starts_with(EIP1167_PREFIX);
            }
        }
    }
    false
}

/// Create a Polymarket proxy wallet via PROXY_FACTORY.proxy([]).
///
/// Calls `proxy((uint8,address,uint256,bytes)[])` with an empty calls array.
/// The factory deploys a minimal-proxy clone keyed to msg.sender (this wallet).
/// Returns the tx hash; call `get_proxy_address_from_tx(tx_hash)` to resolve the address.
///
/// NOTE: One-time gas cost in POL. All subsequent trading via the proxy is relayer-paid.
/// Query PROXY_FACTORY via debug_traceCall to check if a proxy already exists for `eoa_addr`.
///
/// Returns:
/// - `Ok(Some(addr))` — proxy exists on-chain and is a valid EIP-1167 contract
/// - `Ok(None)`       — no proxy exists yet (safe to deploy)
/// - `Err(...)`       — RPC call failed; caller MUST NOT proceed with deployment,
///                      as we cannot distinguish "no proxy" from "RPC error"
pub async fn get_existing_proxy(eoa_addr: &str) -> Result<Option<String>> {
    use sha3::{Digest, Keccak256};
    use crate::config::{Contracts, Urls};

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
        .post(Urls::POLYGON_RPC)
        .json(&body)
        .send()
        .await
        .context("debug_traceCall RPC request failed")?;

    let v: serde_json::Value = resp.json().await
        .context("debug_traceCall response parse failed")?;

    if let Some(err) = v.get("error") {
        anyhow::bail!(
            "debug_traceCall returned an error while checking for existing proxy: {}. \
             Cannot safely determine whether a proxy exists — aborting to prevent duplicate deployment.",
            err
        );
    }

    Ok(find_create_in_trace(&v["result"]))
}

pub async fn create_proxy_wallet() -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // Function selector: keccak256("proxy((uint8,address,uint256,bytes)[])")
    let selector = Keccak256::digest(b"proxy((uint8,address,uint256,bytes)[])");
    let selector_hex = hex::encode(&selector[..4]);

    // ABI-encode empty dynamic array: offset=0x20 (32), length=0
    let calldata = format!(
        "0x{}\
         0000000000000000000000000000000000000000000000000000000000000020\
         0000000000000000000000000000000000000000000000000000000000000000",
        selector_hex
    );

    let result = wallet_contract_call(Contracts::PROXY_FACTORY, &calldata).await?;
    extract_tx_hash(&result)
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

/// Get USDC.e ERC-20 allowance for owner → spender. Returns raw amount (6 decimals).
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
        .post(Urls::POLYGON_RPC)
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
/// For neg_risk=false: approves CTF Exchange only.
/// For neg_risk=true: approves BOTH NEG_RISK_CTF_EXCHANGE and NEG_RISK_ADAPTER —
/// the CLOB checks both contracts in the settlement path for neg_risk markets.
/// Returns the tx hash of the last approval submitted.
pub async fn approve_usdc(neg_risk: bool, amount: u64) -> Result<String> {
    use crate::config::Contracts;
    let usdc = Contracts::USDC_E;
    if neg_risk {
        usdc_approve(usdc, Contracts::NEG_RISK_CTF_EXCHANGE, amount as u128).await?;
        usdc_approve(usdc, Contracts::NEG_RISK_ADAPTER, amount as u128).await
    } else {
        usdc_approve(usdc, Contracts::CTF_EXCHANGE, amount as u128).await
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

/// ABI-encode and submit CTF redeemPositions(collateralToken, parentCollectionId, conditionId, indexSets).
///
/// Redeems all outcome positions for the given conditionId. indexSets [1, 2] covers both
/// YES (bit 0) and NO (bit 1) outcomes — the CTF contract only pays out for winning tokens
/// and silently no-ops for losing ones, so passing both is safe.
/// For neg_risk (multi-outcome) markets use the NEG_RISK_ADAPTER path (not implemented here).
pub async fn ctf_redeem_positions(condition_id: &str) -> Result<String> {
    use sha3::{Digest, Keccak256};
    use crate::config::Contracts;

    // Compute the 4-byte function selector: keccak256("redeemPositions(address,bytes32,bytes32,uint256[])")
    let selector = Keccak256::digest(b"redeemPositions(address,bytes32,bytes32,uint256[])");
    let selector_hex = hex::encode(&selector[..4]);

    // ABI-encode the four parameters.
    // Slots 0-2 are static (address and bytes32); slot 3 is the offset to the dynamic uint256[] array.
    let collateral  = pad_address(Contracts::USDC_E);         // address padded to 32 bytes
    let parent_id   = format!("{:064x}", 0u128);               // bytes32(0) — null parent collection
    let cond_id_hex = condition_id.trim_start_matches("0x");
    let cond_id_pad = format!("{:0>64}", cond_id_hex);         // conditionId as bytes32
    let array_offset = pad_u256(4 * 32);                       // 4 static slots → offset = 128

    // Dynamic array: length=2, [1, 2] (YES indexSet=1, NO indexSet=2)
    let array_len  = pad_u256(2);
    let index_yes  = pad_u256(1);  // outcome 0, indexSet bit 0
    let index_no   = pad_u256(2);  // outcome 1, indexSet bit 1

    let calldata = format!(
        "0x{}{}{}{}{}{}{}{}",
        selector_hex, collateral, parent_id, cond_id_pad,
        array_offset, array_len, index_yes, index_no
    );

    let result = wallet_contract_call(Contracts::CTF, &calldata).await?;
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
        .post(Urls::POLYGON_RPC)
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

/// Get USDC.e (ERC-20) balance for an address. Returns human-readable f64 (dollars).
pub async fn get_usdc_balance(addr: &str) -> Result<f64> {
    use crate::config::{Contracts, Urls};
    // balanceOf(address) selector = 0x70a08231
    let data = format!("0x70a08231{}", pad_address(addr));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": Contracts::USDC_E, "data": data }, "latest"],
        "id": 1
    });
    let v: serde_json::Value = reqwest::Client::new()
        .post(Urls::POLYGON_RPC)
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
    Ok(raw as f64 / 1_000_000.0) // USDC.e has 6 decimals
}

/// Poll eth_getTransactionReceipt until the tx is mined (or timeout).
///
/// Polygon block time is ~2 seconds. We poll every 2 seconds for up to max_wait_secs.
/// Call this after submitting an approval tx before posting any order.
pub async fn wait_for_tx_receipt(tx_hash: &str, max_wait_secs: u64) -> Result<()> {
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
            .post(Urls::POLYGON_RPC)
            .json(&body)
            .send()
            .await;
        if let Ok(r) = resp {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                // receipt is an object (not null) once the tx is mined
                if v["result"].is_object() {
                    // status "0x1" = success, "0x0" = reverted
                    let status = v["result"]["status"].as_str().unwrap_or("0x1");
                    if status == "0x0" {
                        anyhow::bail!(
                            "Transaction {} was mined but reverted (status 0x0). \
                             Check Polygonscan for details.",
                            tx_hash
                        );
                    }
                    return Ok(());
                }
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "Approval tx {} not confirmed within {}s. \
                 Check Polygonscan and retry.",
                tx_hash, max_wait_secs
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
    let output = tokio::process::Command::new("onchainos")
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
    let output = tokio::process::Command::new("onchainos")
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
    let output = tokio::process::Command::new("onchainos")
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
        .post(Urls::POLYGON_RPC)
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

