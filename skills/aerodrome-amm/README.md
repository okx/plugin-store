# aerodrome-amm

Aerodrome Finance classic AMM (volatile + stable pools) plugin for [Plugin Store](https://github.com/okx/plugin-store-community).

Supports swap, quote, pools, positions, add-liquidity, remove-liquidity, and claim-rewards on Base (chain ID 8453).

## Distinction from aerodrome-slipstream

This plugin targets **Aerodrome's classic AMM** (volatile/stable pools), which uses `bool stable` to identify pool types and ERC-20 LP tokens. This is separate from `aerodrome-slipstream` (CLMM, concentrated liquidity, NFT positions with `tickSpacing`).

| Feature | aerodrome-amm (this) | aerodrome-slipstream |
|---------|---------------------|---------------------|
| Pool type | Volatile + Stable | Concentrated Liquidity (CLMM) |
| Router | `0xcF77a3Ba...` | `0xBE6D8f0d...` |
| LP Token | ERC-20 | ERC-721 NFT |
| Pool ID | `bool stable` | `int24 tickSpacing` |

## Commands

```
aerodrome-amm quote        - Get swap quote (read-only)
aerodrome-amm swap         - Swap tokens via classic AMM
aerodrome-amm pools        - Query pool addresses and reserves
aerodrome-amm positions    - View LP token balances
aerodrome-amm add-liquidity    - Add LP to a pool
aerodrome-amm remove-liquidity - Remove LP from a pool
aerodrome-amm claim-rewards    - Claim AERO gauge emissions
```

## Contracts (Base mainnet)

- Router: `0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43`
- PoolFactory: `0x420DD381b31aEf6683db6B902084cB0FFECe40Da`
- Voter: `0x16613524E02ad97eDfeF371bC883F2F5d6C480A5`

## Usage

See [skills/aerodrome-amm/SKILL.md](skills/aerodrome-amm/SKILL.md) for full command documentation.
