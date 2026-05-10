## Overview

Minimal Rust plugin that wraps `onchainos token price-info` to return the current SOL price on Solana. Built primarily to validate the OKOne Rust pipeline + GitLab Releases / Generic Packages path.

## Prerequisites

- onchainos CLI installed and an active session — `onchainos wallet status` reports `loggedIn: true`
- The plugin binary `rust-sol-checker` available on PATH

## Quick Start

```bash
rust-sol-checker
```

Forwards stdout from onchainos. Exits non-zero with stderr diagnostics on failure.

## Implementation Notes

- Pure stdlib Rust — no third-party dependencies (avoids cargo network access in offline runner)
- Wraps a single onchainos command:
  `onchainos token price-info --address So11111111111111111111111111111111111111112 --chain solana`
- Native SOL is represented by the wrapped-SOL mint address `So111…1112`, matching onchainos' convention for SOL price queries

_Retrigger: fixed SIGPIPE in TARGET_TRIPLE detection._
