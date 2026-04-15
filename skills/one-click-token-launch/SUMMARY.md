# one-click-token-launch
One-click multi-launchpad token creation with bundled initial buy, IPFS metadata upload, and MEV protection across Solana and BSC.

## Highlights
- One-call `quick_launch()` entry point — wallet, IPFS, signing, broadcast all handled automatically
- 6 launchpad adapters: pump.fun, Bags.fm, LetsBonk, Moonit (Solana) + Four.Meme, Flap.sh (BSC)
- IPFS upload via pump.fun free endpoint (no API key needed), Pinata fallback
- Bundled initial buy: create + buy in ONE atomic Jito bundle (no front-running)
- Image input: file path, URL, base64, or data URI — auto-normalized
- MEV protection via Jito bundles (Solana) and atomic contract calls (BSC)
- Ephemeral mint keypair generated client-side (pump.fun)
- onchainos Agentic Wallet TEE signing — no private keys in code
- Paper Mode default (DRY_RUN=True), Live Mode requires explicit confirmation
- Web dashboard at localhost:3245 with token logos, bonding curve progress, live stats
- Post-launch monitor with real-time price, holder count, liquidity tracking
- Hot-reload config — modify config.py without restarting
