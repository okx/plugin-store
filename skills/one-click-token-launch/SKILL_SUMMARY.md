# one-click-token-launch — Skill Summary

## Overview
Token Launch (一键发币) is a multi-launchpad token creation tool supporting 6 platforms across Solana and BSC. The `quick_launch()` function provides a single async entry point that handles wallet initialization, balance check, image normalization (file/URL/base64), IPFS metadata upload (pump.fun free endpoint, Pinata fallback), bundled initial buy with Jito MEV protection, and on-chain submission — all via onchainos Agentic Wallet TEE signing. Paper Mode (DRY_RUN=True) is the default. A web dashboard at `http://localhost:3245` displays launch history with token logos, bonding curve progress, and live stats. Post-launch monitoring tracks price, holders, and liquidity in real-time.

## Usage
Run the AI startup protocol: the agent presents the strategy overview, asks for launchpad choice (pump.fun default), collects token details (name, symbol, description, image), asks about bundled initial buy amount, confirms Paper/Live mode, shows a confirmation summary, then launches via `python3 -c "from token_launch import quick_launch; ..."` or the dashboard UI. Prerequisites: onchainos CLI >= 2.1.0, `onchainos wallet login`, and `pip install -r requirements.txt`.

## Commands
| Command | Description |
|---|---|
| `quick_launch(name, symbol, desc, image, ...)` | One-call async token launch (primary entry point) |
| `python3 token_launch.py` | Start the dashboard server on port 3245 |
| `python3 post_launch.py <token_address>` | Post-launch monitor (price, holders, liquidity) |
| `onchainos wallet login` | Authenticate the TEE agentic wallet |

## Triggers
Activates when the user mentions 一键发币, 发币, 创建代币, launch token, create token, deploy token, mint token, launch meme coin, one-click-token-launch, pump.fun launch, or wants to create a new token on Solana or BSC launchpads.
