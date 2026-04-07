# LI.FI / Jumper Plugin

Cross-chain bridge and swap aggregator supporting 79+ EVM chains.

## Features

- List supported chains and tokens
- Get best-route quotes for cross-chain bridges and swaps
- Execute bridges/swaps via LiFiDiamond on-chain
- Check transfer status

## Usage

```bash
lifi get-chains
lifi get-quote --from-chain 8453 --to-chain 42161 --from-token USDC --to-token USDC --amount 5000000
lifi --chain 8453 swap --to-chain 42161 --from-token USDC --to-token USDC --amount 5000000
lifi get-status --tx-hash 0xabc... --from-chain 8453 --to-chain 42161
```
