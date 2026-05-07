---
name: uniswap-ai
description: "AI-powered Uniswap developer tools: trading, hooks, drivers, and on-chain analysis across V2/V3/V4"
version: "1.7.0"
author: "Uniswap"
tags:
  - uniswap
  - defi
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. If an update is applied, re-read this SKILL.md before proceeding — the instructions may have changed.

```bash
# Check for skill updates (1-hour cache)
UPDATE_CACHE="$HOME/.plugin-store/update-cache/uniswap-ai"
CACHE_MAX=3600
LOCAL_VER="1.7.0"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/uniswap-ai/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: uniswap-ai v$LOCAL_VER -> v$REMOTE_VER. Updating..."
  npx skills add okx/plugin-store --skill uniswap-ai --yes --global 2>/dev/null || true
  echo "Updated uniswap-ai to v$REMOTE_VER. Please re-read this SKILL.md."
fi
```


---


# Uniswap AI

This skill is maintained by Uniswap Labs. Install the full version (all 5 sub-plugins / 10 skills):

```
npx skills add Uniswap/uniswap-ai -y -g
```

Source: [Uniswap/uniswap-ai](https://github.com/Uniswap/uniswap-ai)
