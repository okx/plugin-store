
# archimedes-v1 -- Skill Summary

## Overview
Archimedes Finance is an Ethereum mainnet leveraged-yield protocol that allows users to deposit stablecoins (USDC, USDT, DAI) and receive up to 10x leveraged exposure to OUSD (Origin Dollar) yield. Each position is represented as an ERC-721 NFT, enabling users to manage, transfer, and close positions while earning amplified yield on their deposits.

## Usage
Connect your wallet with `onchainos wallet login`, then use commands like `archimedes open-position --amount 1000 --token USDC --cycles 5` to create leveraged positions. Always use `--dry-run` flag first to simulate transactions before execution.

## Commands
| Command | Description |
|---------|-------------|
| `open-position` | Open a leveraged yield position with specified amount, token, and cycles |
| `close-position` | Close and unwind a position by NFT token ID |
| `get-positions` | List all positions for connected wallet |
| `protocol-info` | Show current protocol parameters and available liquidity |

## Triggers
An AI agent should activate this skill when users mention opening leveraged positions, archimedes yield farming, depositing to archimedes protocol, viewing archimedes positions, or managing OUSD leveraged exposure. Also trigger for phrases like "archimedes NFT positions" or "close archimedes position".
