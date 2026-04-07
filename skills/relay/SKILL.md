---
name: relay
description: Bridge and swap assets across 74+ EVM chains using the Relay cross-chain protocol. Supports listing chains, currencies, getting bridge quotes, executing bridge transfers, and monitoring bridge status.
---

# Relay Cross-Chain Bridge Plugin

## Overview

Relay is a solver-based cross-chain bridge supporting 74+ EVM chains. The solver network provides instant, low-cost settlement by fronting liquidity on the destination chain while processing the user's deposit on the source chain.

**Key facts:**
- Supports 74+ EVM chains including Ethereum, Base, Arbitrum, Optimism, Polygon, BNB Chain, and more
- ETH-to-ETH bridge from Base to Ethereum takes ~30 seconds
- No approve step required for native ETH transfers
- All write operations require user confirmation before submission

## Architecture

- Read ops (chains, currencies, quote, status) → direct REST calls to `https://api.relay.link`
- Write ops (bridge) → fetch quote to get step data, then submit via `onchainos wallet contract-call`

## Pre-flight Checks

Before running any command:
1. Verify `onchainos` is installed: `onchainos --version` (requires ≥ 2.0.0)
2. For write operations, verify wallet is logged in: `onchainos wallet balance --chain 8453 --output json`
3. If wallet check fails, prompt: "Please log in with `onchainos wallet login` first."

## Key Addresses

The Relay relayer/router contract address is returned dynamically per quote in `steps[].items[].data.to`. No hardcoded contract addresses needed.

---

## Commands

### `chains` — List Supported Chains

List all EVM chains supported by Relay.

**Usage:**
```
relay chains [--filter <NAME>]
```

**Parameters:**
| Parameter | Required | Description |
|-----------|----------|-------------|
| `--filter` | No | Filter by chain name (partial match) |

**Example:**
```bash
relay chains
relay chains --filter base
```

**No onchainos command required** — pure REST API call to `GET https://api.relay.link/chains`.

---

### `currencies` — List Supported Tokens

List supported tokens/currencies on a specific chain.

**Usage:**
```
relay currencies --chain <CHAIN_ID> [--limit <N>]
```

**Parameters:**
| Parameter | Required | Description |
|-----------|----------|-------------|
| `--chain` | Yes | Chain ID (e.g. 8453 for Base) |
| `--limit` | No | Max tokens to show (default: 20) |

**Example:**
```bash
relay currencies --chain 8453
relay currencies --chain 1 --limit 10
```

**No onchainos command required** — REST API call to `POST https://api.relay.link/currencies/v1`.

---

### `quote` — Get Bridge Quote

Get a quote for bridging assets, including fees and estimated time.

**Usage:**
```
relay quote --from-chain <ID> --to-chain <ID> --token <SYMBOL> --amount <AMOUNT> [--from <ADDR>] [--recipient <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|-----------|----------|-------------|
| `--from-chain` | Yes | Source chain ID (e.g. 8453) |
| `--to-chain` | Yes | Destination chain ID (e.g. 1) |
| `--token` | No | Token symbol (ETH) or address (default: ETH) |
| `--amount` | Yes | Amount in token units (e.g. 0.001) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--recipient` | No | Recipient on destination chain (defaults to sender) |

**Steps:**
1. Resolve wallet address from onchainos (or use --from)
2. Convert amount to wei
3. POST to `https://api.relay.link/quote`
4. Display fees breakdown (gas, relayer, service) and estimated received amount

**Example:**
```bash
relay quote --from-chain 8453 --to-chain 1 --token ETH --amount 0.001
```

**No onchainos command required** — pure read operation.

---

### `bridge` — Execute Bridge Transfer

Bridge assets from one chain to another. **Ask user to confirm before submitting.**

**Usage:**
```
relay bridge --from-chain <ID> --to-chain <ID> --token <SYMBOL> --amount <AMOUNT> [--from <ADDR>] [--recipient <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|-----------|----------|-------------|
| `--from-chain` | Yes | Source chain ID (e.g. 8453) |
| `--to-chain` | Yes | Destination chain ID (e.g. 1) |
| `--token` | No | Token symbol or address (default: ETH) |
| `--amount` | Yes | Amount in token units (e.g. 0.001) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--recipient` | No | Recipient on destination chain (defaults to sender) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Resolve wallet address
2. Call `POST https://api.relay.link/quote` to get execution steps
3. Extract step 0 (deposit): `steps[0].items[0].data` → `{ to, data, value }`
4. Display full fee/amount breakdown to user
5. **Ask user to confirm** the bridge transaction before submitting
6. Execute: `onchainos wallet contract-call --chain <from-chain> --to <step.to> --input-data <step.data> --amt <step.value> --force`
7. Return transaction hash and requestId for status monitoring

**Example:**
```bash
# Dry run to preview
relay bridge --from-chain 8453 --to-chain 1 --token ETH --amount 0.001 --dry-run

# Execute bridge
relay bridge --from-chain 8453 --to-chain 1 --token ETH --amount 0.001
```

**WARNING:** Bridge transfers are irreversible once submitted. Always verify destination chain and recipient address before confirming.

**Follow-up:** After bridge submission, monitor with `relay status --request-id <ID>`.

---

### `status` — Check Bridge Status

Check the status of a bridge transaction by request ID.

**Usage:**
```
relay status --request-id <REQUEST_ID>
```

**Parameters:**
| Parameter | Required | Description |
|-----------|----------|-------------|
| `--request-id` | Yes | Bridge request ID (from quote or bridge output) |

**Status values:**
| Status | Meaning |
|--------|---------|
| `waiting` | Transaction received, awaiting on-chain confirmation |
| `pending` | Bridge in progress, solver processing |
| `success` | Bridge completed, funds delivered |
| `failed` | Bridge failed |
| `refunded` | Transaction was refunded to sender |
| `unknown` | Request not found or not yet indexed |

**Example:**
```bash
relay status --request-id 0x89427b056f4f4caf464cc4318bc55d8ad07bc9708290f864d9b22f98478b8768
```

**No onchainos command required** — REST API call to `GET https://api.relay.link/intents/status`.

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Cannot get wallet address" | Not logged in to onchainos | Run `onchainos wallet login` |
| "API error: ..." | Invalid request params | Check chain IDs and token addresses |
| "No steps in quote response" | Quote API error | Retry or check amount limits |
| "Missing 'to' address in step data" | Unexpected API response | Retry request |
| HTTP timeout | Network issue | Retry after a few seconds |

## Suggested Follow-ups

After **quote**: suggest executing with `relay bridge` using same parameters.

After **bridge**: suggest monitoring with `relay status --request-id <ID>`. Expected time ~30s for ETH transfers.

After **status success**: suggest checking destination chain balance via `onchainos wallet balance --chain <to-chain>`.

## Skill Routing

- For token swaps on same chain → use chain-specific DEX plugins (uniswap, aerodrome, etc.)
- For wallet balance queries → use `onchainos wallet balance`
- For SOL or non-EVM chains → Relay does not support these; use chain-specific bridges
