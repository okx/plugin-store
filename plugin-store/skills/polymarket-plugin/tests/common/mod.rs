// Shared test harness for polymarket-plugin integration tests.
//
// The harness wires together three layers:
//   1. mock_onchainos binary  — intercepts subprocess calls (wallet addresses, contract-call)
//   2. MockRpcServer          — wiremock server impersonating the Polygon JSON-RPC endpoint
//   3. MockClobServer         — wiremock server impersonating the Polymarket CLOB/Gamma/Data APIs
//
// All env var overrides are set on the TestContext and cleaned up on Drop.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};
use serde_json::{json, Value};

// ── Env-var serialization ─────────────────────────────────────────────────────
//
// Integration tests that set POLYMARKET_TEST_* env vars must not run in parallel
// (env vars are process-global). Acquire this mutex before setting any env var,
// hold the guard for the lifetime of the TestContext, and release on drop.

fn env_mutex() -> Arc<tokio::sync::Mutex<()>> {
    static M: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();
    M.get_or_init(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
}

// ── Constants ─────────────────────────────────────────────────────────────────

pub const TEST_WALLET: &str = "0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF";
pub const TEST_TX_HASH: &str =
    "0xABCD1234ABCD1234ABCD1234ABCD1234ABCD1234ABCD1234ABCD1234ABCD1234";
pub const TEST_CONDITION_ID: &str =
    "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
pub const TEST_TOKEN_ID_YES: &str = "21742633143463906290569050155826241533067272736897614950488156847949938836455";
pub const TEST_TOKEN_ID_NO: &str = "52114319501245915516055106046884209969926127482827954674443846427813813222426";

// Contracts from config.rs (duplicated here to avoid importing plugin internals)
pub const CTF_EXCHANGE: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
pub const NEG_RISK_CTF_EXCHANGE: &str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";
pub const NEG_RISK_ADAPTER: &str = "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296";
pub const USDC_E: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
pub const CTF: &str = "0x4D97DCd97eC945f40cF65F87097ACe5EA0476045";

// ── Mock onchainos binary path ────────────────────────────────────────────────

pub fn mock_onchainos_path() -> PathBuf {
    // Resolve relative to the crate root (where cargo test is run from)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join("mock_onchainos.sh")
}

// ── Call log helpers ──────────────────────────────────────────────────────────

/// A single recorded invocation of the mock_onchainos binary.
#[derive(Debug, Clone)]
pub struct OnchainosCall {
    /// All CLI args passed to the mock binary.
    pub args: Vec<String>,
    /// The `--to` address, if present.
    pub to: String,
    /// The `--input-data` hex string, if present.
    pub calldata: String,
}

/// Read all calls recorded in a mock onchainos call log file.
pub fn read_call_log(path: &std::path::Path) -> Vec<OnchainosCall> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .map(|v| OnchainosCall {
            args: v["args"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|a| a.as_str().map(String::from))
                .collect(),
            to: v["to"].as_str().unwrap_or("").to_lowercase(),
            calldata: v["calldata"].as_str().unwrap_or("").to_string(),
        })
        .collect()
}

/// Find calls where the calldata contains the given hex substring (case-insensitive).
pub fn calls_with_calldata<'a>(calls: &'a [OnchainosCall], hex_substr: &str) -> Vec<&'a OnchainosCall> {
    let needle = hex_substr.to_lowercase();
    calls
        .iter()
        .filter(|c| c.calldata.to_lowercase().contains(&needle))
        .collect()
}

/// Return true if any call's calldata contains the ERC-20 approve selector (0x095ea7b3).
pub fn has_approve_call(calls: &[OnchainosCall]) -> bool {
    !calls_with_calldata(calls, "095ea7b3").is_empty()
}

/// Return true if any approve call was sent to the given spender address.
/// The spender is ABI-encoded as the second 32-byte word after the selector.
pub fn approve_targets_address(calls: &[OnchainosCall], spender: &str) -> bool {
    let target = spender.trim_start_matches("0x").to_lowercase();
    let padded = format!("{:0>64}", target); // left-pad to 64 hex chars (32 bytes)
    calls_with_calldata(calls, "095ea7b3")
        .iter()
        .any(|c| c.calldata.to_lowercase().contains(&padded))
}

/// Return true if any approve call encodes amount = u128::MAX.
/// u128::MAX ABI-encoded as uint256: first 16 bytes = 00, last 16 bytes = ff.
pub fn approve_uses_max_uint(calls: &[OnchainosCall]) -> bool {
    let max_uint_suffix = "ffffffffffffffffffffffffffffffff";
    calls_with_calldata(calls, "095ea7b3")
        .iter()
        .any(|c| c.calldata.to_lowercase().ends_with(max_uint_suffix))
}

/// Return the selector (first 4 bytes, 8 hex chars after "0x") from calldata.
pub fn selector_of(calldata: &str) -> &str {
    let hex = calldata.strip_prefix("0x").unwrap_or(calldata);
    if hex.len() >= 8 { &hex[..8] } else { hex }
}

// ── RPC response builders ─────────────────────────────────────────────────────

/// Build a successful JSON-RPC response wrapping `result`.
pub fn rpc_ok(result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "result": result, "id": 1 })
}

/// Build a JSON-RPC response for eth_call returning a uint256 value.
/// `value` is the raw u128 (fits any realistic allowance/balance).
pub fn rpc_eth_call_u256(value: u128) -> Value {
    rpc_ok(Value::String(format!("0x{:064x}", value)))
}

/// eth_call returning a 32-byte zero result (e.g. allowance = 0).
pub fn rpc_eth_call_zero() -> Value {
    rpc_eth_call_u256(0)
}

/// eth_call returning u128::MAX (what a previously approved MAX_UINT looks like).
pub fn rpc_eth_call_max_uint() -> Value {
    // u128::MAX as uint256 = 0x0000000000000000ffffffffffffffffffffffffffffffff
    rpc_ok(Value::String(format!(
        "0x{:0>64}",
        format!("{:x}", u128::MAX)
    )))
}

/// eth_getTransactionReceipt returning success (status 0x1).
pub fn rpc_receipt_success(tx_hash: &str) -> Value {
    rpc_ok(json!({
        "transactionHash": tx_hash,
        "status": "0x1",
        "blockNumber": "0x1234",
        "gasUsed": "0x5208"
    }))
}

/// eth_getTransactionReceipt returning null (tx not yet mined — triggers timeout).
pub fn rpc_receipt_pending() -> Value {
    rpc_ok(Value::Null)
}

/// eth_getTransactionReceipt returning reverted (status 0x0).
pub fn rpc_receipt_reverted(tx_hash: &str) -> Value {
    rpc_ok(json!({
        "transactionHash": tx_hash,
        "status": "0x0",
        "blockNumber": "0x1234",
        "gasUsed": "0x5208"
    }))
}

/// Polygon native balance (in wei, as 0x hex). 1 POL = 1e18 wei.
pub fn rpc_pol_balance(pol: f64) -> Value {
    let wei = (pol * 1e18) as u128;
    rpc_ok(Value::String(format!("0x{:x}", wei)))
}

/// USDC.e balance in raw units (1 USDC.e = 1_000_000 raw).
pub fn rpc_usdc_balance(usdc: f64) -> Value {
    let raw = (usdc * 1_000_000.0) as u128;
    rpc_eth_call_u256(raw)
}

// ── CLOB / Gamma / Data response builders ────────────────────────────────────

pub fn clob_market(condition_id: &str, neg_risk: bool) -> Value {
    json!({
        "condition_id": condition_id,
        "question": "Test market: will X happen?",
        "tokens": [
            { "token_id": TEST_TOKEN_ID_YES, "outcome": "YES", "price": 0.75, "winner": neg_risk },
            { "token_id": TEST_TOKEN_ID_NO,  "outcome": "NO",  "price": 0.25, "winner": false }
        ],
        "active": true,
        "closed": neg_risk,
        "accepting_orders": !neg_risk,
        "neg_risk": neg_risk,
        "maker_base_fee": 200,
        "taker_base_fee": 200,
        "end_date_iso": "2026-01-01T00:00:00Z"
    })
}

pub fn clob_orderbook() -> Value {
    json!({
        "bids": [{ "price": "0.74", "size": "200.00" }],
        "asks": [{ "price": "0.76", "size": "200.00" }],
        "last_update": 1700000000
    })
}

pub fn clob_order_response() -> Value {
    json!({
        "success": true,
        "order_id": "test-order-12345",
        "status": "matched",
        "making_amount": "500000",
        "taking_amount": "750000"
    })
}

pub fn data_positions(condition_id: &str, redeemable: bool) -> Value {
    json!([{
        "conditionId": condition_id,
        "asset": TEST_TOKEN_ID_YES,
        "size": 50.0,
        "avgPrice": 0.65,
        "currentValue": 37.5,
        "redeemable": redeemable,
        "title": "Test market"
    }])
}

pub fn gamma_market(condition_id: &str, neg_risk: bool) -> Value {
    json!([{
        "id": "99999",
        "conditionId": condition_id,
        "slug": "test-market-slug",
        "question": "Test market: will X happen?",
        "active": !neg_risk,
        "closed": neg_risk,
        "acceptingOrders": !neg_risk,
        "negRisk": neg_risk,
        "clobTokenIds": format!("[\"{}\",\"{}\"]", TEST_TOKEN_ID_YES, TEST_TOKEN_ID_NO),
        "outcomePrices": "[\"0.75\",\"0.25\"]",
        "outcomes": "[\"YES\",\"NO\"]",
        "volume24hr": "50000.00"
    }])
}

// ── TestContext ───────────────────────────────────────────────────────────────

/// Holds live mock servers and a temporary call-log file for one test.
/// Env vars are set on construction and removed on drop.
/// The `_env_guard` holds a lock on `env_mutex()` for the lifetime of this struct,
/// ensuring tests that use env vars do not run concurrently.
pub struct TestContext {
    pub rpc_server: MockServer,
    pub clob_server: MockServer,
    pub call_log: tempfile::NamedTempFile,
    env_keys: Vec<String>,
    _env_guard: tokio::sync::OwnedMutexGuard<()>,
}

impl TestContext {
    /// Start mock servers and configure env vars.
    /// Blocks until it acquires the env-var mutex — tests using TestContext run serially.
    pub async fn new() -> Self {
        // Acquire the env-var lock before starting servers. Held until this
        // TestContext is dropped, preventing parallel tests from clobbering env vars.
        let _env_guard = env_mutex().lock_owned().await;

        let rpc_server = MockServer::start().await;
        let clob_server = MockServer::start().await;
        let call_log = tempfile::NamedTempFile::new().expect("temp file");

        let mock_bin = mock_onchainos_path();
        assert!(
            mock_bin.exists(),
            "mock_onchainos.sh not found at {:?}",
            mock_bin
        );

        let mut ctx = TestContext {
            rpc_server,
            clob_server,
            call_log,
            env_keys: Vec::new(),
            _env_guard,
        };

        ctx.set_env("POLYMARKET_TEST_POLYGON_RPC", &ctx.rpc_server.uri());
        ctx.set_env("POLYMARKET_TEST_CLOB_URL", &ctx.clob_server.uri());
        ctx.set_env("POLYMARKET_TEST_GAMMA_URL", &ctx.clob_server.uri()); // share server
        ctx.set_env("POLYMARKET_TEST_DATA_URL", &ctx.clob_server.uri());  // share server
        ctx.set_env("POLYMARKET_ONCHAINOS_BIN", mock_bin.to_str().unwrap());
        ctx.set_env("MOCK_ONCHAINOS_WALLET", TEST_WALLET);
        ctx.set_env("MOCK_ONCHAINOS_TX_HASH", TEST_TX_HASH);
        let call_log_path = ctx.call_log.path().to_str().unwrap().to_string();
        ctx.set_env("MOCK_ONCHAINOS_CALL_LOG", &call_log_path);

        ctx
    }

    fn set_env(&mut self, key: &str, value: &str) {
        std::env::set_var(key, value);
        self.env_keys.push(key.to_string());
    }

    /// Read the call log recorded by the mock binary.
    pub fn calls(&self) -> Vec<OnchainosCall> {
        read_call_log(self.call_log.path())
    }

    /// Register a default Polygon RPC handler that routes requests by JSON-RPC method.
    ///
    /// Returns different responses for:
    ///   eth_call           → zero by default (allowance = 0, balance depends on selector)
    ///   eth_getBalance     → 1 POL (enough to pass gas check)
    ///   eth_getTransactionReceipt → success on first poll
    pub async fn mock_rpc_defaults(&self) {
        // eth_getBalance (POL balance for gas check)
        Mock::given(method("POST"))
            .and(wiremock::matchers::body_partial_json(
                json!({"method": "eth_getBalance"}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(rpc_pol_balance(1.0)),
            )
            .mount(&self.rpc_server)
            .await;

        // eth_getTransactionReceipt — immediate success
        Mock::given(method("POST"))
            .and(wiremock::matchers::body_partial_json(
                json!({"method": "eth_getTransactionReceipt"}),
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(rpc_receipt_success(TEST_TX_HASH)),
            )
            .mount(&self.rpc_server)
            .await;

        // eth_call — zero (override per-test for specific functions)
        Mock::given(method("POST"))
            .and(wiremock::matchers::body_partial_json(
                json!({"method": "eth_call"}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(rpc_eth_call_zero()),
            )
            .mount(&self.rpc_server)
            .await;
    }

    /// Register a CLOB API handler returning a standard binary market.
    pub async fn mock_clob_market(&self, condition_id: &str) {
        let body = clob_market(condition_id, false);
        Mock::given(method("GET"))
            .and(path_regex("^/markets/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.clob_server)
            .await;
    }

    /// Register a CLOB API handler returning a neg_risk market.
    pub async fn mock_clob_market_neg_risk(&self, condition_id: &str) {
        let body = clob_market(condition_id, true);
        Mock::given(method("GET"))
            .and(path_regex("^/markets/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.clob_server)
            .await;
    }

    /// Register a Data API positions handler.
    pub async fn mock_positions(&self, condition_id: &str, redeemable: bool) {
        let body = data_positions(condition_id, redeemable);
        Mock::given(method("GET"))
            .and(path("/positions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.clob_server)
            .await;
    }

    /// Register a Gamma API market handler.
    pub async fn mock_gamma_market(&self, condition_id: &str) {
        let body = gamma_market(condition_id, false);
        Mock::given(method("GET"))
            .and(path_regex("^/markets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.clob_server)
            .await;
    }

    /// Override the eth_call response to return a specific u128 value.
    /// Mounts *before* the default handler so it takes priority (wiremock matches in mount order).
    pub async fn mock_eth_call_returns(&self, value: u128) {
        Mock::given(method("POST"))
            .and(wiremock::matchers::body_partial_json(
                json!({"method": "eth_call"}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(rpc_eth_call_u256(value)),
            )
            .up_to_n_times(100)
            .mount(&self.rpc_server)
            .await;
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        for key in &self.env_keys {
            std::env::remove_var(key);
        }
    }
}
