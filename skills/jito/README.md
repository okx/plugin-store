# Jito Liquid Staking Plugin

Jito is a MEV-enhanced liquid staking protocol on Solana. Users stake SOL to receive JitoSOL, which earns both validator staking rewards and MEV rewards from Jito's block engine.

## Features

- **rates** — Query current SOL↔JitoSOL exchange rate and APY from on-chain pool state + DeFiLlama
- **positions** — View JitoSOL balance and SOL equivalent value
- **stake** — Deposit SOL to receive JitoSOL (DepositSol instruction on SPL Stake Pool)
- **unstake** — Redeem JitoSOL for a stake account (delayed, unlocks after epoch ~2-3 days)

## Chain

Solana mainnet (chain ID: 501)

## Key Addresses

| Contract | Address |
|----------|---------|
| Jito Stake Pool | `Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb` |
| JitoSOL Mint | `J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn` |
| SPL Stake Pool Program | `SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy` |

## Usage

```bash
# Query rates
jito rates --chain 501

# Query positions
jito positions --chain 501

# Stake 0.001 SOL (dry-run first)
jito stake --amount 0.001 --chain 501 --dry-run
jito stake --amount 0.001 --chain 501

# Unstake 0.005 JitoSOL (dry-run preview)
jito unstake --amount 0.005 --chain 501 --dry-run
```

## Architecture

This plugin directly interacts with the Solana SPL Stake Pool program on-chain:
- Read operations use Solana JSON-RPC (`getAccountInfo`, `getTokenAccountBalance`)
- Write operations construct SPL Stake Pool instructions, serialize as unsigned transactions, and broadcast via `onchainos wallet contract-call --unsigned-tx`
- PDA derivation (withdraw authority, ATA) uses SHA256 + Ed25519 curve check

## License

MIT
