---
title: MCP Connection Methods
impact: LOW
impactDescription: "Setup guide for connecting to deBridge MCP across different environments"
tags: debridge-mcp, proxy, cli, mcp-client, stdio, streamable-http
---

# Connecting to deBridge MCP

## Recommended: Direct Streamable HTTP Connection

If your environment supports Streamable HTTP MCP transport, connect directly to the hosted endpoint — no installation, no local process:

```
https://agents.debridge.com/mcp
```

### Claude Code (CLI & IDE plugins)

```bash
claude mcp add --transport http debridge https://agents.debridge.com/mcp
```

### Generic Streamable HTTP configuration

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

This works in Claude Desktop, Cursor, Windsurf, VS Code (GitHub Copilot), Cline, Continue, Zed, and any agent that supports Streamable HTTP transport.

---

## Fallback: Stdio Proxy via @debridge-finance/debridge-mcp

Some agent frameworks only support stdio transport and cannot connect to a remote HTTP endpoint directly. Use `@debridge-finance/debridge-mcp` — a thin stdio proxy that forwards all requests to the hosted deBridge MCP:

### Claude Code (CLI & IDE plugins)

```bash
claude mcp add debridge npx -- -y @debridge-finance/debridge-mcp@latest
```

### Generic stdio configuration

```json
{
  "mcpServers": {
    "debridge": {
      "command": "npx",
      "args": ["-y", "@debridge-finance/debridge-mcp@latest"]
    }
  }
}
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `REMOTE_MCP_URL` | `https://agents.debridge.com/mcp` | Remote MCP endpoint to proxy to |
| `MCP_TRANSPORT` | `stdio` | Transport mode: `stdio` or `http` |
| `PORT` | `3000` | HTTP listen port (HTTP mode only) |

Key details:
- The proxy dynamically discovers tools and resources from the upstream endpoint — no local tool definitions needed.
- In stdio mode (default), it opens a long-lived connection to the upstream MCP and mirrors capabilities locally.
- In HTTP mode (`MCP_TRANSPORT=http`), it runs an Express reverse proxy on `localhost:3000/mcp`.
- Works anywhere `npx` is available — no global install required.

---

After setup via either method, verify the connection by calling `mcp__debridge__get_supported_chains` (no parameters). If it returns chain data, MCP is ready.
