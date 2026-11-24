# Aave V3 Plugin Changelog

### v0.3.0 (2026-09-03) — failures are readable: JSON on stdout, full cause chain

Two long-standing defects made every failure opaque. Neither was introduced by
the 0.2.9 receipt/allowance work — `main.rs`, `onchainos.rs` and `positions.rs`
were untouched by it.

- **fix**: failures were written to **stderr** with exit `1` and an **empty stdout**.
  SKILL.md tells the caller to parse stdout, so on any failure an agent got an
  empty string and could report nothing beyond "the command failed". Failures now
  print the same JSON contract the other plugins use — `{"ok":false,...}` on stdout,
  exit `0`. **Breaking for shell callers that branch on `$?`**: branch on `ok` instead.
- **fix**: the error payload used `err.to_string()`, which renders only the
  outermost `anyhow` context and drops the cause chain — where the actual reason
  lives. `error` is now `{:#}` (whole chain on one line) and a new `causes` array
  lists each layer, outermost first. This was the only `{}` error rendering left in
  the crate; `rpc.rs` already used `{:#}` in all 7 of its error paths.
- **fix**: `resolve_token` raised `No token match for '<sym>' on chain <id>` and
  discarded the payload it had just read. `run_cmd` attaches onchainos's stderr and
  stdout only when the process exits **non-zero**, but a search that answers
  `{"ok":true,"data":[],"notifications":[...]}` exits `0` — so the entries count and
  any quota or rate-limit notice were invisible even with the cause chain restored.
  Both no-match errors now carry them.
- **fix**: `positions.rs` and `reserves.rs` logged their non-fatal onchainos
  failures with `{}`, dropping the chain in the warning too.
- **fix**: `claim_rewards` pattern-matches an onchainos message to detect "no
  rewards". It matched `e.to_string()`, which works only because `defi_collect`
  returns `run_cmd`'s error unwrapped — a single `with_context` on that path would
  have silently broken the check. It now matches the flattened chain.
- **docs**: SKILL.md documents the error output contract (stdout, `ok`, `causes`,
  exit `0`) and how to read a quota notification. The notice appears on **successful**
  lookups too, so it is not by itself why a symbol failed to resolve.
- **tests**: +5. `error_json` (2) covers `ok:false`, chain flattening and the
  single-layer case; `describe_response` (3) covers an empty payload with and
  without notifications, and a payload carrying neither field.
- **chore**: version skips 0.2.9 — that tag is already published — and the contract
  change earns the minor bump.
