---
name: trustx-plugin
description: "ERC-8004 reputation scoring and trust-policy gating for X Layer agents"
version: "1.0.0"
author: "Alanas"
tags:
  - xlayer
  - erc-8004
  - reputation
  - trust
  - risk
---

# TrustX Plugin

Use this skill when the user asks to assess trust/reputation of an X Layer agent before collaboration, delegation, approvals, wallet-signing, or autonomous execution.

## What it does

TrustX runs an ERC-8004 reputation check and returns a decision packet:
- `score` (0-100)
- `confidence` (0-1)
- `rating` (`high|medium|low|unknown`)
- policy verdict (`allow|sandbox|block`) via preset rules (`strict|balanced|growth`)

## Preconditions

1. Node.js >= 20.10.0
2. Repository available: `https://github.com/Ananas310/best-x-layer-skill`
3. Optional for OnchainOS primary path:
   - `OKX_ONCHAINOS_API_KEY`
   - `OKX_ONCHAINOS_SECRET_KEY`
   - `OKX_ONCHAINOS_PASSPHRASE`

If credentials are missing, TrustX can still run in RPC mode with lower signal coverage.

## Install / Run

Primary (plugin install path):

```bash
npx skills add okx/plugin-store --skill trustx-plugin
```

Then run the installed command:

```bash
rep8004 --agent 1 --pretty
```

Source fallback (for local development/debug):

```bash
git clone https://github.com/Ananas310/best-x-layer-skill.git
cd best-x-layer-skill
npm install
npm test
node src/cli.js --agent 1 --pretty
```

Strict live check (source repo flow):

```bash
npm run smoke:live:strict
```

## Integration Pattern

```js
import { getReputation, evaluateTrustPolicy } from '8004-reputation-skill';

const report = await getReputation('1', { source: 'auto' });
const verdict = evaluateTrustPolicy(report, {
  preset: 'strict',
  risk: 'high',
});

// verdict.decision.action => allow | sandbox | block
```

## When to use which preset

- `strict`: wallet signing, fund control, irreversible actions
- `balanced`: normal production delegation
- `growth`: onboarding/new-agent exploration with safeguards

## Error Handling

| Error / Signal | Meaning | Action |
|---|---|---|
| `ok: false` | Request failed/invalid input | Stop and ask for valid `agentId/address/handle` |
| `rating: unknown` | Insufficient trust evidence | Use `sandbox` path and reduce permissions |
| `meta.indexerError` set | OnchainOS path unavailable/missing credentials | Continue in RPC mode, lower confidence expectations |
| low `confidence` | Sparse/disagreeing signals | Require human review for high-risk operations |

## Security Notes

- Do not grant signing authority on score alone; gate on score + confidence + policy.
- For high-risk actions, require `strict` preset and `high` rating.
- Never expose OnchainOS credentials in logs, prompts, or commits.
- `.env` must remain untracked in git.
