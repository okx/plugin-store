---
name: smart-tradex
description: "AI-powered smart money copy trader with safety checks, stop-loss, and dry-run mode for Solana and X Layer"
version: "1.0.0"
author: "Kane"
tags:
  - trading
  - copy-trade
  - smart-money
  - solana
  - safety
  - stop-loss
---

# Smart-TradeX

## Overview

Smart-TradeX is an AI-powered copy trading skill that monitors smart money and whale wallet signals, performs automatic security checks on tokens before any trade, and executes copy trades on Solana and X Layer with built-in stop-loss protection. It always asks the user to confirm before executing any real transaction and supports a dry-run mode for safe testing without risking funds.

## Risk Disclaimer

This skill involves automated trading and carries financial risk. Never trade more than you can afford to lose. Always start with dry-run mode to understand the strategy before using real funds. The author is not responsible for any financial losses.

## Pre-flight Checks

Before using this skill, ensure:

1. The onchainos CLI is installed and configured by running: npx skills add okx/onchainos-skills
2. Add onchainos to your PATH if not found: export PATH="$HOME/.local/bin:$PATH"
3. Your Agentic Wallet is set up and funded with the amount you want to trade
4. You have read the Risk Disclaimer above

## Modes

### Dry-Run Mode (Default - Recommended for beginners)
Simulates all trades without executing real transactions. Use this to test the strategy safely.

### Live Mode
Executes real trades after user confirmation at every step. Never trades automatically without approval.

## Commands

### Step 1: Scan Smart Money Signals

Fetch the latest smart money and whale wallet activity on Solana or X Layer.

Run: onchainos signal list --chain solana
Or:  onchainos signal list --chain xlayer

When to use: Always run this first to identify what tokens smart money is buying.
Output: List of tokens being accumulated by top traders with wallet addresses and trade sizes.

### Step 2: Check Token Security

Before considering any trade, scan the token for rug pull risk and security issues.

Run: onchainos security token-scan --address TOKEN_ADDRESS --chain solana

When to use: Run this on every token found in Step 1 before proceeding.
Output: Security score, honeypot detection, liquidity lock status, contract risks.
Rules:
- If security score is below 70, STOP and warn the user. Do not proceed.
- If honeypot is detected, STOP immediately and warn the user.
- Only proceed if the token passes all critical checks.

### Step 3: Get Token Market Data

Run: onchainos market price --address TOKEN_ADDRESS --chain solana
Run: onchainos market kline --address TOKEN_ADDRESS --chain solana --interval 1H

When to use: After the token passes security checks, analyze market conditions.
Output: Current price, 24h change, volume, price chart data.
Rules:
- If 24h price change is above +50%, warn the user the token may already be pumped.
- Present the data clearly to the user before asking for confirmation.

### Step 4: Check Wallet Balance

Run: onchainos wallet balance --chain solana
Run: onchainos portfolio all-balances --address WALLET_ADDRESS --chain solana

When to use: Before every trade to confirm available balance.
Output: Available token balances across the wallet.

### Step 5: Get Swap Quote

Run: onchainos swap quote --from USDC --to TOKEN_ADDRESS --amount AMOUNT --chain solana

When to use: After the user confirms they want to proceed with the trade.
Output: Expected output amount, price impact, swap route, fees.
Rules:
- If price impact is above 5%, warn the user strongly before proceeding.
- Always show the full quote details to the user.

### Step 6: User Confirmation (Mandatory)

Before executing any real trade, present a full summary to the user and ask for explicit confirmation. Only proceed if the user explicitly says yes.

### Step 7: Execute Swap (Live Mode Only)

Run: onchainos swap swap --from USDC --to TOKEN_ADDRESS --amount AMOUNT --chain solana --slippage 1

When to use: Only after explicit user confirmation in Step 6.
Output: Transaction hash, actual amount received, final price.
Important: Always save the entry price after execution for stop-loss monitoring.

### Step 8: Monitor Position and Stop-Loss

Run: onchainos market price --address TOKEN_ADDRESS --chain solana

When to use: Periodically after a trade is executed.
Stop-loss logic:
- Default stop-loss is 15% below entry price
- If current price drops below stop-loss level, alert the user immediately
- Ask user if they want to exit the position
- Never automatically sell without user confirmation

## Safety Rules

1. Never execute a trade without explicit user confirmation
2. Never trade a token with a security score below 70
3. Never trade a honeypot token under any circumstances
4. Always show price impact before confirming
5. Always set a stop-loss at entry (default 15%, configurable)
6. Never invest more than the user specifies
7. Always run in dry-run mode by default unless user switches to live mode
8. If any step fails or returns an error, stop the workflow and inform the user

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| onchainos not found | CLI not installed | Run npx skills add okx/onchainos-skills |
| Token not found | Invalid token address | Ask user to verify the contract address |
| Insufficient balance | Not enough funds | Show current balance and ask user to top up |
| Security scan failed | API error | Retry once, if still failing skip that token |
| Price impact too high | Low liquidity | Warn user and suggest reducing trade amount |
| Rate limited | Too many requests | Wait 10 seconds and retry |
| Swap failed | Network or slippage issue | Inform user, suggest retrying with higher slippage |

## Configuration Options

- Chain: solana (default) or xlayer
- Mode: dry-run (default) or live
- Stop-loss: percentage below entry price (default 15%)
- Max trade amount: maximum USDC per trade (default 50 USDC)
- Min security score: minimum score to allow a trade (default 70)

## Skill Routing

- For wallet balances only, use okx-wallet-portfolio skill
- For token security only, use okx-security skill
- For simple swaps without copy trading, use okx-dex-swap skill
- For market data only, use okx-dex-market skill

## Trigger Keywords

The following phrases should activate this skill:

- "copy trade"
- "follow smart money"
- "whale copy"
- "copy whale trades"
- "smart money signals"
- "follow whale wallet"
- "auto copy trade"
- "mirror trade"
- "copy successful traders"
- "trade like whales"
- "follow top traders"
- "smart tradex"
- "start copy trading"
- "find whale signals"
- "what are whales buying"

## Do NOT trigger for

- Simple token price checks (use okx-dex-market instead)
- Basic swaps without copy trading intent (use okx-dex-swap instead)
- Portfolio checks only (use okx-wallet-portfolio instead)
- Security scans only (use okx-security instead)
