## Overview

One-Click Token Launch is a multi-launchpad token creation skill that deploys a new token with optional bundled initial buy, IPFS metadata upload, and MEV protection in a single command across 6 launchpads on Solana and BSC.

Core operations:

- Deploy tokens on 6 launchpads: pump.fun, LetsBonk, Bags.fm, Moonit, Four.meme, Flap.sh
- Bundle an initial buy with the launch in one atomic transaction
- Upload token image and description to IPFS (Pinata or pump.fun IPFS)
- Submit via Jito bundle for MEV protection on Solana launches
- Return explorer link (Solscan or BscScan) upon successful launch

Tags: `token-launch` `meme-coin` `solana` `bsc` `pump.fun` `launchpad` `onchainos`

## Prerequisites

- No IP/region restrictions (check local regulations on token issuance)
- Supported chains: Solana, BSC
- Supported launchpads: pump.fun, LetsBonk, Bags.fm, Moonit, Four.meme, Flap.sh
- onchainos CLI installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- Sufficient SOL or BNB for launchpad fees, initial buy, and gas
- (Optional) Pinata API key for IPFS metadata fallback

## Quick Start

1. **Install the skill**: `plugin-store install one-click-token-launch`
2. **Configure your token**: Edit `config.py` with your token name, symbol, image path, description, and initial buy amount
3. **Launch your token**: Tell your agent "Launch a token on pump.fun" or "一键发币"
4. **Review cost summary**: The skill shows total estimated fees (launchpad + gas + Jito) before executing
5. **Confirm and deploy**: The skill uploads metadata to IPFS, deploys the token, and submits the bundled buy
6. **Verify on-chain**: Check the returned Solscan or BscScan link to confirm your token is live
