# Aave V3 Skill — Summary

## What This Skill Does

This skill lets OnchainOS users interact with **Aave V3**, the leading decentralized lending protocol ($43B+ TVL), using natural language.

Users can:
- **Earn yield** by supplying assets (USDC, WETH, cbBTC, etc.) and receiving aTokens
- **Borrow** assets against their collateral at variable interest rates
- **Monitor** portfolio health factor and liquidation risk in real-time
- **Manage** collateral settings and efficiency mode (E-Mode)
- **Claim** accrued protocol rewards

**Supported chains:** Ethereum, Polygon, Arbitrum One, Base (default)

---

## Implemented Operations

| Command | Description | On-Chain Calls |
|---------|-------------|----------------|
| `supply` | Deposit assets to earn yield (aTokens) | ERC-20 approve + Pool.supply() |
| `withdraw` | Redeem aTokens for underlying | Pool.withdraw() |
| `borrow` | Borrow against posted collateral | Pool.borrow() |
| `repay` | Repay variable-rate debt | ERC-20 approve + Pool.repay() |
| `health-factor` | Check HF, collateral, debt (read-only) | eth_call getUserAccountData |
| `reserves` | List APYs and market rates (read-only) | eth_call getReservesList + getReserveData |
| `positions` | Full portfolio snapshot (read-only) | onchainos defi positions |
| `set-collateral` | Enable/disable asset as collateral | Pool.setUserUseReserveAsCollateral() |
| `set-emode` | Set efficiency mode category | Pool.setUserEMode() |
| `claim-rewards` | Claim protocol/platform rewards | onchainos defi collect |

---

## Architecture

**Binary:** `aave-v3` (Rust, built with `alloy-sol-types` for ABI encoding)

**Data sources:** On-chain RPC only — no external API keys required.

```
User prompt
    ↓
SKILL.md routing
    ↓
aave-v3 binary
    ├── Read ops (health-factor, reserves): eth_call via public RPC
    ├── Write ops: ABI calldata encoded with alloy-sol-types
    │       ├── ERC-20 approve: onchainos wallet contract-call → token contract
    │       └── Pool action: onchainos wallet contract-call → Pool address
    └── Pool address resolved at runtime via PoolAddressesProvider.getPool()
```

Key design choices:
- **Pool address is NEVER hardcoded** — always resolved at runtime via `PoolAddressesProvider.getPool()`. This is mandatory because the Pool address can change on upgrades.
- **No external API keys** — all market data (APYs, reserves) is fetched directly from on-chain state.
- **Dry-run support** on all write operations — shows full calldata before broadcasting.
- **Health factor safety checks** built into borrow and set-collateral commands.

---

## On-Chain Test Evidence

All operations were tested on **Base mainnet** (chain ID: 8453) on 2026-04-04.

| Operation | Tx Hash |
|-----------|---------|
| set-emode (disable E-Mode) | [0xc2334ff7...](https://basescan.org/tx/0xc2334ff718f949505e29a1bc951d42ef570bf00b892114afa89d8467e5ea4594) |
| supply 1.0 USDC — approve | [0xf8477dc5...](https://basescan.org/tx/0xf8477dc58d43ef2ce0c578c5a811bd9b7c394e03a8b8c27dbb505c2d5c8f41ce) |
| supply 1.0 USDC — deposit | [0x7e26f856...](https://basescan.org/tx/0x7e26f856230f17e82111cf0afe64d736bdfc1e4827a49c6723547b554962d153) |
| set-collateral (enable USDC) | [0xde876c26...](https://basescan.org/tx/0xde876c261e1c8a6b3478ebca0dcf45b5bb8797da2a00c51453bb8ebfa6c9d582) |
| borrow 0.0001 WETH | [0x2bb2e54e...](https://basescan.org/tx/0x2bb2e54e4032a11805991689e6170dbf34b9ee74d4d501f696998e37c756dd5e) |
| repay 0.0001 WETH | [0x738ddef5...](https://basescan.org/tx/0x738ddef5886c6bf1a0a9da51b2264a8782fed8eeeeb9f231cf2bc8767467e6ed) |
| withdraw 0.9 USDC | [0xd8b8be63...](https://basescan.org/tx/0xd8b8be63cb4db9bc64bb5b8db4f2d516aad3cc1cf6b1db29f197af9c0cd35f19) |
| claim-rewards | No tx — no claimable rewards (correct behavior) |

Full test details: [`tests/test_results.md`](./tests/test_results.md)
