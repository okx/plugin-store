# bobby-protocol

Adversarial AI trading intelligence on OKX X Layer. Three agents debate every trade, a Judge Mode audits on 6 dimensions, and anyone can stake OKB to prove Bobby was wrong.

## Installation

```bash
npx skills add okx/plugin-store --skill bobby-protocol
```

## What it does

Bobby Protocol exposes 15 MCP tools over Streamable HTTP that any AI agent can consume:

- **10 free tools** — market intel, technical analysis, signals, bounty queries
- **5 premium tools** — full analysis, 3-agent debate, judge audit, security scan, portfolio analysis
- Premium tools settle at 0.001 OKB per call via x402 on X Layer (Chain 196)

### Key endpoints

| Endpoint | Purpose |
|----------|---------|
| `POST /api/mcp-http` | MCP Streamable HTTP (JSON-RPC 2.0) |
| `GET /api/reputation` | On-chain track record |
| `GET /api/registry` | Agent catalog + tool list |
| `GET /api/activity` | Live call feed |

### Contracts on X Layer

- AgentEconomyV2: `0xD9540D770C8aF67e9E6412C92D78E34bc11ED871`
- ConvictionOracle: `0x03FA39B3a5B316B7cAcDabD3442577EE32Ab5f3A`
- TrackRecord: `0xF841b428E6d743187D7BE2242eccC1078fdE2395`
- AdversarialBounties: `0xa8005ab465a0e02cb14824cd0e7630391fba673d` (verified)

## Links

- Website: https://bobbyprotocol.xyz
- GitHub: https://github.com/anthonysurfermx/Bobby-Agent-Trader
- Skill file: https://bobbyprotocol.xyz/skill.md

## License

MIT
