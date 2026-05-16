from safe_competition_trading_agent.candidate_selector import is_real_competition_token, select_candidate


def test_select_candidate_prefers_liquid_low_risk_real_token():
    request = {
        "tokenOut": None,
        "candidates": [
            {"symbol": "USDC", "liquidityUsd": "1000000", "riskLevel": "LOW"},
            {"symbol": "MEME", "liquidityUsd": "150000", "riskLevel": "LOW", "volume24hUsd": "75000"},
            {"symbol": "BAD", "riskLevel": "HIGH", "tags": ["honeypot"]},
        ],
    }

    selected = select_candidate(request)

    assert selected["symbol"] == "MEME"
    assert selected["selectionScore"] > 50


def test_real_competition_token_excludes_stable_native():
    assert is_real_competition_token({"symbol": "MEME"}) is True
    assert is_real_competition_token({"symbol": "USDC"}) is False
    assert is_real_competition_token({"symbol": "SOL"}) is False
