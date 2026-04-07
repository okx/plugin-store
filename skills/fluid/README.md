# Fluid Protocol Plugin

Fluid Protocol (by Instadapp) integration for onchainos — DEX + Lending on Base, Ethereum, and Arbitrum.

## Features

- **Lending** — Supply/withdraw to ERC-4626 fTokens (fUSDC, fWETH, fGHO, fEURC) and earn yield
- **DEX** — Swap tokens via Fluid AMM (EURC/USDC, wstETH/ETH, weETH/ETH, USDe/USDC, FLUID/ETH)
- **Positions** — View your lending positions across all fTokens
- **Markets** — List all fToken markets with supply rates
- **Vault Borrow/Repay** — Dry-run only (liquidation risk protection)

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Base (default) | 8453 |
| Ethereum Mainnet | 1 |
| Arbitrum | 42161 |

## Quick Start

```bash
# List lending markets
fluid --chain 8453 markets

# View your positions
fluid --chain 8453 positions

# Supply 10 USDC to earn yield (dry-run first)
fluid --chain 8453 --dry-run supply --ftoken fUSDC --amount 10
fluid --chain 8453 supply --ftoken fUSDC --amount 10

# Withdraw 5 USDC
fluid --chain 8453 withdraw --ftoken fUSDC --amount 5

# Get a swap quote
fluid --chain 8453 quote --token-in EURC --token-out USDC --amount-in 100

# Swap EURC to USDC (dry-run first)
fluid --chain 8453 --dry-run swap --token-in EURC --token-out USDC --amount-in 100
fluid --chain 8453 swap --token-in EURC --token-out USDC --amount-in 100
```

## Build

```bash
cargo build --release
```

Binary will be at `target/release/fluid`.

## Architecture

- No external API required — all data from on-chain resolver contracts via `eth_call`
- Write operations use `onchainos wallet contract-call --force`
- `resolve_wallet` uses `onchainos wallet addresses` with chainIndex lookup
- ERC-4626 standard for fToken deposit/withdraw/redeem

## Contract Addresses (Base)

| Contract | Address |
|----------|---------|
| LendingResolver | `0x48D32f49aFeAEC7AE66ad7B9264f446fc11a1569` |
| fUSDC | `0xf42f5795D9ac7e9D757dB633D693cD548Cfd9169` |
| fWETH | `0x9272D6153133175175Bc276512B2336BE3931CE9` |
