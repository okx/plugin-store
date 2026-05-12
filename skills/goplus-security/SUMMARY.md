# GoPlus Security

## Overview

GoPlus Security enables AI agents to run read-only Web3 security checks through GoPlus Security APIs. It covers token risk analysis, malicious address checks, phishing website detection, NFT contract security, token approval risk, and Solana/Sui token security checks.

## Prerequisites

- GoPlus API credentials.
- `GOPLUS_API_KEY` and `GOPLUS_API_SECRET` exported in the shell environment.
- Standard command-line tools: `curl`, `date`, `printf`, and either `shasum` or `sha1sum`.

## Quick Start

Set credentials:

```bash
export GOPLUS_API_KEY="your_app_key"
export GOPLUS_API_SECRET="your_app_secret"
```

Ask the agent to check a token, address, website, NFT contract, approval risk, Solana token, or Sui token. This plugin is read-only and never signs transactions, broadcasts transactions, transfers assets, or handles private keys.
