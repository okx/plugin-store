# polymarket-agent-skills — Skill Summary

## Overview
The polymarket-agent-skills plugin provides a comprehensive reference for integrating with the Polymarket prediction market protocol on Polygon (chain ID 137). It covers the complete trading lifecycle: L1/L2 authentication, order placement (GTC/GTD/FOK/FAK and batch modes), market data via the CLOB and Gamma APIs, real-time WebSocket streaming for market and user events, CTF conditional token operations (split/merge/redeem), cross-chain asset bridging, and gasless relayer transactions. Modular reference documents (authentication.md, order-patterns.md, market-data.md) are included for deep-dive use.

## Usage
Reference this skill when building agents or bots that interact with Polymarket markets. Use the CLOB API (`clob.polymarket.com`) for order management, the Gamma API for market discovery and pricing, and the WebSocket channels for real-time event feeds. The TypeScript `@polymarket/clob-client` and Python `py-clob-client` SDKs are both supported.

## Commands
This is a reference skill with no CLI commands. Integration is done programmatically via the CLOB REST API and WebSocket endpoints using the provided SDKs.

## Triggers
Activates when the user mentions Polymarket, prediction markets, CLOB trading on Polygon, CTF tokens, or tasks involving conditional market order placement, orderbook data, or cross-chain bridging to Polymarket.
