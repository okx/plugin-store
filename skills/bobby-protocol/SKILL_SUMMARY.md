# bobby-protocol — Skill Summary

## Overview
Bobby Protocol is an adversarial AI trading intelligence system on OKX X Layer. Three AI agents (Alpha Hunter, Red Team, CIO) debate every trade, a Judge Mode audits decisions on 6 dimensions, and anyone can stake OKB to prove Bobby was wrong via on-chain adversarial bounties. It exposes 15 MCP tools over Streamable HTTP — 10 free for discovery, 5 premium gated by x402 on-chain payment (0.001 OKB each).

## Usage
Invoke Bobby's tools via MCP Streamable HTTP at `https://bobbyprotocol.xyz/api/mcp-http`. Free tools require no setup. Premium tools follow the x402 payment flow: call → receive challenge → pay on X Layer → retry with proof.

## Commands
| Tool | Type | Purpose |
|------|------|---------|
| bobby_intel | Free | 10-source market intelligence briefing |
| bobby_ta | Free | Technical analysis (SMA, RSI, MACD, Bollinger) |
| bobby_stats | Free | On-chain track record (win rate, PnL) |
| bobby_analyze | Premium | Full 3-agent debate with conviction score |
| bobby_debate | Premium | Complete adversarial debate transcript |
| bobby_judge | Premium | 6-dimension debate quality audit |
| bobby_security_scan | Premium | Token honeypot/rug detection |
| bobby_wallet_portfolio | Premium | Multi-chain portfolio analysis |
| bobby_bounty_list | Free | List open adversarial bounties |
| bobby_bounty_post | Free | Build unsigned bounty calldata |

## Triggers
Activates when the user asks about trading analysis, market intelligence, adversarial debate, conviction scoring, token security, portfolio review, or wants to challenge AI trading decisions on-chain.
