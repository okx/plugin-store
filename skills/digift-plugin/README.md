# digift-plugin

DigiFT RWA platform CLI — query tokenized real-world asset products, check prices, fees,
settlement calendars, whitelist status, and build unsigned subscribe/redeem/approve
transactions for on-chain execution via Onchain OS.

## What it does

- **Data queries**  — products, prices, fees, parameters, calendar,
  whitelist, orders. All data from the REST API.
- **On-chain operations**  — building subscribe/redeem/approve
  transactions. Outputs standardized `TxBody` payloads, **never signs or broadcasts**.

All contract addresses, token addresses, and chain configurations come from the API —
query first, act second.

## Setup

### API Key

A DigiFT API key is required for all commands. To obtain one:

1. Register and complete KYC at [digift.io](https://digift.io)
2. Generate an API key from your account dashboard
3. Export it:

```bash
export DIGIFT_API_KEY=<your-api-key>
```

## Commands

### Query

| Command | Description |
|---------|-------------|
| `digift products` | List all available products |
| `digift chains` | Platform chain info, contract addresses, currencies |
| `digift whitelist <address> --chain <chain-id>` | Check wallet whitelist status |
| `digift info <tokencode>` | Product chain info (token address, precision) |
| `digift issuance <tokencode>` | Issuance details (issuer, ISIN) |
| `digift sub-params <tokencode>` | Subscription parameters (fee, min, max, increment) |
| `digift red-params <tokencode>` | Redemption parameters |
| `digift calendar <tokencode> [--type sub\|red]` | Trading calendar & settlement cycle |
| `digift price <tokencode>` | Current price & yield |
| `digift price-history <tokencode>` | Historical prices |
| `digift order <txhash>` | Lookup order by transaction hash |
| `digift orders <address> [--project <project>] [--size <n>]` | List orders for wallet |

### On-Chain

On-chain commands output a `TxBody` payload. Sign and broadcast via Onchain OS.

| Command | Description |
|---------|-------------|
| `digift balance <token> <address> --chain <chain-id> [--rpc <rpc-url>]` | Check token balance |
| `digift subscribe --product <tokencode> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]` | Build subscribe tx |
| `digift redeem --product <tokencode> --quantity <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]` | Build redeem tx |
| `digift approve --token <address> --spender <address> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>]` | Build ERC20 approve tx |

## Quick Start

```bash
export DIGIFT_API_KEY=<your-key>

digift products
digift price <tokencode>
digift subscribe --product <tokencode> --amount 1000 --from <address> --chain 1
```

## Security

- This plugin **never signs or broadcasts** transactions — it only builds unsigned `TxBody` payloads.
- Signing and broadcasting are handled by Onchain OS.
- No private keys are ever handled or accessed.
