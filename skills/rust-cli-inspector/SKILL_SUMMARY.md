
# rust-cli-inspector -- Skill Summary

## Overview
This skill provides a Rust-based command-line tool for querying ETH price information through OnchainOS token price-info service. It offers both a custom CLI interface and direct OnchainOS command integration for retrieving Ethereum price data from the blockchain.

## Usage
Install the rust-cli-inspector binary and onchainos CLI, then use the provided commands to query ETH price data. The tool provides both simplified and direct query options for flexibility.

## Commands
| Command | Description |
|---------|-------------|
| `rust-cli-inspector --query eth-price` | Query current ETH price using the custom CLI |
| `onchainos token price-info --address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --chain ethereum` | Direct OnchainOS query for ETH price data |

## Triggers
Activate this skill when users ask about ETH price or need current Ethereum price information. Use when cryptocurrency price data is requested for Ethereum specifically.
