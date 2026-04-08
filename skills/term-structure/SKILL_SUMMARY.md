
# term-structure -- Skill Summary

## Overview
This skill enables fixed-rate lending and borrowing on TermMax (rebranded Term Structure), which uses a customized Uniswap V3 AMM for continuous fixed-rate operations. Lenders receive FT tokens redeemable at maturity for underlying plus fixed interest, while borrowers receive GT NFTs representing their debt positions. The protocol operates across Arbitrum, Ethereum, and BNB Chain with various collateral markets.

## Usage
Use commands to view markets and positions, then execute lending/borrowing operations. Always check market liquidity with `get-markets` before large transactions, as TVL is limited (~$3.6M total).

## Commands
| Command | Description |
|---------|-------------|
| `get-markets` | List active TermMax markets with current APR and liquidity |
| `get-position` | View current lend (FT) and borrow (GT) positions |
| `lend` | Lend tokens at fixed rate, receive FT bond tokens |
| `borrow` | Borrow tokens by posting collateral, receive GT NFT |
| `repay` | Repay borrow position using GT NFT loanId |
| `redeem` | Redeem FT tokens after maturity for underlying + interest |

## Triggers
Activate when users want fixed-rate lending/borrowing, mention "term structure," "termmax," "fixed rate yield," or need alternatives to variable-rate protocols like Aave for predictable returns.
