# Renzo Plugin

Renzo EigenLayer liquid restaking plugin for onchainos.

Deposit ETH or stETH into Renzo to receive ezETH (liquid restaking token) and earn EigenLayer AVS rewards.

## Supported Operations

- `deposit-eth` — Deposit native ETH to mint ezETH
- `deposit-steth` — Deposit stETH to mint ezETH (approve + deposit)
- `get-apr` — Get current restaking APR
- `balance` — Check ezETH and stETH balances
- `get-tvl` — Get protocol TVL

## Chain

Ethereum mainnet (chain ID 1)

## Key Contracts

- RestakeManager: `0x74a09653A083691711cF8215a6ab074BB4e99ef5`
- ezETH token: `0xbf5495Efe5DB9ce00f80364C8B423567e58d2110`
