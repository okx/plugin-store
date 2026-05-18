---
title: MCP Server Configuration by Environment
impact: HIGH
impactDescription: "Required for connecting agents to deBridge execution layer"
tags: mcp, setup, streaming, stdio, claude-desktop, cursor, claude-code
---

# deBridge MCP Setup

## Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "debridge": {
      "type": "streamable-http",
      "url": "https://agents.debridge.com/mcp"
    }
  }
}
```

---

## Cursor / Windsurf / VS Code

Add to `.cursor/mcp.json`, `.windsurf/mcp.json`, or equivalent IDE MCP config:

```json
{
  "mcpServers": {
    "debridge": {
      "type": "streamable-http",
      "url": "https://agents.debridge.com/mcp"
    }
  }
}
```

---

## Claude Code (CLI)

Streamable HTTP (preferred):
```bash
claude mcp add --transport http debridge https://agents.debridge.com/mcp
```

Stdio proxy (for environments without Streamable HTTP support):
```bash
claude mcp add debridge npx -- -y @debridge-finance/debridge-mcp@latest
```

---

## Headless / Programmatic (MCP SDK)

### Streaming Transport

```typescript
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

const client = new Client({ name: "my-agent", version: "1.0.0" });
const transport = new StreamableHTTPClientTransport(
  new URL("https://agents.debridge.com/mcp")
);
await client.connect(transport);

const chains = await client.callTool({
  name: "get_supported_chains",
  arguments: {}
});
```

---

## Verification

After configuration, verify the connection:

1. Call `mcp__debridge__get_supported_chains` (no parameters).
2. Expected: JSON array of chain objects with `chainId` and `chainName`.
3. If successful → MCP is ready. Proceed to WALLET_DISCOVERY in SKILL.md.
4. If failed → see troubleshooting below.

---

## Troubleshooting

| Symptom                    | Cause                     | Fix                                                      |
|----------------------------|---------------------------|----------------------------------------------------------|
| Tool `mcp__debridge__*` not found | Server not configured | Add config per sections above                            |
| Connection refused (stdio) | Node.js missing           | Install Node.js 18+                                     |
| Connection timeout         | Network/firewall          | Check HTTPS access to `agents.debridge.com`          |
| Auth error                 | None expected             | deBridge MCP is public, no API key needed                |
