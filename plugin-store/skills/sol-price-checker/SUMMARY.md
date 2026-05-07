## Overview

Minimal Go plugin that wraps `onchainos token price-info` to return the current SOL price on Solana. Built primarily to validate the OKOne Go build pipeline.

## Prerequisites

- onchainos CLI installed and an active session — `onchainos wallet status` reports `loggedIn: true`
- The plugin binary `sol-price-checker` available on PATH

## Quick Start

```bash
sol-price-checker
```

Returns the JSON `data` payload from onchainos. Stderr-prints diagnostic messages on failure and exits non-zero.

## Implementation Notes

- Pure stdlib Go — no third-party dependencies
- Wraps a single onchainos command:
  `onchainos token price-info --address So11111111111111111111111111111111111111112 --chain solana`
- Native SOL is represented by the wrapped-SOL mint address `So111…1112`, matching onchainos' convention for SOL price queries

_Pipeline retrigger: validate Go path._
