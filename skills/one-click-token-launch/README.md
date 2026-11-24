# Token Launch (一键发币)

One-click multi-launchpad token creation — supports pump.fun, Bags.fm, LetsBonk, Moonit on Solana and Four.Meme, Flap.sh on BSC. Bundled initial buy with MEV protection, IPFS metadata upload, and post-launch monitoring. All on-chain operations powered by onchainos Agentic Wallet (TEE signing, no private keys needed).

一键发币工具 — 支持 pump.fun、Bags.fm、LetsBonk、Moonit（Solana）及 Four.Meme、Flap.sh（BSC）。原子捆绑买入 + Jito MEV 保护，IPFS 元数据上传，发币后实时监控。全部链上操作通过 onchainos Agentic Wallet TEE 签名，无需私钥。

## Features

- **6 Launchpads** — pump.fun, Bags.fm, LetsBonk, Moonit (Solana) + Four.Meme, Flap.sh (BSC)
- **One-Call Launch** — `quick_launch()` handles wallet, IPFS, signing, broadcast automatically
- **Bundled Buy** — Create + buy in ONE atomic Jito bundle, no front-running
- **IPFS Upload** — pump.fun free endpoint (no API key), Pinata fallback
- **Flexible Image Input** — File path, URL, base64, data URI
- **MEV Protection** — Jito bundles (Solana), atomic contract calls (BSC)
- **TEE Signing** — onchainos Agentic Wallet, private keys never leave secure enclave
- **Paper Mode** — DRY_RUN=True by default, safe to test
- **Web Dashboard** — Token logos, bonding curve progress, live stats at http://localhost:3245
- **Post-Launch Monitor** — Real-time price, holders, liquidity tracking
- **Hot-Reload** — Modify config.py without restarting

## Install

```bash
npx skills add okx/plugin-store --skill one-click-token-launch
```

## Prerequisites

```bash
# 1. onchainos CLI >= 2.1.0
onchainos --version

# 2. Login to Agentic Wallet
onchainos wallet login <your-email>

# 3. Python dependencies
pip install -r requirements.txt
```

## Risk Warning

> Token creation is irreversible. Launched tokens may fail, lose all liquidity, or face regulatory scrutiny. Always test in Paper Mode (DRY_RUN=True) first. This tool is for educational and research purposes only — not investment advice.

> 代币创建不可逆。已发行的代币可能失败、失去所有流动性或面临监管审查。请始终先在模拟模式 (DRY_RUN=True) 下测试。本工具仅供教育和研究用途，不构成投资建议。

## License

MIT
