---
name: bpb-lite
description: "Public AI-agent workflow for Based Pings Bot, focused on Solana automated signals, volume insights, platform attribution, and compact research output"
version: "0.1.0"
author: "solgoodman"
tags:
  - solana
  - memecoin
  - signals
  - analytics
---

# BPB Lite

## Overview

BPB means **Based Pings Bot**.

BPB is a Solana memecoin ping and signal product built around early-entry detection, momentum recognition, and quality filtering. BPB Lite is the public plugin-style workflow that exposes the intelligence layer without exposing sensitive thresholds or private delivery infrastructure.

Use it when the user wants a fast answer about:

- whether a token still looks early
- whether momentum is expanding in a meaningful way
- what launch source or platform context the token belongs to
- what market structure supports or weakens the setup

## What the BPB system publicly does

BPB can be described publicly as a system that:

1. reads volume injections and movement across meaningful time windows
2. detects early runner and continuation behavior
3. checks liquidity, holder concentration, and token structure
4. tracks smart-money context using more than **1,000 wallets**
5. reads **Streamflow locks** as part of token-quality context
6. separates launch source from downstream liquidity venue when needed
7. supports subscriber-facing alert filters by setup style and quality

## How BPB thinks

Keep the explanation public-safe but strong:

- start with **volume behavior and timing windows**
- explain that **momentum has shape**, not just size
- describe that BPB checks whether structure is strong enough to justify a ping
- mention wallets, locks, and platform context as confidence-building layers
- keep the signal framing anchored to **EARLY / MOMENTUM / CONFIRMATION**
- treat **RISE** as platform context, not signal language

## When to use BPB Lite

Use BPB Lite when the user says things like:

- "scan this CA"
- "is this still early?"
- "what's the signal here?"
- "what launchpad is this from?"
- "why would BPB ping this?"

## Pre-flight checks

Before using this skill:

1. Confirm the input is a Solana contract address.
2. Prefer stable market and attribution sources first.
3. If some fields are unavailable, return the best compact read you can instead of failing.
4. Keep the output signal-oriented, not just descriptive.
5. Distinguish between **signal language** and **platform language**.

## Core workflow

1. Resolve token identity from the contract address.
2. Pull market context such as MC, liquidity, volume, and age.
3. Add holder or concentration context when available.
4. Detect launch source or venue context when possible.
5. Frame the setup using signal language:
   - EARLY
   - MOMENTUM
   - CONFIRMATION
6. Add confidence framing.
7. Return a short verdict explaining the setup.
8. Include research links for operator follow-up.

## Platform and launch context

Support public hints across the launchpads and venues BPB cares about, including:

- Pumpfun
- Bonk
- LaunchLab
- Printr
- Meteora
- DBC
- Raydium
- RISE

Rules to preserve:

- Treat **Printr** as a launch source when direct metadata confirms it.
- Treat **Meteora** as a liquidity venue when the evidence points there.
- Treat **RISE** as platform context, not a signal tier.
- Keep launch-source claims separate from liquidity-venue claims when both matter.

## Output shape

A strong BPB Lite response should usually include:

- token name and symbol
- contract address
- platform or launch context
- confidence tier
- MC, liquidity, volume, holders, age
- smart-holder or wallet-context mention when available
- signal framing: EARLY, MOMENTUM, or CONFIRMATION
- evidence bullets
- short verdict
- research links

## Guardrails

- Do not expose private trigger thresholds.
- Do not expose private re-ping, routing, or suppression mechanics.
- Do not present the system as random or purely reactive.
- Keep the explanation focused on what BPB is able to observe: momentum, volume, structure, locks, and wallet behavior.
- If attribution is incomplete, say the evidence is partial rather than writing weak filler like "unknown is preferred."

## References

See `references/data-sources.md` for the trust matrix and source posture.

## Local packaging note

For OKX Plugin Store style local setup:

```bash
npx skills add okx/plugin-store --skill plugin-store
```
