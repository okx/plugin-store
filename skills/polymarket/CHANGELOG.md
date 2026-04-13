# Polymarket Plugin Changelog

### v0.2.7 (2026-04-13) — patch fixes

- **fix [N1]**: `get-series` on 4h series no longer reports wrong `session` and `trading_hours`. Root cause: `session` was computed using the NYSE hours check regardless of series type, and `trading_hours` was hardcoded to `"9:30 AM – 4:00 PM ET, Monday–Friday"` for all series. Fix: for `nyse_hours_only: false` series, `session` is always `"24/7 — market open"` and `trading_hours` is `"24/7"`. Also fixes the `interval` field for 4h series (now shows `"4 hours"` instead of `"240 minutes"`).
- **fix [N2]**: `--token-id` no longer requires `--market-id`. `--market-id` is now optional in `buy` and `sell`; it is required only when `--token-id` is not provided. When `--token-id` is given, the binary skips all market lookup and `--market-id` is unused. Attempting to call `buy`/`sell` without either flag returns a clear error.
- **fix [N3]**: `get-series` slot output now includes `end_unix` (Unix timestamp integer) alongside the existing `end` (ISO-8601 string). SKILL.md agent caching protocol references `current_slot.end_unix` for slot-validity checks — the field now exists.
- **fix [N4]**: `buy --market-id btc-5m` (and 15m variants) no longer blocks trading outside NYSE hours. The NYSE hours gate in `get_current_slot` has been removed; the binary now always attempts to find the current accepting slot and lets the CLOB be the source of truth. If no slot is accepting orders, a clear `"No open market found"` error is returned. Live verification confirms 5m/15m slots accept orders 24/7 on the CLOB despite the NYSE hours label.

### v0.2.7 (2026-04-13) — original release

- **feat**: Series trading for recurring "Up or Down" crypto markets. Supports 5-minute (NYSE hours), 15-minute (NYSE hours), and 4-hour (24/7) slots for BTC, ETH, SOL, and XRP. Use `buy --market-id btc-5m`, `btc-15m`, or `btc-4h` — the plugin auto-resolves to the current accepting slot at trade time.
- **feat**: `get-series` command — shows the current and next slot with prices, token IDs, seconds remaining, liquidity, and a ready-to-run buy hint. `--list` enumerates all 12 supported series (4 assets × 3 intervals).
- **feat**: DST-aware NYSE trading hours check. For 5m and 15m series, reports time until next session when called outside hours. 4h series runs 24/7 and bypasses the hours check.
- **feat**: Slot transition gap handling — at the 5/15-minute boundary, tries the next slot automatically before failing.
- **perf**: Eliminated 3 redundant HTTP calls on the buy/sell hot path: deduplicated CLOB fee fetch (~150ms), eliminated separate tick-size call by reading it from the order book response (~100ms), and avoided double Gamma fetch on series markets (~200ms).
- **perf**: Parallelized order book fetch + wallet address subprocess via `tokio::join!` for live orders (~100-200ms saved). Dry-run still works without a configured wallet.
- **feat**: `buy --token-id <id>` / `sell --token-id <id>` fast path — skips all market resolution when the token ID is known from a prior `get-series` call. Only 2 HTTP calls (book + fee) vs 5-6 normally. Target: ~0.5s binary execution. Token IDs change each slot so re-run `get-series` at each new slot.

### v0.2.6 (2026-04-12)

- **fix (critical) [C1]**: `buy` on `neg_risk: true` markets no longer approves the wrong contract. Root cause: `get_gamma_market_by_slug` omits `negRisk` for many markets, causing the field to default to `false` and `approve_usdc` to target `CTF_EXCHANGE` instead of `NEG_RISK_CTF_EXCHANGE`. Fix: `resolve_market_token` now fetches the CLOB market by `condition_id` after the Gamma lookup to get the authoritative `neg_risk`. Falls back to the Gamma value if the CLOB is unreachable. Same fix applied in `redeem`.
- **fix (major) [M1 buy]**: Approval tx no longer fires when the wallet has insufficient USDC.e balance. After computing the exact order amount, the plugin reads `balance` from the `/balance-allowance` response and bails with a clear error before submitting any on-chain tx.
- **fix (major) [M1 sell]**: GCD alignment and zero-amount guard now run before the CTF approval tx. Previously, `setApprovalForAll` could fire for an order that would immediately fail the divisibility check (e.g. `--shares 0.001`). Sell is fully restructured: public-API work (market lookup, tick size, price, GCD) happens first; auth operations (balance check, approval, signing) happen after.
- **fix [N1]**: `buy --dry-run` now returns full projected order fields: `condition_id`, `token_id`, `side`, `order_type`, `limit_price`, `usdc_amount`, `shares`, `fee_rate_bps`, `post_only`, `expires`. Market resolution and GCD alignment run in dry-run mode; only wallet and signing operations are skipped.
- **fix [N2]**: `sell --dry-run` now runs GCD alignment and shows the adjusted `limit_price`, `shares`, and `usdc_out`. Output includes `limit_price_requested` and `price_adjusted: true/false` so the user can see exactly what the live command would execute.
- **fix [N3]**: `is_ctf_approved_for_all` now returns `Result<bool>` instead of `bool`. Callers log a warning to stderr when the Polygon RPC check fails (previously silent) and proceed to re-approve (setApprovalForAll is idempotent). Approval log messages now include the specific exchange name (e.g. "Neg Risk CTF Exchange" vs "CTF Exchange").
- **fix [N4]**: `sell` logs a `[polymarket] Note: price adjusted from X to Y` warning to stderr when the user's `--price` is rounded to satisfy the market's tick size constraint. Matches the existing adjustment warning in `buy`.
- **fix [N5]**: `get-positions` output now includes a `redeemable_note` field. For `redeemable: true` positions: "resolved — winning outcome, redeem to collect USDC.e" or "resolved — losing outcome, redemption would receive $0" (when `current_value ≈ 0`). Prevents agents from routing users to the `redeem` command for losing positions.
- **fix [S1]**: `redeem` now checks the wallet's positions for the target market before submitting the tx. If all redeemable positions show `current_value ≈ $0`, a clear warning is logged to stderr: "This market resolved against your positions — redeeming will cost gas and receive nothing."
- **fix [N6]**: Added betting-vocabulary trigger phrases to plugin description: `place a bet on`, `buy prediction market`, `bet on`, `trade on prediction markets`, `prediction trading`, `place a prediction market bet`, `i want to bet on`.

### v0.2.5 (2026-04-12)

- **fix**: Stale credentials auto-cleared on 401 — `buy` and `sell` now detect `NOT AUTHORIZED`/`UNAUTHORIZED` responses from the CLOB, delete `~/.config/polymarket/creds.json` automatically, and return a clear error asking the user to re-run. Previously the user had to find and delete the file manually.
- **fix**: `accepting_orders` guard added to `resolve_market_token` (used by `buy` and `sell`). Attempting to trade on a closed or resolved market now exits immediately with a clear error before any wallet calls or approval transactions.
- **fix (SKILL)**: Added targeted agent guidance for six common user deviation scenarios: extracting market ID from Polymarket URLs (#1), short-lived market warning before resting GTC orders (#3), amount vs shares clarification (#5), no "Polymarket deposit" step misconception (#10), cancel only applies to open orders (#11), price field represents probability not dollar value (#12).
- **feat**: `check-access` command — dedicated geo-restriction check. Sends an empty `POST /order` to the CLOB with no auth headers; the CLOB applies geo-checks before auth on this endpoint, returning HTTP 403 + `"Trading restricted in your region"` for blocked IPs and 400/401 for unrestricted ones. Body-matched (not status-code-only) to avoid false positives. Returns `accessible: true/false`. Run once before recommending USDC top-up. Tested live on both restricted and unrestricted IPs.
- **feat**: `redeem --market-id <id>` command — redeems winning outcome tokens after a market resolves by calling `redeemPositions` on the Gnosis CTF contract with `indexSets=[1,2]`. The CTF contract pays out winning tokens and silently no-ops for losing ones, so passing both is safe. `--dry-run` previews the call without submitting. Not supported for `neg_risk: true` markets (use Polymarket web UI).
- **fix (critical)**: `sell` on `neg_risk: true` markets no longer always fails with "allowance not enough". `approve_ctf` now approves both `NEG_RISK_CTF_EXCHANGE` and `NEG_RISK_ADAPTER` for neg_risk markets, mirroring the `approve_usdc` pattern already used by `buy`.
- **fix**: `sell` no longer fires a redundant `setApprovalForAll` transaction when CTF tokens are already approved. Approval state is now read via direct on-chain `isApprovedForAll` eth_call to the Polygon RPC before deciding whether to approve.
- **fix**: `buy` now pre-validates resting limit orders (price below best ask) against `min_order_size` (typically 5 shares). Clear error with share count and ≈USDC cost is returned before any on-chain approval. `--round-up` automatically snaps up to the minimum. Market (FOK) orders are exempt.
- **fix**: `--keyword` filter in `list-markets` now works. The Gamma API `?q=` parameter was confirmed to be a no-op — replaced with client-side substring filtering on `question` and `slug` fields.
- **fix**: `sell` zero-amount divisibility guard now actually fires (was documented in SKILL.md but not implemented). Prevents approval tx from being sent when shares are too small to produce a valid order.
- **fix**: `sell` now warns on stderr when GCD alignment reduces the requested share amount (e.g. 9.0 shares silently sold as 8.75). The remainder and the reason are logged.
- **fix**: `sell --dry-run` output now includes `side`, `order_type`, `limit_price`, `post_only`, and `expires` fields (previously only `market_id`, `outcome`, `shares`, and `estimated_price: null`).
- **fix**: `buy` now warns on stderr when USDC amount is rounded down by GCD alignment (e.g. `$2.00 → $1.98`). Consistent with the existing `--round-up` stderr note.
- **fix**: `get-market` now returns `fee_bps` (from `maker_base_fee` on the CLOB API) instead of always-null `fee`. Per-token `last_trade` removed — the CLOB `/book` endpoint returns a market-level value regardless of token_id, making it unreliable per-token.
- **fix**: `list-markets` no longer emits `category` field — the Gamma API `category` field is consistently null across all markets.
- **fix**: `--expires` help text corrected from "60 seconds" to "90 seconds" to match actual enforcement.
- **fix (SKILL)**: Telemetry version in preflight script corrected from `0.2.1` to `0.2.5`.
- **fix (SKILL)**: `buy --dry-run` flag added to buy flags table (was functional but undocumented).
- **fix (SKILL)**: Minimum order size guidance updated to reflect that `min_order_size` IS enforced by the CLOB for resting orders (contrary to the v0.2.3 note).

### v0.2.4 (2026-04-12)

- **feat**: `buy --round-up` flag — when the requested amount is too small to satisfy Polymarket's divisibility constraints at the given price, snaps up to the nearest valid minimum instead of erroring. Logs the rounded amount to stderr; output JSON includes `rounded_up: true` and both `usdc_requested` and `usdc_amount` fields for transparency.
- **fix (SKILL)**: Agent flow for small-amount errors now collapses two independent minimums (divisibility guard and CLOB FOK floor) into a single user prompt. For market orders, agent presents both constraints together and offers the choice between a $1 market order or a resting limit order below the spread (which avoids the $1 CLOB floor). Agents must never autonomously choose a higher amount.
- **feat**: `buy --post-only` and `sell --post-only` — maker-only flag; rejects order if it would immediately cross the spread. Incompatible with FOK. Qualifies for Polymarket's maker rebates program (20–50% of fees returned daily).
- **feat**: `buy --expires <unix_ts>` and `sell --expires <unix_ts>` — GTD (Good Till Date) orders that auto-cancel at the given timestamp. Minimum 90 seconds in the future (CLOB enforces "now + 1 min 30 s" security threshold); automatically sets `order_type: GTD`. Both `expires` and `post_only` fields appear in command output.
- **fix**: `buy` on `neg_risk: true` markets (multi-outcome: NBA Finals, World Cup winner, award markets, etc.) now works correctly. The CLOB checks USDC allowance on both `NEG_RISK_CTF_EXCHANGE` and `NEG_RISK_ADAPTER` for these markets — the plugin previously only approved `NEG_RISK_CTF_EXCHANGE`, causing "not enough allowance" rejections. Both contracts are now approved.
- **fix**: `get-market` `best_bid` and `best_ask` fields now show the correct best price for each outcome token. The CLOB API returns bids in ascending order and asks in descending order — the previous `.first()` lookup was returning the worst price in the book rather than the best.
- **fix**: GTD `--expires` minimum validation tightened from 60 s to 90 s to match the CLOB's actual "now + 1 minute + 30 seconds" security threshold, preventing runtime rejections.

### v0.2.3 (2026-04-12)

- **fix**: GCD amount arithmetic now uses `tick_scale = round(1/tick_size)` instead of hardcoded `100`. Fixes "breaks minimum tick size rule" rejections on markets with tick_size=0.001 (e.g. very low-probability political markets). Affected both buy and sell order construction.
- **fix**: `sell` command now uses the same GCD-based integer arithmetic as `buy` — previously used independent `round_size_down` + `round_amount_down` which could produce a maker/taker ratio that didn't equal the price exactly, causing API rejection.
- **fix**: Removed `min_order_size` pre-flight check from `buy` — the field returned by the CLOB API is unreliable (returns `"5"` uniformly regardless of actual enforcement) and was causing false rejections. The CLOB now speaks for itself via `INVALID_ORDER_MIN_SIZE` errors.
- **fix**: Added zero-amount divisibility guard to `buy` (computed before approval tx) — catches orders that are mathematically too small to satisfy CLOB divisibility constraints at the given price, with a clear error and computed minimum viable amount.
- **fix (SKILL)**: Clarified that `min_order_size` API field must never be used to auto-escalate order amounts; agents must surface size errors to the user and ask for explicit confirmation before retrying.

### v0.2.2 (2026-04-11)

- **feat**: Minimum order size guard — fetches `min_order_size` from order book before placing; prints actionable error and exits with code 1 if amount is below market minimum.
- **fix**: Order book iteration corrected — CLOB API returns bids ascending (best=last) and asks descending (best=last); was previously iterating from worst price causing market orders to be priced at 0.01/0.99.
- **fix**: GCD-based integer arithmetic for buy order amounts — guarantees `maker_raw / taker_raw == price` exactly, eliminating "invalid amounts" rejections caused by independent floating-point rounding.
- **feat (SKILL)**: Pre-sell liquidity check — agent must inspect `get-market` output for null best_bid, collapsed price (< 50% of last trade), wide spread (> 0.15), or thin market (< $1,000 liquidity) and warn user before executing sell.
