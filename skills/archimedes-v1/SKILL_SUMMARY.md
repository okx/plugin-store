
# archimedes-v1 -- Skill Summary

## Overview
Archimedes Finance is an Ethereum mainnet leveraged-yield protocol that allows users to deposit stablecoins (USDC, USDT, DAI) and receive up to 10x leveraged exposure to OUSD yield. Each position is represented as an ERC-721 NFT, making positions tradeable and easily manageable. The protocol charges origination fees in ARCH tokens and includes comprehensive safety mechanisms like slippage protection and liquidity checks.

## Usage
Connect your wallet with `onchainos wallet login`, then use commands like `archimedes open-position --amount 1000 --token USDC --cycles 5` to create leveraged positions. Always dry-run first with `--dry-run` flag to simulate transactions before execution.

## Commands
| Command | Description |
|---------|-------------|
| `open-position` | Open a leveraged yield position with specified amount, token, and cycles |
| `close-position` | Close/unwind a position by NFT token ID |
| `get-positions` | List all positions for a wallet |
| `protocol-info` | Show current protocol parameters and available liquidity |

## Triggers
Activate when users want to open leveraged OUSD positions, manage existing Archimedes positions, or check protocol status and available leverage. Key phrases include "archimedes position", "leveraged yield", "OUSD leverage", and "close archimedes".
