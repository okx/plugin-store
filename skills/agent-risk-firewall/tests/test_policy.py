from agent_risk_firewall.models import validate_check_input
from agent_risk_firewall.policy import evaluate


def context(operation="buy", slippage=0.5, price_impact=0.5, amount_usd=10):
    payload = {
        "chain": "xlayer",
        "operation": operation,
        "walletAddress": "0x0000000000000000000000000000000000000000",
        "tokenIn": {
            "address": "0x0000000000000000000000000000000000000001",
            "symbol": "USDC",
            "decimals": 6,
        },
        "tokenOut": {
            "address": "0x0000000000000000000000000000000000000002",
            "symbol": "RISK",
            "decimals": 18,
        },
        "amountIn": "10",
        "amountInUsd": amount_usd,
        "quote": {"slippagePct": slippage, "priceImpactPct": price_impact},
    }
    normalized, findings = validate_check_input(payload)
    return normalized, findings


def evidence(token_risk="LOW", tx_action=None, simulation_data=None):
    tx_scan = {"status": "skipped"}
    if tx_action:
        tx_scan = {"status": "ok", "data": {"action": tx_action}}
    simulation = {"status": "skipped"}
    if simulation_data is not None:
        simulation = {"status": "ok", "data": simulation_data}
    return {
        "tokenScan": {"status": "ok", "data": {"riskLevel": token_risk}},
        "tokenReport": {"status": "ok", "data": {"liquidityUsd": 500000}},
        "txScan": tx_scan,
        "simulation": simulation,
    }


def test_critical_token_buy_blocks():
    ctx, findings = context("buy")
    result = evaluate(ctx, evidence("CRITICAL"), findings)
    assert result["verdict"] == "block"
    assert any(reason["code"] == "TOKEN_CRITICAL" for reason in result["reasons"])


def test_critical_token_sell_warns():
    ctx, findings = context("sell")
    result = evaluate(ctx, evidence("CRITICAL"), findings)
    assert result["verdict"] == "warn"
    assert any(reason["code"] == "TOKEN_CRITICAL_SELL" for reason in result["reasons"])


def test_tx_scan_block_wins():
    payload = {
        "chain": "xlayer",
        "operation": "swap",
        "walletAddress": "0x0000000000000000000000000000000000000000",
        "tokenIn": {"address": "0x0000000000000000000000000000000000000001"},
        "tokenOut": {"address": "0x0000000000000000000000000000000000000002"},
        "quote": {"slippagePct": 0.1, "priceImpactPct": 0.1},
        "tx": {
            "from": "0x0000000000000000000000000000000000000000",
            "to": "0x0000000000000000000000000000000000000003",
            "data": "0x",
        },
    }
    ctx, findings = validate_check_input(payload)
    result = evaluate(ctx, evidence("LOW", tx_action="block"), findings)
    assert result["verdict"] == "block"
    assert any(reason["code"] == "TX_SCAN_BLOCK" for reason in result["reasons"])


def test_simulation_revert_blocks():
    payload = {
        "chain": "xlayer",
        "operation": "swap",
        "walletAddress": "0x0000000000000000000000000000000000000000",
        "tokenIn": {"address": "0x0000000000000000000000000000000000000001"},
        "tokenOut": {"address": "0x0000000000000000000000000000000000000002"},
        "quote": {"slippagePct": 0.1, "priceImpactPct": 0.1},
        "tx": {
            "from": "0x0000000000000000000000000000000000000000",
            "to": "0x0000000000000000000000000000000000000003",
            "data": "0x",
        },
    }
    ctx, findings = validate_check_input(payload)
    ev = evidence("LOW", simulation_data={"success": False, "revertReason": "transfer failed"})
    result = evaluate(ctx, ev, findings)
    assert result["verdict"] == "block"
    assert any(reason["code"] == "SIMULATION_REVERT" for reason in result["reasons"])


def test_slippage_thresholds():
    ctx_warn, findings_warn = context("swap", slippage=4)
    result_warn = evaluate(ctx_warn, evidence("LOW"), findings_warn)
    assert result_warn["verdict"] == "warn"
    assert any(reason["code"] == "SLIPPAGE_ELEVATED" for reason in result_warn["reasons"])

    ctx_block, findings_block = context("swap", slippage=6)
    result_block = evaluate(ctx_block, evidence("LOW"), findings_block)
    assert result_block["verdict"] == "block"
    assert any(reason["code"] == "SLIPPAGE_HIGH" for reason in result_block["reasons"])


def test_scan_timeout_warns_not_allow():
    ctx, findings = context("swap")
    ev = {
        "tokenScan": {"status": "timeout"},
        "tokenReport": {"status": "skipped"},
        "txScan": {"status": "skipped"},
        "simulation": {"status": "skipped"},
    }
    result = evaluate(ctx, ev, findings)
    assert result["verdict"] == "warn"
    assert result["verdict"] != "allow"
    assert any(reason["code"] == "SCAN_TIMEOUT" for reason in result["reasons"])


def test_solana_evm_address_mismatch_blocks():
    payload = {
        "chain": "solana",
        "operation": "swap",
        "walletAddress": "0x0000000000000000000000000000000000000000",
        "tokenIn": {"address": "So11111111111111111111111111111111111111112"},
        "tokenOut": {"address": "11111111111111111111111111111111"},
        "quote": {"slippagePct": 0.1, "priceImpactPct": 0.1},
    }
    ctx, findings = validate_check_input(payload)
    result = evaluate(ctx, evidence("LOW"), findings)
    assert result["verdict"] == "block"
    assert any(reason["code"] == "ADDRESS_CHAIN_MISMATCH" for reason in result["reasons"])
