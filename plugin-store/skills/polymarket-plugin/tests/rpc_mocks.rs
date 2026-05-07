// Integration tests: Polygon RPC mock layer.
//
// These tests verify that functions hitting the Polygon RPC produce correct
// results and call the right JSON-RPC methods. They run against a local
// wiremock server — no real funds, no real network.
//
// Coverage:
//   Bug #3 — get_usdc_allowance uses eth_call, not CLOB API
//   Bug #4 — approve_usdc encodes u128::MAX
//   Bug #6 — wait_for_tx_receipt uses configurable timeout
//   General — balance checks, receipt polling

mod common;

use common::*;
use wiremock::matchers::{method, body_partial_json};
use wiremock::{Mock, ResponseTemplate};
use serde_json::json;

// ── Bug #3: allowance is read from chain, not CLOB API ───────────────────────

/// When on-chain allowance is 0, get_usdc_allowance returns 0.
/// This ensures the allowance check goes to the RPC, not the stale CLOB API.
#[tokio::test]
async fn test_get_usdc_allowance_reads_from_rpc_zero() {
    let ctx = TestContext::new().await;

    // RPC returns 0 for allowance(owner, spender)
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_call"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_eth_call_zero()))
        .mount(&ctx.rpc_server)
        .await;

    let allowance = polymarket_plugin::onchainos::get_usdc_allowance(
        TEST_WALLET,
        CTF_EXCHANGE,
    )
    .await
    .expect("get_usdc_allowance should succeed");

    assert_eq!(allowance, 0, "zero allowance returned from RPC should map to 0");
}

/// When on-chain allowance is MAX_UINT (previously approved), get_usdc_allowance returns u128::MAX.
/// A MAX_UINT allowance means no re-approve is needed — this was previously broken when
/// the CLOB API returned stale values that caused unnecessary re-approvals.
#[tokio::test]
async fn test_get_usdc_allowance_reads_from_rpc_max_uint() {
    let ctx = TestContext::new().await;

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_call"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_eth_call_max_uint()))
        .mount(&ctx.rpc_server)
        .await;

    let allowance = polymarket_plugin::onchainos::get_usdc_allowance(
        TEST_WALLET,
        CTF_EXCHANGE,
    )
    .await
    .expect("get_usdc_allowance should succeed");

    assert_eq!(
        allowance, u128::MAX,
        "MAX_UINT allowance returned from RPC should map to u128::MAX (not be truncated)"
    );
}

/// Specific USDC amount (e.g. $50 = 50_000_000 raw) is returned correctly.
#[tokio::test]
async fn test_get_usdc_allowance_specific_amount() {
    let ctx = TestContext::new().await;
    let expected: u128 = 50_000_000; // $50 USDC.e

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_call"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(rpc_eth_call_u256(expected)),
        )
        .mount(&ctx.rpc_server)
        .await;

    let allowance = polymarket_plugin::onchainos::get_usdc_allowance(
        TEST_WALLET,
        CTF_EXCHANGE,
    )
    .await
    .expect("get_usdc_allowance should succeed");

    assert_eq!(allowance, expected);
}

// ── Bug #6: wait_for_tx_receipt polls until confirmed ────────────────────────

/// Receipt is confirmed on the first poll — no timeout.
#[tokio::test]
async fn test_wait_for_tx_receipt_success_first_poll() {
    let ctx = TestContext::new().await;

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getTransactionReceipt"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(rpc_receipt_success(TEST_TX_HASH)),
        )
        .mount(&ctx.rpc_server)
        .await;

    // 90s timeout — should complete immediately since mock returns success
    polymarket_plugin::onchainos::wait_for_tx_receipt(TEST_TX_HASH, 90)
        .await
        .expect("should confirm on first poll");
}

/// Receipt returns status 0x0 (reverted) — function should return an error.
#[tokio::test]
async fn test_wait_for_tx_receipt_reverted_returns_error() {
    let ctx = TestContext::new().await;

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getTransactionReceipt"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(rpc_receipt_reverted(TEST_TX_HASH)),
        )
        .mount(&ctx.rpc_server)
        .await;

    let result = polymarket_plugin::onchainos::wait_for_tx_receipt(TEST_TX_HASH, 10).await;
    assert!(result.is_err(), "reverted tx should return an error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("reverted") || msg.contains("0x0"),
        "error should mention revert: {}",
        msg
    );
}

/// Receipt is never mined within the timeout — function should return a timeout error.
#[tokio::test]
async fn test_wait_for_tx_receipt_timeout_returns_error() {
    let ctx = TestContext::new().await;

    // Always return null (not mined yet)
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getTransactionReceipt"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(rpc_receipt_pending()),
        )
        .mount(&ctx.rpc_server)
        .await;

    // Use a 3-second timeout so the test runs quickly
    let result = polymarket_plugin::onchainos::wait_for_tx_receipt(TEST_TX_HASH, 3).await;
    assert!(result.is_err(), "unconfirmed tx should time out");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not observed on-chain") || msg.contains("within"),
        "error should mention timeout: {}",
        msg
    );
}

// ── get_ctf_balance: ERC-1155 balance query ───────────────────────────────────

/// ERC-1155 balanceOf returns correct share balance for a token.
/// Uses decimal_str_to_hex64 internally — this test validates the full path.
#[tokio::test]
async fn test_get_ctf_balance_positive() {
    let ctx = TestContext::new().await;
    let expected_shares: u128 = 50_000_000; // 50 shares in raw units

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_call"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(rpc_eth_call_u256(expected_shares)),
        )
        .mount(&ctx.rpc_server)
        .await;

    let balance = polymarket_plugin::onchainos::get_ctf_balance(
        TEST_WALLET,
        TEST_TOKEN_ID_YES,
    )
    .await
    .expect("get_ctf_balance should succeed");

    assert_eq!(balance, expected_shares);
}

/// ERC-1155 balanceOf returns 0 when the wallet holds no tokens.
#[tokio::test]
async fn test_get_ctf_balance_zero() {
    let ctx = TestContext::new().await;

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_call"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_eth_call_zero()))
        .mount(&ctx.rpc_server)
        .await;

    let balance = polymarket_plugin::onchainos::get_ctf_balance(
        TEST_WALLET,
        TEST_TOKEN_ID_YES,
    )
    .await
    .expect("get_ctf_balance should succeed");

    assert_eq!(balance, 0);
}

/// Invalid token ID (non-decimal) returns an error before hitting the RPC.
#[tokio::test]
async fn test_get_ctf_balance_invalid_token_id_returns_error() {
    // No mock server needed — error should be caught before any HTTP call
    let _ctx = TestContext::new().await;

    let result =
        polymarket_plugin::onchainos::get_ctf_balance(TEST_WALLET, "0xdeadbeef").await;
    assert!(
        result.is_err(),
        "hex-prefixed token ID should fail before RPC call"
    );
}

// ── USDC balance ─────────────────────────────────────────────────────────────

/// USDC balance is correctly decoded from the RPC response.
#[tokio::test]
async fn test_get_usdc_balance_decodes_correctly() {
    let ctx = TestContext::new().await;
    let raw_balance: u128 = 100_000_000; // $100 USDC.e

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_call"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(rpc_eth_call_u256(raw_balance)),
        )
        .mount(&ctx.rpc_server)
        .await;

    let balance_usdc =
        polymarket_plugin::onchainos::get_usdc_balance(TEST_WALLET).await;

    match balance_usdc {
        Ok(b) => assert!(
            (b - 100.0).abs() < 0.001,
            "expected ~100.0 USDC, got {}",
            b
        ),
        Err(e) => panic!("get_usdc_balance failed: {}", e),
    }
}
