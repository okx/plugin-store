---
name: x402-guard
description: "AI Bot Spending Firewall — enforce per-bot spending limits, get real-time alerts, and kill rogue agents instantly on X Layer."
version: "1.0.0"
author: "Mintttch"
tags:
  - security
  - spending-limits
  - x-layer
  - onchainos
  - ai-agents
---

# x402 Guard — AI Bot Spending Firewall

## Overview

x402 Guard is a spending firewall that sits between your AI agents and the blockchain. Every payment must pass through the guard before execution — enforcing per-bot daily limits, per-transaction caps, and domain rules. Every decision is logged immutably to **GuardLog.sol on X Layer**.

**ALWAYS call `guard_check` before any payment.** If the action is `block`, do NOT proceed.

---

## Pre-flight Setup

### 1. Start x402 Guard backend

```bash
git clone https://github.com/mintttch2/x402-guard
cd x402-guard/backend
pip install -r requirements.txt
python3 main.py
# Guard API now running at http://localhost:8000
```

### 2. Add to your agent's MCP config (.mcp.json)

```json
{
  "mcpServers": {
    "x402-guard": {
      "command": "python3",
      "args": ["/path/to/x402-guard/mcp_server.py"],
      "env": {
        "X402_GUARD_URL": "http://localhost:8000"
      }
    }
  }
}
```

### 3. Register your bot

```bash
curl -X POST http://localhost:8000/policies/ \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "my-trading-bot",
    "name": "My Trading Bot",
    "bot_type": "sniper",
    "daily_limit": 100.0,
    "hourly_limit": 30.0,
    "per_tx_limit": 15.0,
    "wallet_address": "0xYourAgenticWalletAddress"
  }'
```

---

## Tools

### guard_check

**ALWAYS call before any payment.** Returns `approve`, `soft_alert`, or `block`.

```
Input:
  agent_id   (required) — your bot's unique ID, e.g. "sniper-bot-1"
  amount     (required) — payment amount in USD
  pay_to     (required) — recipient address or domain
  network    (optional) — default: "eip155:1952" (X Layer testnet)
  asset      (optional) — default: USDC on X Layer

Output:
  action          — "approve" | "soft_alert" | "block"
  allowed         — true | false
  reason          — explanation
  remaining_daily — USD remaining today
  remaining_hourly — USD remaining this hour
```

**Decision rules:**
- `approve` → proceed with payment
- `soft_alert` → warn user, but can proceed (>80% of limit used)
- `block` → DO NOT proceed, limit exceeded

**Example flow:**
```python
# Before every payment:
result = guard_check(agent_id="my-bot", amount=12.5, pay_to="0xRecipient")
if result["action"] == "block":
    raise Exception(f"Payment blocked: {result['reason']}")
# Only proceed if approved
execute_payment(amount=12.5, recipient="0xRecipient")
```

---

### guard_register

Register a new bot with spending limits.

```
Input:
  agent_id       (required) — unique bot ID
  name           (required) — display name, e.g. "Sniper Bot #1"
  daily_limit    (required) — max USD per day
  per_tx_limit   (required) — max USD per single transaction
  hourly_limit   (optional) — max USD per hour
  wallet_address (optional) — OKX Agentic Wallet EVM address
  bot_type       (optional) — sniper | arbitrage | prediction | sentiment | custom
```

---

### guard_stats

Get current spending stats for a bot.

```
Input:  agent_id

Output:
  daily_spent      — USD spent today
  hourly_spent     — USD spent this hour
  daily_limit      — configured daily limit
  remaining_daily  — USD remaining today
  total_transactions, approved_transactions, blocked_transactions
```

---

### guard_kill

Emergency kill switch — immediately blocks all transactions for a bot.

```
Input:  agent_id
Effect: Sets policy active=false, all future guard_check calls return "block"
```

---

## Complete Integration Example

```python
import requests
import uuid

GUARD_URL = "http://localhost:8000"

def safe_pay(agent_id: str, amount: float, recipient: str) -> dict:
    """Call guard before every payment. Raises if blocked."""
    r = requests.post(f"{GUARD_URL}/guard/check", json={
        "agent_id":   agent_id,
        "amount":     amount,
        "pay_to":     recipient,
        "asset":      "0xcB8BF24c6cE16Ad21D707c9505421a17f2bec79D",
        "network":    "eip155:1952",
        "request_id": str(uuid.uuid4()),
    })
    result = r.json()

    if result["action"] == "block":
        raise Exception(f"[GUARD] Payment BLOCKED: {result['reason']}")
    if result["action"] == "soft_alert":
        print(f"[GUARD] Warning: {result['reason']} (remaining: ${result['remaining_daily']:.2f})")

    return result

# Usage in your bot:
guard_result = safe_pay("sniper-bot-1", 12.5, "0xDEXRouter")
# Only reaches here if approved or soft_alert
execute_onchain_payment(amount=12.5, recipient="0xDEXRouter")
```

---

## Dashboard

Monitor all agents at `http://localhost:3000`:
- Real-time transaction feed
- Per-agent spending progress bars
- Block reasons chart
- Emergency kill-all button

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `action: block` | Limit exceeded | Do NOT pay. Wait for reset or adjust policy. |
| `action: soft_alert` | >80% of limit used | Warn user, can still proceed |
| `No active policy found` | Bot not registered | Call guard_register first |
| Connection refused | Backend not running | Start with `python3 main.py` |

---

## Security

- x402 Guard does NOT hold private keys
- All signing happens in your OKX Agentic Wallet
- Guard only approves/blocks — never executes
- Every decision logged to GuardLog.sol on X Layer (immutable)
- Contract: `0x295A3807ea95c69d835B44C6DaBA994C8580ef01`

---

## Links

- GitHub: https://github.com/mintttch2/x402-guard
- Dashboard: http://localhost:3000 (after setup)
- Contract: https://www.oklink.com/x-layer-testnet/address/0x295A3807ea95c69d835B44C6DaBA994C8580ef01
- OKX Build X Hackathon: https://www.moltbook.com/m/buildx
