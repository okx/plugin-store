---
title: Skill Freshness Check
impact: LOW
impactDescription: "Optional — refresh skills to get latest chain support and bug fixes"
tags: freshness, update, github, mcp-resources, llms-txt
---

# Skill Freshness Check

Skills evolve with new chain support, MCP tool changes, and bug fixes. Try methods in order; stop at the first success.

## Method 1: GitHub Raw Fetch (CLI, Headless with HTTP tools)

Fetch the skill catalog and individual files directly from GitHub raw content. Detect the available HTTP tool:

```bash
# Prefer: gh CLI (authenticated, no rate limit)
gh api repos/debridge-finance/debridge-skills/contents/skills/index.json \
  --jq '.content' | base64 -d

# Or: curl
curl -sL https://raw.githubusercontent.com/debridge-finance/debridge-skills/main/skills/index.json

# Or: wget
wget -qO- https://raw.githubusercontent.com/debridge-finance/debridge-skills/main/skills/index.json
```

Parse `index.json` to discover skills, then fetch individual SKILL.md files:

```bash
curl -sL https://raw.githubusercontent.com/debridge-finance/debridge-skills/main/skills/bridge/SKILL.md
```

Agents with built-in web fetch (Claude Code `WebFetch`, OpenHands `browse`) can use the same raw URLs directly.

## Method 2: MCP Resources (MCP Desktop, any MCP-connected environment)

If deBridge MCP is connected (confirmed during ACCESS_SETUP), the server at `https://agents.debridge.com/mcp` exposes skills as MCP resources. List and read them:

```
resources/list  → returns available skill resources
resources/read  → fetch individual skill content by URI
```

This requires no filesystem access — works in sandboxed MCP Desktop environments.

## Method 3: llms.txt (any environment with HTTP fetch)

Fetch the lightweight skill catalog:

```
https://agents.debridge.com/llms.txt
```

This returns skill names, descriptions, and links to individual SKILL.md files. Parse for the skills relevant to the current task. Works in any environment that can make HTTPS requests.

## No Update Available

If none of the above work (no HTTP tools, no MCP), proceed with bundled/cached skills. They are functional but may be outdated. Inform the user:

> "Using bundled deBridge skills. For the latest instructions, visit https://github.com/debridge-finance/debridge-skills"
