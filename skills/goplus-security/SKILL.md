---
name: goplus-security
description: "Run read-only Web3 security checks with GoPlus Security APIs"
version: "1.0.0"
author: "GoPlus Security"
tags:
  - security
  - goplus
  - token-risk
  - phishing
  - approval-risk
  - web3
---

# GoPlus Security

## Overview

Use this skill to run read-only Web3 security checks with GoPlus Security APIs. It supports EVM token security, malicious address detection, phishing website detection, NFT contract security, token approval risk, Solana token security, and Sui token security.

This skill never signs transactions, broadcasts transactions, transfers assets, manages approvals, or handles private keys. It only queries GoPlus APIs and summarizes security data for the user.

## When to Use

Use this skill when the user asks to:

- Check whether a token contract is risky, a honeypot, mintable, pausable, blacklisted, or overly centralized.
- Check whether an address is malicious, blacklisted, suspicious, a contract, or related to scams.
- Check whether a crypto website or DApp URL is phishing.
- Check whether NFT contracts have security risks.
- Check token approvals for risk visibility.
- Analyze Solana or Sui token security.

Do not use this skill for swaps, transfers, transaction signing, transaction simulation, gas estimation, portfolio balance, or market price queries. Route those requests to the relevant wallet, swap, gateway, portfolio, market, or onchain security skill.

## Pre-flight Checks

Before calling GoPlus APIs:

1. Confirm the request is read-only.
2. Confirm `GOPLUS_API_KEY` and `GOPLUS_API_SECRET` are available in the shell environment.
3. Never print, summarize, or expose credential values.
4. Use `curl` for HTTP requests. Use `jq` if available for formatting, but do not require it.
5. If the user provides a wallet address, URL, or contract address, treat it as user-provided data and only send it to `api.gopluslabs.io` for the requested check.

Credential check:

```bash
test -n "$GOPLUS_API_KEY" && test -n "$GOPLUS_API_SECRET"
```

If credentials are missing, ask the user to set them:

```bash
export GOPLUS_API_KEY="your_app_key"
export GOPLUS_API_SECRET="your_app_secret"
```

## Authentication

GoPlus API requests require an access token. Generate a SHA-1 signature from:

```text
app_key + unix_timestamp_seconds + app_secret
```

Then call the token endpoint. The `Authorization` header for subsequent API calls is the raw access token value returned by GoPlus. Do not prefix it with `Bearer`.

Get an access token:

```bash
TIME="$(date +%s)"
if command -v shasum >/dev/null 2>&1; then
  SIGN="$(printf "%s%s%s" "$GOPLUS_API_KEY" "$TIME" "$GOPLUS_API_SECRET" | shasum | awk '{print $1}')"
else
  SIGN="$(printf "%s%s%s" "$GOPLUS_API_KEY" "$TIME" "$GOPLUS_API_SECRET" | sha1sum | awk '{print $1}')"
fi

GOPLUS_ACCESS_TOKEN="$(
  curl -sS -X POST "https://api.gopluslabs.io/api/v1/token" \
    -H "Content-Type: application/json" \
    -d "{\"app_key\":\"$GOPLUS_API_KEY\",\"time\":\"$TIME\",\"sign\":\"$SIGN\"}" \
  | sed -n 's/.*"access_token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p'
)"

test -n "$GOPLUS_ACCESS_TOKEN"
```

If token extraction fails, show the API error message without exposing credentials. If `jq` is installed, prefer this extraction:

```bash
GOPLUS_ACCESS_TOKEN="$(
  curl -sS -X POST "https://api.gopluslabs.io/api/v1/token" \
    -H "Content-Type: application/json" \
    -d "{\"app_key\":\"$GOPLUS_API_KEY\",\"time\":\"$TIME\",\"sign\":\"$SIGN\"}" \
  | jq -r '.result.access_token // empty'
)"
```

## Commands

### EVM Token Security

Analyze one or more EVM token contracts.

```bash
CHAIN_ID="1"
CONTRACT_ADDRESSES="0x0000000000000000000000000000000000000000"

curl -sS "https://api.gopluslabs.io/api/v1/token_security/$CHAIN_ID?contract_addresses=$CONTRACT_ADDRESSES" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

Use this for ERC-20 or EVM-compatible token risk analysis. Common chain IDs include Ethereum `1`, BSC `56`, Polygon `137`, Arbitrum `42161`, Optimism `10`, Base `8453`, Avalanche `43114`, and X Layer `196`.

Summarize high-signal fields when present:

- `is_honeypot`
- `cannot_buy`
- `cannot_sell_all`
- `buy_tax`, `sell_tax`, `transfer_tax`
- `is_open_source`
- `is_proxy`
- `is_mintable`
- `hidden_owner`
- `can_take_back_ownership`
- `owner_change_balance`
- `transfer_pausable`
- `is_blacklisted`
- `holder_count`
- holder and LP concentration fields
- DEX and liquidity fields

### Malicious Address Check

Check one or more EVM addresses for security labels and malicious activity.

```bash
CHAIN_ID="1"
ADDRESSES="0x0000000000000000000000000000000000000000"

curl -sS "https://api.gopluslabs.io/api/v1/address_security/$ADDRESSES?chain_id=$CHAIN_ID" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

Use this when the user asks if an address is malicious, suspicious, blacklisted, related to scams, or safe to interact with.

### Phishing Website Check

Check whether a URL is flagged as phishing or has website contract security issues.

```bash
URL_TO_CHECK="$USER_PROVIDED_URL"

curl -sS "https://api.gopluslabs.io/api/v1/phishing_site?url=$URL_TO_CHECK" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

Use this before a user connects a wallet to a DApp, clicks an unknown crypto link, or reviews a suspicious website.

### NFT Security

Analyze one or more EVM NFT contracts. Add `token_id` when the user requests a specific NFT token.

```bash
CHAIN_ID="1"
CONTRACT_ADDRESSES="0x0000000000000000000000000000000000000000"

curl -sS "https://api.gopluslabs.io/api/v1/nft_security/$CHAIN_ID?contract_addresses=$CONTRACT_ADDRESSES" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

With token ID:

```bash
TOKEN_ID="1"

curl -sS "https://api.gopluslabs.io/api/v1/nft_security/$CHAIN_ID?contract_addresses=$CONTRACT_ADDRESSES&token_id=$TOKEN_ID" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

### Approval Security

Analyze token approval risk for one or more addresses on an EVM chain.

```bash
CHAIN_ID="1"
ADDRESSES="0x0000000000000000000000000000000000000000"

curl -sS "https://api.gopluslabs.io/api/v1/approval_security/$CHAIN_ID?contract_addresses=$ADDRESSES" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

This is read-only. If risky approvals are found, explain the risk and tell the user that revocation is a separate wallet/transaction action requiring explicit user confirmation through an appropriate transaction-capable tool.

### Solana Token Security

Analyze one or more Solana token mints.

```bash
CONTRACT_ADDRESSES="So11111111111111111111111111111111111111112"

curl -sS "https://api.gopluslabs.io/api/v1/solana/token_security?contract_addresses=$CONTRACT_ADDRESSES" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

Summarize mint authority, freeze authority, close authority, balance mutability, metadata mutability, transfer fees/hooks, non-transferable status, default account state, DEX/liquidity data, and holder concentration when present.

### Sui Token Security

Analyze one or more Sui token contracts.

```bash
CONTRACT_ADDRESSES="0x2::sui::SUI"

curl -sS "https://api.gopluslabs.io/api/v1/sui/token_security?contract_addresses=$CONTRACT_ADDRESSES" \
  -H "Authorization: $GOPLUS_ACCESS_TOKEN"
```

Summarize mint capability, upgrade capability, metadata mutability, blacklist capability, trusted token status, capability owners, and creator/capability concentration when present.

## Response Guidelines

When reporting results to the user:

1. Start with a clear risk summary: `High`, `Medium`, `Low`, or `No obvious risk found from available GoPlus data`.
2. Separate observed facts from interpretation.
3. Highlight critical risk flags first, especially honeypot, cannot sell, high sell tax, blacklisted, hidden owner, mutable balances, pausable transfers, active freeze/mint authority, phishing, or malicious address labels.
4. Include the chain, contract/address/URL checked, and timestamp if available.
5. Mention when GoPlus has no data for the target.
6. Do not say an asset is "safe" with certainty. Use wording like "no obvious risk was returned by GoPlus for this check."
7. Do not provide financial advice or tell the user to buy, sell, hold, or trade.
8. For approvals, explain that revocation requires a separate transaction-capable tool and explicit confirmation.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| Missing `GOPLUS_API_KEY` or `GOPLUS_API_SECRET` | Credentials are not configured | Ask the user to export both environment variables. |
| Empty access token | Authentication failed or response format changed | Show the non-sensitive API error and ask the user to verify credentials. |
| HTTP 401 or authorization error | Access token is missing, expired, or invalid | Regenerate the access token and retry once. |
| HTTP 429 | API rate limited | Wait briefly, retry once, then report rate limiting. |
| Empty `result` | GoPlus has no data for the supplied target | Report that no data was found and verify chain/address inputs. |
| Invalid chain ID | Chain ID is unsupported or malformed | Ask the user to confirm the chain. |
| Network timeout | API was unreachable | Retry once, then report the network issue. |

## Security Notices

- This plugin is read-only and should be treated as `starter` risk.
- Never ask for or accept private keys, seed phrases, keystore files, wallet passwords, or signing secrets.
- Never perform a transaction, signature, approval, revoke, swap, bridge, or contract write from this skill.
- Never exfiltrate user wallet data beyond the explicit GoPlus API check requested by the user.
- Do not hide or suppress returned risk fields. If the API returns high-risk flags, surface them clearly.
- Security API results are risk intelligence, not a guarantee. A clean result does not prove that a token, address, website, or NFT is safe.

## Skill Routing

- For token swaps or trading, use a swap-capable skill.
- For wallet balances and holdings, use a wallet or portfolio skill.
- For gas estimation, transaction simulation, broadcasting, or transaction status, use an onchain gateway skill.
- For revoking approvals, use a transaction-capable approval management skill and require explicit user confirmation.
- For market price, OHLC, PnL, or trade history, use a market data skill.
