# morpho-base

Morpho V1 on Base — permissionless isolated lending on the Base network.

This plugin lets OnchainOS users supply assets to MetaMorpho vaults, borrow from Morpho Blue markets, and manage positions on Base via natural language.

## Supported Chain

- **Base** (chain ID: 8453)

## Commands

| Command | Description |
|---------|-------------|
| `supply` | Supply assets to a MetaMorpho vault (ERC-20 approve + ERC-4626 deposit) |
| `withdraw` | Withdraw assets from a MetaMorpho vault (ERC-4626 withdraw/redeem) |
| `borrow` | Borrow from a Morpho Blue market |
| `repay` | Repay debt to a Morpho Blue market |
| `supply-collateral` | Supply collateral to a Morpho Blue market |
| `markets` | List Morpho Blue markets with rates and TVL (read-only) |
| `vaults` | List MetaMorpho vaults with APY and TVL (read-only) |
| `positions` | View wallet's active positions (read-only) |
| `claim-rewards` | Claim Merkl rewards |

## Architecture

**Binary:** Rust (`morpho-base`), ABI encoding via manual hex construction  
**Data sources:** Morpho Blue GraphQL API (`https://blue-api.morpho.org/graphql`) for markets/vaults/positions; direct `eth_call` for on-chain reads

- Write ops: ERC-4626 or Morpho Blue calldata encoded by binary → submitted via `onchainos wallet contract-call --force`
- Wallet address resolved via `onchainos wallet balance --chain 8453` (never defaulted to zero address)
- Read ops: Morpho Blue GraphQL API + direct `eth_call`

## Key Contracts (Base, chain 8453)

| Contract | Address |
|----------|---------|
| Morpho Blue | `0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb` |
| Merkl Distributor | `0x3Ef3D8bA38EBe18DB133cEc108f4D14CE00Dd9Ae` |
| Steakhouse USDC | `0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183` |
| Moonwell Flagship USDC | `0xc1256Ae5FF1cf2719D4937adb3bbCCab2E00A2Ca` |
