# Supported Chains and Tokens

This file summarizes chain IDs explicitly enumerated in the Across API schema. The canonical, always-current list is the API itself (see endpoints below).

## Mainnet Chains (per API schema)

| Chain | Chain ID |
|-------|----------|
| Ethereum | 1 |
| Optimism | 10 |
| Polygon | 137 |
| zkSync Era | 324 |
| Base | 8453 |
| Arbitrum | 42161 |
| Linea | 59144 |

## Testnet Chains (per API schema)

| Chain | Chain ID |
|-------|----------|
| Ethereum Sepolia | 11155111 |
| Arbitrum Sepolia | 421614 |
| Base Sepolia | 84532 |
| Optimism Sepolia | 11155420 |

## Discovering Tokens and Routes

### Supported chains (Swap API)
```
GET /swap/chains
```
Returns all chains that support crosschain swaps.

### Supported tokens (Swap API)
```
GET /swap/tokens
```
Returns all whitelisted tokens across all chains. Tokens must be whitelisted by swap sources to appear here.

### Supported sources (Swap API)
```
GET /swap/sources
GET /swap/sources?chainId={id}
```
Returns supported swap sources globally or for a specific chain.

### Available routes (bridge-only)
```
GET /available-routes?originChainId={id}&destinationChainId={id}
```
Returns all supported bridgeable token pairs between two chains, including token symbols and addresses on each chain. This only includes bridgeable tokens.

## Common Token Addresses (reference examples)

These are commonly used token addresses for building API calls. Always verify against `GET /swap/tokens` or `GET /available-routes` before production use — addresses can change as token contracts are upgraded.

### USDC
| Chain | Address |
|-------|---------|
| Ethereum (1) | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` |
| Arbitrum (42161) | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |
| Base (8453) | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| Optimism (10) | `0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85` |
| Polygon (137) | `0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359` |

### WETH (use this instead of ETH)
| Chain | Address |
|-------|---------|
| Ethereum (1) | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` |
| Arbitrum (42161) | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` |
| Base (8453) | `0x4200000000000000000000000000000000000006` |
| Optimism (10) | `0x4200000000000000000000000000000000000006` |

### USDT
| Chain | Address |
|-------|---------|
| Ethereum (1) | `0xdAC17F958D2ee523a2206206994597C13D831ec7` |
| Arbitrum (42161) | `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9` |

### DAI
| Chain | Address |
|-------|---------|
| Ethereum (1) | `0x6B175474E89094C44Da98b954EedeAC495271d0F` |
| Arbitrum (42161) | `0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1` |
| Optimism (10) | `0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1` |

### WBTC
| Chain | Address |
|-------|---------|
| Ethereum (1) | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` |
| Arbitrum (42161) | `0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f` |

## Token Address Notes
- **Always use wrapped native token addresses** when querying APIs (WETH, not ETH; WMATIC, not MATIC).
- Token addresses are **chain-specific** — the same token has different addresses on different chains.
- Amounts are always in the token's **native decimals** (USDC = 6 decimals, WETH = 18 decimals, DAI = 18 decimals, WBTC = 8 decimals).
- When in doubt, query the API. The addresses above are for reference only.

## Contract Addresses
Full contract addresses for all chains (mainnet and testnet) are maintained at:
https://docs.across.to/reference/contract-addresses

## Notes
- The API is the source of truth for current chain and token support.
- Chain lists expand over time. Use the endpoints above for the latest data.
