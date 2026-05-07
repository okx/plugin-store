// Integration tests: mock onchainos binary layer.
//
// These tests verify that functions which invoke the onchainos subprocess
// produce the correct CLI arguments and calldata. The mock_onchainos.sh binary
// records every invocation as a JSON line in the call log — tests then assert
// on that log rather than on RPC responses.
//
// Coverage:
//   Bug #1 — onchainos_bin() resolves the binary via $HOME path, not bare name
//   Bug #4 — approve_usdc encodes u128::MAX, not the exact order amount
//   Bug #4 — approve_usdc for neg_risk targets both exchange and adapter

mod common;

use common::*;
use wiremock::matchers::{method, body_partial_json};
use wiremock::{Mock, ResponseTemplate};
use serde_json::json;

// ── Bug #1: onchainos_bin resolves the binary ─────────────────────────────────

/// get_wallet_address() must use onchainos_bin() (which picks up
/// POLYMARKET_ONCHAINOS_BIN) rather than the bare string "onchainos".
/// If the bare name were used, the test binary would not be invoked and the
/// call log would remain empty.
#[tokio::test]
async fn test_get_wallet_address_uses_onchainos_bin_override() {
    let ctx = TestContext::new().await;

    let addr = polymarket_plugin::onchainos::get_wallet_address()
        .await
        .expect("get_wallet_address should succeed via mock binary");

    // The mock binary returns TEST_WALLET for `wallet addresses`
    assert_eq!(
        addr.to_lowercase(),
        TEST_WALLET.to_lowercase(),
        "wallet address should come from the mock binary, not a real onchainos"
    );

    // Call log must contain exactly one entry for `wallet addresses`
    let calls = ctx.calls();
    assert!(
        !calls.is_empty(),
        "mock binary should have been invoked (call log is empty — binary not found via POLYMARKET_ONCHAINOS_BIN)"
    );
    let has_wallet_cmd = calls
        .iter()
        .any(|c| c.args.iter().any(|a| a == "wallet") && c.args.iter().any(|a| a == "addresses"));
    assert!(has_wallet_cmd, "call log should contain a 'wallet addresses' invocation");
}

// ── Bug #4: approve_usdc always encodes u128::MAX ─────────────────────────────

/// approve_usdc for a normal (non-neg_risk) market must approve u128::MAX,
/// not the exact order amount.
///
/// Before fix: approve_usdc(neg_risk: bool, amount: u64) encoded `amount`.
/// After fix:  approve_usdc(neg_risk: bool) always encodes u128::MAX.
///
/// Regression test: if the encoding ever reverts to exact-amount, the
/// approve_uses_max_uint check will fail.
#[tokio::test]
async fn test_approve_usdc_encodes_max_uint_normal_market() {
    let ctx = TestContext::new().await;

    // Approve needs a successful tx receipt
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getTransactionReceipt"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(rpc_receipt_success(TEST_TX_HASH)),
        )
        .mount(&ctx.rpc_server)
        .await;

    polymarket_plugin::onchainos::approve_usdc(false)
        .await
        .expect("approve_usdc should succeed");

    let calls = ctx.calls();
    assert!(
        has_approve_call(&calls),
        "approve_usdc should emit a contract-call with the ERC-20 approve selector (0x095ea7b3)"
    );
    assert!(
        approve_uses_max_uint(&calls),
        "approve_usdc must encode u128::MAX as the allowance, not a specific amount — \
         encoding a specific amount causes re-approval on every trade when the previous \
         approval was MAX_UINT"
    );
}

/// approve_usdc for a neg_risk market must:
///   (a) approve CTF_EXCHANGE (normal exchange) — u128::MAX
///   (b) also approve NEG_RISK_ADAPTER — u128::MAX
///
/// Both targets are required for neg_risk sells and redeems to succeed.
#[tokio::test]
async fn test_approve_usdc_neg_risk_targets_both_contracts() {
    let ctx = TestContext::new().await;

    // Two approve tx receipts needed (exchange + adapter)
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getTransactionReceipt"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(rpc_receipt_success(TEST_TX_HASH)),
        )
        .mount(&ctx.rpc_server)
        .await;

    polymarket_plugin::onchainos::approve_usdc(true /* neg_risk */)
        .await
        .expect("approve_usdc neg_risk should succeed");

    let calls = ctx.calls();

    // Must have at least two approve calls (one per contract)
    let approve_calls: Vec<_> = calls_with_calldata(&calls, "095ea7b3");
    assert!(
        approve_calls.len() >= 2,
        "neg_risk approve_usdc must emit at least 2 approve calls (exchange + adapter), got {}",
        approve_calls.len()
    );

    // Both must use u128::MAX
    assert!(
        approve_uses_max_uint(&calls),
        "all approve calls must encode u128::MAX"
    );

    // One call must target NEG_RISK_CTF_EXCHANGE
    assert!(
        approve_targets_address(&calls, NEG_RISK_CTF_EXCHANGE),
        "approve_usdc neg_risk must approve NEG_RISK_CTF_EXCHANGE ({})",
        NEG_RISK_CTF_EXCHANGE
    );

    // One call must target NEG_RISK_ADAPTER
    assert!(
        approve_targets_address(&calls, NEG_RISK_ADAPTER),
        "approve_usdc neg_risk must approve NEG_RISK_ADAPTER ({}) — \
         missing this approval causes neg_risk sells and redeems to fail with insufficient allowance",
        NEG_RISK_ADAPTER
    );
}

// ── Bug #2: negrisk_redeem_positions calldata ─────────────────────────────────

/// negrisk_redeem_positions must call the NEG_RISK_ADAPTER contract, not
/// the CTF_EXCHANGE. Before the fix, redeem was stubbed out for neg_risk markets
/// and never reached this path.
///
/// The calldata must encode:
///   selector: 0x64e936d2  (redeemPositions(bytes32,uint256[]))
///   condition_id: 32-byte padded
///   array offset: 0x0000...0040 (64 decimal = 0x40)
///   array length: 2
///   amounts[0], amounts[1]: u128 values zero-padded to 32 bytes each
#[tokio::test]
async fn test_negrisk_redeem_positions_calls_adapter_contract() {
    let ctx = TestContext::new().await;

    // eth_call_simulate is called first — return a non-error response (no "error" key = success)
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_call"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(rpc_ok(serde_json::json!("0x"))),
        )
        .mount(&ctx.rpc_server)
        .await;

    // Contract call needs a successful receipt
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getTransactionReceipt"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(rpc_receipt_success(TEST_TX_HASH)),
        )
        .mount(&ctx.rpc_server)
        .await;

    let amounts: &[u128] = &[50_000_000, 0];
    polymarket_plugin::onchainos::negrisk_redeem_positions(
        TEST_CONDITION_ID,
        amounts,
        TEST_WALLET,
    )
    .await
    .expect("negrisk_redeem_positions should succeed");

    let calls = ctx.calls();

    // Must have at least one contract-call entry
    assert!(!calls.is_empty(), "negrisk_redeem_positions must invoke onchainos");

    // The `to` address must be the NEG_RISK_ADAPTER
    let adapter_lower = NEG_RISK_ADAPTER.to_lowercase();
    let targeted_adapter = calls.iter().any(|c| c.to == adapter_lower);
    assert!(
        targeted_adapter,
        "negrisk_redeem_positions must call NEG_RISK_ADAPTER ({}), not CTF_EXCHANGE — \
         calling the wrong contract causes the tx to revert",
        NEG_RISK_ADAPTER
    );

    // Calldata must start with the redeemPositions(bytes32,uint256[]) selector.
    // Selector = first 4 bytes of keccak256("redeemPositions(bytes32,uint256[])") = 0xdbeccb23.
    // Verified by running `build_negrisk_redeem_calldata` and inspecting the output.
    let redeem_selector = "dbeccb23";
    let has_redeem_calldata = calls
        .iter()
        .any(|c| c.calldata.to_lowercase().contains(redeem_selector));
    assert!(
        has_redeem_calldata,
        "negrisk_redeem_positions calldata must contain redeemPositions selector (0x{})\n\
         actual calls ({} entries):\n{}",
        redeem_selector,
        calls.len(),
        calls.iter().map(|c| format!("  to={} calldata={}", c.to, &c.calldata[..c.calldata.len().min(40)])).collect::<Vec<_>>().join("\n")
    );
}

/// The calldata for negrisk_redeem_positions must encode the array offset correctly.
/// The dynamic uint256[] array starts at byte offset 64 (0x40) after the selector,
/// meaning the offset word in the calldata must be 0x0000...0040.
///
/// An incorrect offset (e.g. 0x20 = 32) causes the contract to read the wrong
/// memory region and will either revert or silently zero the amounts.
///
/// Calldata layout (each "word" = 32 bytes = 64 hex chars):
///   [0..4]   selector (4 bytes)
///   [4..36]  condition_id (bytes32)
///   [36..68] array_offset (uint256) — must be 64 = 0x40
///   [68..100] array_length (uint256) — must be len(amounts)
///   [100..]  amounts[i] (uint256 each)
#[test]
fn test_negrisk_redeem_calldata_array_offset_is_64() {
    // This is a pure unit test of the ABI encoder — no subprocess or async needed.
    let calldata =
        polymarket_plugin::onchainos::build_negrisk_redeem_calldata(TEST_CONDITION_ID, &[100_u128, 200_u128]);

    // Strip "0x" prefix; skip 8-char selector
    let hex = calldata.strip_prefix("0x").unwrap_or(&calldata);
    assert!(hex.len() >= 8 + 64 + 64, "calldata too short: {}", calldata);

    // Word at position 1 (after selector) = condition_id (64 chars)
    // Word at position 2 = array offset (64 chars)
    let array_offset_word = &hex[8 + 64..8 + 64 + 64];
    let expected_offset = format!("{:0>64x}", 64u64);
    assert_eq!(
        array_offset_word, expected_offset,
        "ABI dynamic array offset must be 64 (0x40), got: 0x{}",
        array_offset_word
    );
}

/// The array length word must be 2 (for [yes_amount, no_amount]).
#[test]
fn test_negrisk_redeem_calldata_array_length_is_2() {
    let calldata =
        polymarket_plugin::onchainos::build_negrisk_redeem_calldata(TEST_CONDITION_ID, &[100_u128, 200_u128]);

    let hex = calldata.strip_prefix("0x").unwrap_or(&calldata);
    // Hex layout (positions in the hex string after "0x"):
    //   [0..8]     = selector (4 bytes = 8 hex chars)
    //   [8..72]    = condition_id (32 bytes = 64 hex chars)
    //   [72..136]  = array offset (32 bytes = 64 hex chars)
    //   [136..200] = array length (32 bytes = 64 hex chars)
    let length_word = &hex[8 + 64 + 64..8 + 64 + 64 + 64];
    let expected_length = format!("{:0>64x}", 2u64);
    assert_eq!(
        length_word, expected_length,
        "ABI array length must be 2, got: 0x{}",
        length_word
    );
}
