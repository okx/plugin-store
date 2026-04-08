
# fluid
Fluid Protocol DEX + Lending integration for supply/withdraw to ERC-4626 fTokens and swap via Fluid AMM across Base, Ethereum, and Arbitrum.

## Highlights
- ERC-4626 fToken lending with yield earning (fUSDC, fWETH, fGHO, fEURC)
- Concentrated AMM DEX for token swaps (EURC/USDC, wstETH/ETH, weETH/ETH)
- Multi-chain support across Base, Ethereum, and Arbitrum
- Direct on-chain data via resolver contracts with no external APIs
- Safe dry-run simulation before all write operations
- Automatic approval handling for ERC-20 deposits
- Real-time lending positions and market data
- Liquidation-risk protection with vault operations limited to dry-run only

---SEPARATOR---

# fluid -- Skill Summary

## Overview
The Fluid Protocol skill provides comprehensive access to Instadapp's Fluid ecosystem, combining DeFi lending and DEX functionality. Users can supply assets to earn yield through ERC-4626 fTokens, swap tokens via the concentrated AMM, and manage their lending positions across Base, Ethereum, and Arbitrum networks. All operations use direct on-chain calls for maximum reliability and transparency.

## Usage
Run `fluid markets` to view available lending opportunities, use `fluid positions` to check your current holdings, and execute `fluid supply --ftoken fUSDC --amount 10` to start earning yield. Always use `--dry-run` first for write operations.

## Commands
- `fluid markets` - List fToken lending markets and rates
- `fluid positions` - View your current lending positions
- `fluid supply --ftoken <TOKEN> --amount <N>` - Supply assets to earn yield
- `fluid withdraw --ftoken <TOKEN> --amount <N>` - Withdraw from lending positions
- `fluid swap --token-in <IN> --token-out <OUT> --amount-in <N>` - Swap via Fluid DEX
- `fluid quote --token-in <IN> --token-out <OUT> --amount-in <N>` - Get swap quotes
- `fluid --dry-run borrow/repay` - Simulate vault operations (dry-run only)

## Triggers
Activate when users want to earn yield on crypto assets, swap tokens with minimal slippage, or manage DeFi lending positions. Look for phrases like "fluid yield," "supply to fluid," "swap on fluid dex," or "fluid lending positions."
