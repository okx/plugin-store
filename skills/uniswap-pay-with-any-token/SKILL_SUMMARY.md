# uniswap-pay-with-any-token — Skill Summary

## Overview
The uniswap-pay-with-any-token skill enables AI agents to autonomously satisfy HTTP 402 Payment Required responses using any ERC-20 token held in their wallet. When an API or service returns a 402 challenge, this skill invokes the Uniswap Trading API to swap the agent's available token into the required payment token, then uses the Tempo CLI to complete settlement via the MPP (Machine Payment Protocol) or x402 protocol. The swap and payment steps are coordinated so no manual intervention is needed.

## Usage
Install via `claude plugin add @uniswap/uniswap-trading`. The skill activates automatically when the agent encounters an HTTP 402 response. Ensure the Tempo CLI is installed and a funded wallet is configured; the skill handles token selection and the swap-then-pay sequence.

## Commands
| Tool | Description |
|---|---|
| `tempo pay` | Tempo CLI command to settle an MPP/x402 payment challenge |
| Trading API `POST /quote` | Get swap quote for token-to-payment-token conversion |
| Trading API `POST /swap` | Execute the token swap before payment |

## Triggers
Activates when the agent receives an HTTP 402 Payment Required response, when the user mentions paying with a token via Uniswap, or when setting up machine-to-machine payment flows using MPP or x402 protocols.
