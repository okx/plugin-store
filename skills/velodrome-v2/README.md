# velodrome-v2

Velodrome V2 classic AMM plugin for Optimism (chain ID 10).

Swap tokens and manage volatile/stable LP positions on Velodrome V2 — the largest DEX on Optimism.

## Supported Operations

- `quote` — Get swap quote (no transaction)
- `swap` — Swap tokens via Router
- `pools` — Query pool info (reserves, addresses)
- `positions` — View LP token balances
- `add-liquidity` — Add liquidity to volatile or stable pool
- `remove-liquidity` — Remove LP tokens
- `claim-rewards` — Claim VELO gauge emissions

## Chain

Optimism (chain ID: 10)

## Key Contracts

| Contract | Address |
|---------|---------|
| Router | `0xa062aE8A9c5e11aaA026fc2670B0D65cCc8B2858` |
| PoolFactory | `0xF1046053aa5682b4F9a81b5481394DA16BE5FF5a` |
| Voter | `0x41C914ee0c7E1A5edCD0295623e6dC557B5aBf3C` |

## Usage

See `skills/velodrome-v2/SKILL.md` for full documentation.
