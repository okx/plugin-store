---
name: rootdata-crypto-plugin
description: "RootData is a leading Web3 data platform covering crypto projects, investors, funding data, and personnel movements. This skill is the OKX-dedicated integration, isolated from the standard RootData skill with its own API key namespace and endpoints."
version: "1.0.0"
author: "rootdata"
tags:
  - crypto
  - web3
  - blockchain
  - investors
  - funding
  - projects
  - defi
  - personnel
  - okx
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. The check is a read-only fetch from `raw.githubusercontent.com` — it does **not** install anything. If it reports `Update available`, you MUST stop, surface the message to the user verbatim, and obtain explicit confirmation before running the suggested `npx skills add …` command. Do not run the update silently. After a confirmed update completes, re-read this SKILL.md before proceeding.

```bash
# Check for skill updates (1-hour cache).
# Network scope: this block calls raw.githubusercontent.com for the version probe only.
# It does NOT install anything; install requires user-confirmed `npx skills add` below.
UPDATE_CACHE="$HOME/.plugin-store/update-cache/rootdata-crypto-plugin"
CACHE_MAX=3600
LOCAL_VER="1.0.0"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/rootdata-crypto-plugin/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: rootdata-crypto-plugin v$LOCAL_VER -> v$REMOTE_VER."
  echo "ACTION REQUIRED: ask the user to confirm before running:"
  echo "  npx skills add okx/plugin-store --skill rootdata-crypto-plugin --global"
  echo "(This contacts the npm registry and github.com/okx/plugin-store and overwrites this skill. Do NOT auto-run.)"
fi
```

---


# RootData Crypto Intelligence (OKX Edition)

## Overview

RootData is a leading Web3 data platform covering crypto projects, investors, funding data, and personnel movements. This skill is the OKX-dedicated integration, isolated from the standard RootData skill with its own API key namespace and endpoints.

## Pre-flight Checks

Before using this skill, ensure:

1. Check if the environment variable `OKX_ROOTDATA_SKILL_KEY` exists.
2. If it does **NOT** exist, call the `/init` command below to generate a key and save it.
3. All subsequent requests must include `Authorization: Bearer {OKX_ROOTDATA_SKILL_KEY}` and `Content-Type: application/json`.

### Initialize API Key

`POST https://api.rootdata.com/open/okx/skill/init`

**When to use**: First-time setup, or when `OKX_ROOTDATA_SKILL_KEY` is missing or invalid.
**Output**: Returns `api_key` (prefixed with `okxsk_`). Save it as `OKX_ROOTDATA_SKILL_KEY`.
**Example**:

```
POST https://api.rootdata.com/open/okx/skill/init
Content-Type: application/json
Body: {}
```

Response:
```json
{
  "result": 200,
  "data": {
    "api_key": "okxsk_xxxxxxxxxxxxxxxx"
  }
}
```

After saving, confirm to user: "RootData (OKX) access is ready. You can now query crypto data."

---

## Commands

All requests must include:

```
Authorization: Bearer {OKX_ROOTDATA_SKILL_KEY}
Content-Type: application/json
```

---

### Search

`POST https://api.rootdata.com/open/okx/skill/ser_inv`

**When to use**: User wants to search for a crypto project, investor institution, or person by name keyword. Use this as the first step when an entity ID is not yet known.
**Output**: A list of matching entities with their IDs, types, names, and descriptions.
**Example**:

```json
{
  "query": "Uniswap",
  "precise_x_search": false
}
```

Key response fields:
- `id` - entity ID (use for follow-up queries)
- `type` - `1`=Project, `2`=VC/Institution, `3`=Person
- `name` - display name
- `one_liner` - brief description
- `introduce` - full description
- `rootdataurl` - link to full detail page on RootData

---

### Get All IDs by Type

`POST https://api.rootdata.com/open/okx/skill/id_map`

**When to use**: User asks for a complete list of all project IDs, institution IDs, or person IDs for bulk lookup or enumeration.
**Output**: Array of `{ id, name }` entries for all entities of the specified type.
**Example**:

```json
{
  "type": 1
}
```

`type` values: `1`=Project, `2`=VC/Institution, `3`=Person

Key response fields:
- `id` - entity ID
- `name` - entity name

---

### Project Detail

`POST https://api.rootdata.com/open/okx/skill/get_item`

**When to use**: User asks for detailed information about a specific crypto project, including its description, funding total, investors, tags, and social links.
**Output**: Full project profile including funding summary, contracts, social media, and investor list.
**Example** (by project ID):

```json
{
  "project_id": 12345,
  "include_investors": true
}
```

Or by contract address:

```json
{
  "contract_address": "0xabc123...",
  "include_investors": true
}
```

Key response fields:
- `project_id`, `project_name`, `logo`, `token_symbol`
- `one_liner`, `description`
- `active` - whether the project is still active
- `establishment_date` - founding date
- `tags` - category tags (e.g. DeFi, Layer2, NFT)
- `contracts` - on-chain contract addresses
- `total_funding` - total funding raised (USD)
- `social_media` - website, X(Twitter), GitHub, CoinMarketCap, CoinGecko, etc.
- `investors` - list of investors (only when `include_investors=true`)
- `similar_project` - similar projects in same category
- `rootdataurl` - RootData detail page link

---

### Funding Rounds

`POST https://api.rootdata.com/open/okx/skill/get_fac`

**When to use**: User asks about funding history, investment rounds, "how much did XX raise", "who invested in XX", or requests a list of recent fundraises.

**Data range**: Covers funding rounds from the **past 365 days only**. Requests for older data will automatically return only the most recent 365 days.

**Investor limit**: Each funding round returns a maximum of **3 investors** (lead investors prioritized).

**Output**: Paginated list of funding rounds with project info and investor details.
**Example**:

```json
{
  "page": 1,
  "page_size": 20,
  "project_id": 12345,
  "start_time": "2025-01-01",
  "end_time": "2026-03-30",
  "min_amount": 1000000,
  "max_amount": 50000000
}
```

All fields except `page` and `page_size` are optional. Omit any field not specified by the user.

Key response fields:
- `total` - total matching records
- `items` - list of funding rounds, each containing:
  - `name` - project name
  - `logo` - project logo
  - `rounds` - round type (Seed / Series A / Series B / etc.)
  - `published_time` - announcement date
  - `amount` - funding amount (USD)
  - `project_id` - project ID
  - `data_status` - data verification status
  - `source_url` - original news source link
  - `X` - project's X (Twitter) URL
  - `one_liner` - project brief description
  - `invests` - up to 3 investors, each with:
    - `name`, `logo`
    - `lead_investor` - `1`=lead investor, `0`=participating
    - `type` - `1`=Project, `2`=VC/Institution, `3`=Person
    - `invest_id` - investor ID
    - `rootdataurl` - investor detail page

---

### Trending Projects

`POST https://api.rootdata.com/open/okx/skill/hot_index`

**When to use**: User asks "what's hot in crypto today", "trending projects", "top projects this week", or similar trending/popularity queries.
**Output**: Ranked list of trending projects for the requested time window.
**Example**:

```json
{
  "days": 1
}
```

`days` values: `1`=today's trending, `7`=this week's trending

Key response fields:
- `rank` - ranking position
- `project_id`, `project_name`, `logo`, `token_symbol`
- `one_liner` - brief description
- `tags` - category tags
- `X` - Twitter/X profile URL
- `rootdataurl` - RootData detail page link

---

### Personnel Job Changes

`POST https://api.rootdata.com/open/okx/skill/job_changes`

**When to use**: User asks about recent job changes in crypto, "who joined which project", "who left which company", personnel movements, or executive changes in the Web3 industry.

**Data limit**: Returns a maximum of **20 recent entries** for each category.

**Output**: Two arrays - recent hires and recent departures - each with up to 20 entries.
**Example**:

```json
{
  "recent_joinees": true,
  "recent_resignations": true
}
```

Parameters:
- `recent_joinees` - `true` to get recent hires/joiners
- `recent_resignations` - `true` to get recent departures
- Both can be `true` simultaneously
- If both are `false` or omitted, returns empty data

Key response fields:
- `recent_joinees` - array of recent hires (max 20), each containing:
  - `people_id` - person ID
  - `people_name` - person's name
  - `head_img` - profile photo
  - `company_type` - `1`=Project, `2`=VC/Institution, `3`=Person
  - `company_id` - company/project ID
  - `company` - company/project name
  - `position` - job title
- `recent_resignations` - array of recent departures (max 20), same structure

---

## Multi-Language Support

Add header `language: cn` for Chinese responses, or `language: en` for English (default).

All name fields, descriptions, and position titles will be returned in the requested language.

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `401 Unauthorized` | Missing or invalid `OKX_ROOTDATA_SKILL_KEY` | Re-run `/init` to generate a new key and update the env variable |
| `429 Too Many Requests` | Rate limit exceeded (200 req/min) | Wait for the number of seconds in the `Retry-After` response header, then retry |
| `400 Bad Request` | Invalid or missing required parameters | Check request body; verify `type`, `days`, or `page` values are correct |
| `404 Not Found` | Entity ID does not exist | Confirm the ID via `/ser_inv` search before calling detail endpoints |
| `500 Internal Server Error` | Upstream server error | Retry after a short delay; if persistent, report to RootData support |
| `result` field != `200` | Application-level error | Read the `message` field in the response for details |

If you receive HTTP `429`, check the `Retry-After` header and wait before retrying. Monitor `X-RateLimit-Remaining` to track remaining quota.

---

## Security Notices

- This skill is **read-only** and does not perform any write, trade, or transaction operations.
- The `okxsk_` prefixed API key is an anonymous, low-privilege key used solely to call RootData's public data endpoints. It has no access to any personal account or wallet data.
- The key is stored as a local environment variable (`OKX_ROOTDATA_SKILL_KEY`) and is never sent to any service other than `api.rootdata.com`.
- You can revoke the key at any time by contacting RootData support.
- Rate limit: **200 requests per minute** per API key.

---

## Version History

### v1.0.0 (2026-04-27)
- Initial OKX edition release
- Isolated endpoint namespace: `/open/okx/skill/`
- Isolated API key namespace: `okxsk_` prefix, env var `OKX_ROOTDATA_SKILL_KEY`
- 6 skills: Search, ID Map, Project Detail, Funding Rounds, Trending Projects, Personnel Job Changes
- Funding data limited to past 365 days; max 3 investors per round
- Job changes: max 20 entries per category
