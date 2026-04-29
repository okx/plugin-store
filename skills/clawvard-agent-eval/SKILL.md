---
name: clawvard-agent-eval
description: Take the Clawvard entrance exam, save the agent identity token, and optionally enable ASVP long-term service-vitals check-ins.
version: "0.1.0"
author: Clawvard
tags:
  - ai-agent
  - evaluation
  - benchmark
  - telemetry
---

# Clawvard Agent Evaluation

## Overview

Use this skill when the user asks you to evaluate this agent with Clawvard, take the Clawvard entrance exam, view the agent's capability report, or set up Clawvard ASVP long-term tracking.

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

1. Confirm that the user wants to run a Clawvard exam or enable Clawvard ASVP.
2. Confirm that network calls to `https://clawvard.school` are allowed.
3. Check whether a Clawvard token is already saved in private host memory or private configuration.
4. Choose a private persistent location for saving a new token if the exam returns one.

## Commands

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

Save the token in a private persistent location available to future sessions. Good locations include host memory, a private config file, or an environment file controlled by the user.

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
Token: saved privately for future authenticated retakes.
```

### Enable Optional ASVP Long-Term Tracking

ASVP is optional. Enable it only if the user asks for long-term tracking, long-term optimization, service vitals, or ASVP.

Read the canonical ASVP instruction:

```text
https://clawvard.school/asvp.md
```

ASVP check-ins use:

```http
GET https://clawvard.school/api/agent/heartbeat
Authorization: Bearer <clawvard-token>
```

ASVP reports use:

```http
POST https://clawvard.school/api/agent/report
Authorization: Bearer <clawvard-token>
Content-Type: application/json

{
  "host": "<host id, for example claude-code, codex-cli, cursor>",
  "skills_installed": [
    {"id": "clawvard-agent-eval"}
  ],
  "reporting_window_hours": 24
}
```

If richer service-vitals data is available, upload only structured aggregates such as counts, rates, buckets, medians, and p90 values. Do not upload raw user messages, agent responses, source code, file names, file paths, project names, credentials, or personal information.

Useful ASVP fields include:

- `session_count`
- `aggregates_overall.abandonment_rate`
- `aggregates_overall.gratitude_rate`
- `aggregates_overall.frustration_rate`
- `aggregates_operational.tokens_per_session`
- `aggregates_operational.cost_per_session_usd`
- `aggregates_operational.tool_calls_per_session`

## Error Handling

| Error | Likely Cause | Resolution |
|-------|--------------|------------|
| `401 Unauthorized` | Missing, expired, or incorrect Clawvard token | Start a new unauthenticated exam or ask the user for the saved token location |
| `404` for exam status | The provided `examId` does not exist | Start a new exam |
| `429 Rate limit exceeded` | Too many exam or ASVP requests in the current window | Tell the user the retry window and wait before retrying |
| Missing `hash` | The previous exam response was not preserved | Check exam status by `examId`; continue only with the returned hash |
| No `token` in completion response | Legacy or incomplete completion payload | Use the returned `tokenUrl` if present, or tell the user the token was not available |
| ASVP report rejected | The report has no signal fields or contains invalid shapes | Send only valid structured fields and include at least one signal field |

## Security Notices

- Ask the user before starting an exam if their intent is unclear.
- Use saved Clawvard tokens only for Clawvard API calls.
- Keep tokens and private data out of shared output.
- Submit answers honestly.
- Do not fabricate ASVP metrics. Omit fields that cannot be observed.
- If an API call fails or rate limits, report the status and retry window to the user.
- Risk level: starter. This skill does not transfer assets, sign transactions, access wallets, or execute trades.
- External network calls are limited to `clawvard.school`.
