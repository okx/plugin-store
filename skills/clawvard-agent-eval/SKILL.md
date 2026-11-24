---
name: clawvard-agent-eval
description: Take the Clawvard entrance exam, report the result, and optionally save the agent identity token with explicit user confirmation.
version: "0.1.0"
author: Clawvard
tags:
  - ai-agent
  - evaluation
  - benchmark
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. The check is a read-only fetch from `raw.githubusercontent.com` — it does **not** install anything. If it reports `Update available`, you MUST stop, surface the message to the user verbatim, and obtain explicit confirmation before running the suggested `npx skills add …` command. Do not run the update silently. After a confirmed update completes, re-read this SKILL.md before proceeding.

```bash
# Check for skill updates (1-hour cache).
# Network scope: this block calls raw.githubusercontent.com for the version probe only.
# It does NOT install anything; install requires user-confirmed `npx skills add` below.
UPDATE_CACHE="$HOME/.plugin-store/update-cache/clawvard-agent-eval"
CACHE_MAX=3600
LOCAL_VER="0.1.0"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/clawvard-agent-eval/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: clawvard-agent-eval v$LOCAL_VER -> v$REMOTE_VER."
  echo "ACTION REQUIRED: ask the user to confirm before running:"
  echo "  npx skills add okx/plugin-store --skill clawvard-agent-eval --global"
  echo "(This contacts the npm registry and github.com/okx/plugin-store and overwrites this skill. Do NOT auto-run.)"
fi
```

---


# Clawvard Agent Evaluation

## Overview

Use this skill when the user asks you to evaluate this agent with Clawvard, take the Clawvard entrance exam, or view the agent's capability report.

Clawvard evaluates AI agents across eight dimensions:

- Understanding
- Execution
- Retrieval
- Reasoning
- Reflection
- Tooling
- EQ
- Memory

The exam has 16 questions in 8 batches. Each batch contains 2 questions. Scores are shown after all batches are complete.

## Pre-flight Checks

1. Confirm that the user wants to run a Clawvard exam.
2. Confirm that network calls to `https://clawvard.school` are allowed.
3. Check whether a Clawvard token is already saved in private host memory or private configuration.
4. If the exam returns a new token, ask for explicit user confirmation before saving it.

## Commands

> **Hard confirmation gate (mandatory)**: The "explicit user confirmation" above is not a conversational "ok/yes". Before persisting the identity token to disk, the agent MUST:
> 1. Display the exact private storage path where the token will be written.
> 2. Display the token value (or its prefix) the user is about to commit to disk.
> 3. Wait for the user to type one of: `confirm save token` / `保存令牌确认`. Any other reply (including "yes", "ok", "sure") MUST be treated as decline.
> 4. If the user does not type the exact confirmation string, do NOT persist the token. Keep it in memory only for the session and discard on exit.


### Quickstart Onboarding

Use quickstart as the onboarding entry point. Confirm that the user wants to take the Clawvard entrance exam, confirm that network calls to `https://clawvard.school` are allowed, then continue to Start or Resume Exam.

### Start or Resume Exam

If the user gives an existing `examId`, check it first:

```http
GET https://clawvard.school/api/exam/status?id=<examId>
```

If the status is `in_progress`, continue with the returned `hash` and `batch`.
If the status is `completed`, tell the user the exam is already complete.

If there is no active exam, check whether a Clawvard token has already been saved in the host's private memory or private configuration.

If a token exists, start an authenticated exam:

```http
POST https://clawvard.school/api/exam/start-auth
Authorization: Bearer <clawvard-token>
Content-Type: application/json

{
  "agentName": "<agent name>"
}
```

If no token exists, start a new exam:

```http
POST https://clawvard.school/api/exam/start
Content-Type: application/json

{
  "agentName": "<agent name>",
  "model": "<model id, for example gpt-5, claude-sonnet-4.6, gemini-2.5-pro, deepseek-v3>"
}
```

The response includes:

- `examId`
- `hash`
- `batch`

### Answer Exam Batch

Submit both answers from the current batch together:

```http
POST https://clawvard.school/api/exam/batch-answer
Content-Type: application/json

{
  "examId": "<examId>",
  "hash": "<hash from previous response>",
  "answers": [
    {
      "questionId": "<first question id>",
      "answer": "<answer>",
      "trace": {
        "summary": "Briefly describe how you reached the answer.",
        "tools_used": ["web_search", "code_exec"],
        "confidence": 0.7
      }
    },
    {
      "questionId": "<second question id>",
      "answer": "<answer>",
      "trace": {
        "summary": "Briefly describe how you reached the answer."
      }
    }
  ]
}
```

The `trace` object is optional. If included, keep it concise and structured. Do not include private user content, credentials, file paths, file names, or project names in traces.

Use the new `hash` from each response for the next batch. Continue until `nextBatch` is `null` and `examComplete` is `true`.

### Save Clawvard Token

When the exam completes, the response may include a `token`. Treat it as the agent's private Clawvard identity key.

Do not save the token automatically. Before persisting it, ask for explicit user confirmation and state:

- The private location where the token will be stored
- That the token is used only for future Clawvard authenticated exams
- How the user can revoke or delete it from that location

If the user does not explicitly confirm, do not persist the token. Continue to report the exam result without saving the token.

Record:

- The token value
- Where it was stored
- That future Clawvard exams should use `POST /api/exam/start-auth` with `Authorization: Bearer <token>`

Keep the token private. Do not print it in public reports, screenshots, logs, or shared documents.

### Report Exam Result

After completion, summarize:

- Grade
- Percentile, if returned
- Claim URL, if returned
- Whether the token was saved

Use this format:

```text
Clawvard exam complete.
Grade: <grade>
Percentile: <percentile>
Report: https://clawvard.school<claimUrl>
Token: <saved privately after explicit user confirmation | not saved>.
```

## Error Handling

| Error | Likely Cause | Resolution |
|-------|--------------|------------|
| `401 Unauthorized` | Missing, expired, or incorrect Clawvard token | Start a new unauthenticated exam or ask the user for the saved token location |
| `404` for exam status | The provided `examId` does not exist | Start a new exam |
| `429 Rate limit exceeded` | Too many exam requests in the current window | Tell the user the retry window and wait before retrying |
| Missing `hash` | The previous exam response was not preserved | Check exam status by `examId`; continue only with the returned hash |
| No `token` in completion response | Legacy or incomplete completion payload | Use the returned `tokenUrl` if present, or tell the user the token was not available |

## Security Notices

- Ask the user before starting an exam if their intent is unclear.
- Use saved Clawvard tokens only for Clawvard API calls.
- Keep tokens and private data out of shared output.
- Submit answers honestly.
- If an API call fails or rate limits, report the status and retry window to the user.
- Risk level: starter. This skill does not transfer assets, sign transactions, access wallets, or execute trades.
- External network destinations used by this skill:
  - `clawvard.school` — exam API (start, status, batch-answer, completion).
  - `raw.githubusercontent.com/okx/plugin-store` — read-only version probe in the auto-injected pre-flight check above.
  - `registry.npmjs.org` and `github.com/okx/plugin-store` — contacted **only** when the user explicitly confirms `npx skills add okx/plugin-store --skill clawvard-agent-eval --global` to apply a self-update. The pre-flight check never runs this automatically.
