from agent_risk_firewall.models import validate_check_input
from agent_risk_firewall.policy import evaluate, get_policy


def payload(
    operation="swap",
    chain="xlayer",
    token_in=None,
    token_out=None,
    quote=None,
    amount_usd=10,
    wallet_value_usd=None,
    tx=None,
    approval=None,
    external_evidence=None,
    competition=None,
    profile="balanced",
):
    base = {
        "chain": chain,
        "operation": operation,
        "walletAddress": "0x0000000000000000000000000000000000000000",
        "tokenIn": token_in
        or {
            "address": "0x0000000000000000000000000000000000000001",
            "symbol": "USDC",
            "decimals": 6,
        },
        "tokenOut": token_out
        or {
            "address": "0x0000000000000000000000000000000000000002",
            "symbol": "RISK",
            "decimals": 18,
        },
        "amountIn": "10",
        "amountInUsd": amount_usd,
        "quote": quote or {"slippagePct": 0.5, "priceImpactPct": 0.5},
        "policyProfile": profile,
    }
    if wallet_value_usd is not None:
        base["walletValueUsd"] = wallet_value_usd
    if tx is not None:
        base["tx"] = tx
    if approval is not None:
        base["approval"] = approval
    if external_evidence is not None:
        base["externalEvidence"] = external_evidence
    if competition is not None:
        base["competition"] = competition
    return base


def safe_evidence():
    return {
        "tokenScan": {"status": "ok", "data": {"riskLevel": "LOW"}},
        "tokenReport": {"status": "ok", "data": {"liquidityUsd": 500000}},
        "txScan": {"status": "skipped"},
        "simulation": {"status": "skipped"},
    }


def run(payload_data, evidence=None):
    ctx, findings = validate_check_input(payload_data)
    return evaluate(ctx, evidence or safe_evidence(), findings, ctx["policyProfile"])


def test_policy_profiles_are_available():
    assert get_policy("strict")["maxTradeUsd"] == 100
    assert get_policy("competition")["disallowStableNativeOnlyPair"] is True
    assert get_policy("degen-small-size")["slippageBlockPct"] == 12


def test_strict_profile_blocks_scan_unavailable():
    ctx, findings = validate_check_input(payload(profile="strict"))
    result = evaluate(
        ctx,
        {
            "tokenScan": {"status": "unavailable"},
            "tokenReport": {"status": "skipped"},
            "txScan": {"status": "skipped"},
            "simulation": {"status": "skipped"},
        },
        findings,
        "strict",
    )
    assert result["verdict"] == "block"
    assert any(reason["code"] == "SCAN_UNAVAILABLE" for reason in result["reasons"])


def test_competition_profile_blocks_stable_native_only_pair():
    result = run(
        payload(
            profile="competition",
            token_in={
                "address": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                "symbol": "OKB",
                "decimals": 18,
            },
            token_out={
                "address": "0x779ded0c9e1022225f8e0630b35a9b54be713736",
                "symbol": "USDT",
                "decimals": 6,
            },
        )
    )
    assert result["verdict"] == "block"
    assert any(reason["code"] == "COMPETITION_PAIR_NOT_ELIGIBLE" for reason in result["reasons"])


def test_competition_profile_warns_without_competition_context():
    result = run(payload(profile="competition"))

    assert result["verdict"] == "warn"
    assert any(reason["code"] == "COMPETITION_CONTEXT_MISSING" for reason in result["reasons"])


def test_competition_profile_blocks_inactive_competition():
    result = run(payload(profile="competition", competition={"active": False, "joined": True, "supportedChains": ["xlayer"]}))

    assert result["verdict"] == "block"
    assert any(reason["code"] == "COMPETITION_INACTIVE" for reason in result["reasons"])


def test_competition_profile_blocks_unsupported_chain():
    result = run(payload(profile="competition", competition={"active": True, "joined": True, "supportedChains": ["solana"]}))

    assert result["verdict"] == "block"
    assert any(reason["code"] == "COMPETITION_CHAIN_UNSUPPORTED" for reason in result["reasons"])


def test_competition_profile_warns_when_not_joined():
    result = run(payload(profile="competition", competition={"active": True, "joined": False, "supportedChains": ["xlayer", "solana"]}))

    assert result["verdict"] == "warn"
    assert any(reason["code"] == "COMPETITION_NOT_JOINED" for reason in result["reasons"])


def test_competition_profile_warns_below_competition_thresholds():
    result = run(
        payload(
            profile="competition",
            amount_usd=10,
            wallet_value_usd=150,
            competition={
                "active": True,
                "joined": True,
                "chainName": "X Layer",
                "minParticipationUsd": 25,
                "minLeaderboardUsd": 100,
                "minWalletBalanceUsd": 200,
                "eligibleTokenTradeRequired": True,
            },
        )
    )

    assert result["verdict"] == "warn"
    codes = {reason["code"] for reason in result["reasons"]}
    assert "COMPETITION_VOLUME_BELOW_PARTICIPATION_MIN" in codes
    assert "COMPETITION_VOLUME_BELOW_LEADERBOARD_MIN" in codes
    assert "COMPETITION_BALANCE_BELOW_MIN" in codes


def test_competition_profile_allows_real_token_trade_with_context():
    result = run(
        payload(
            profile="competition",
            amount_usd=10,
            competition={
                "active": True,
                "joined": True,
                "chainName": "X Layer",
                "minParticipationUsd": 5,
                "eligibleTokenTradeRequired": True,
            },
        )
    )

    assert result["verdict"] == "allow"
    assert not any(reason["code"].startswith("COMPETITION_") for reason in result["reasons"])


def test_degen_small_size_allows_higher_slippage_but_blocks_large_trade():
    result = run(payload(profile="degen-small-size", quote={"slippagePct": 6, "priceImpactPct": 1}))
    assert result["verdict"] == "warn"
    assert any(reason["code"] == "SLIPPAGE_ELEVATED" for reason in result["reasons"])

    large = run(payload(profile="degen-small-size", amount_usd=26))
    assert large["verdict"] == "block"
    assert any(reason["code"] == "TRADE_CAP_EXCEEDED" for reason in large["reasons"])


def test_external_goplus_honeypot_blocks_and_is_preserved_in_evidence():
    result = run(
        payload(
            external_evidence={
                "goplus": {
                    "riskLevel": "HIGH",
                    "is_honeypot": "1",
                    "buy_tax": "10",
                    "sell_tax": "10",
                }
            }
        )
    )
    assert result["verdict"] == "block"
    assert result["evidence"]["externalEvidence"]["goplus"]["is_honeypot"] == "1"
    assert any(reason["code"] == "EXTERNAL_GOPLUS_HONEYPOT" for reason in result["reasons"])


def test_external_birdeye_holder_concentration_warns():
    result = run(payload(external_evidence={"birdeye": {"top10HolderPercent": 75, "liquidityUsd": 200000}}))
    assert result["verdict"] == "warn"
    assert any(reason["code"] == "EXTERNAL_HOLDER_CONCENTRATION_ELEVATED" for reason in result["reasons"])


def test_external_rootdata_scam_tag_blocks():
    result = run(payload(external_evidence={"rootdata": {"tags": ["scam"]}}))
    assert result["verdict"] == "block"
    assert any(reason["code"] == "EXTERNAL_ROOTDATA_CRITICAL" for reason in result["reasons"])


def test_approval_unlimited_eoa_warns_with_balanced_policy():
    result = run(
        payload(
            operation="approval",
            token_in={
                "address": "0x0000000000000000000000000000000000000001",
                "symbol": "USDC",
                "decimals": 6,
            },
            token_out=None,
            approval={
                "spender": "0x0000000000000000000000000000000000000004",
                "spenderType": "eoa",
                "isUnlimited": True,
            },
        )
    )
    assert result["verdict"] == "warn"
    assert any(reason["code"] == "APPROVAL_UNLIMITED" for reason in result["reasons"])
    assert any(reason["code"] == "APPROVAL_SPENDER_EOA" for reason in result["reasons"])


def test_approval_blocked_spender_blocks():
    result = run(
        payload(
            operation="approval",
            approval={
                "spender": "0x0000000000000000000000000000000000000004",
                "blockedSpenders": ["0x0000000000000000000000000000000000000004"],
            },
        )
    )
    assert result["verdict"] == "block"
    assert any(reason["code"] == "APPROVAL_SPENDER_BLOCKED" for reason in result["reasons"])


def test_audit_trail_is_deterministic_and_contains_hashes():
    request = payload()
    first = run(request)
    second = run(request)
    assert first["audit"] == second["audit"]
    assert first["audit"]["decisionId"].startswith("arf_")
    assert first["audit"]["policyVersion"] == "1.2.0"
    assert len(first["audit"]["evidenceHash"]) == 64
