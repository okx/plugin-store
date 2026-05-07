# Polymarket Plugin Changelog

### v0.6.0 (2026-05-05) — Deposit wallet support (POLY_1271 / new user flow)

- **feat**: New `TradingMode::DepositWallet` — ERC-1967 proxy per user, deployed by `DEPOSIT_WALLET_FACTORY`. Fully gasless (relayer-paid). `maker = signer = deposit_wallet_address`, `signature_type = 3` (POLY_1271 / ERC-1271).
- **feat**: `setup-deposit-wallet` command — 6-step onboarding: deploy via relayer WALLET-CREATE → sign 5-target approval batch (pUSD + CTF ERC-1155) → sync CLOB balance-allowance `signature_type=3` → save mode.
- **feat**: `get_existing_deposit_wallet` — on-chain factory probe + `eth_getCode` confirmation. Used by `quickstart` for creds-less recovery.
- **feat**: `buy` / `sell` / `rfq` — DepositWallet branch: maker=signer=deposit_wallet, sig_type=3, `sign_order_v2_poly1271_via_onchainos`. No per-trade POL or approval needed.
- **fix**: `sign_order_v2_poly1271_via_onchainos` — Implements Solady ERC-7739 TypedDataSign composite signature format. Plain EIP-712 over the order hash fails because the deposit wallet's `isValidSignature` re-hashes via the TypedDataSign envelope internally. The EOA now signs `keccak256("\x19\x01" || CTF_Exchange_domain_sep || TypedDataSign_struct_hash)` via onchainos EIP-712 JSON with `primaryType=TypedDataSign`. Wire format: `[65 ECDSA sig] + [32 app_domain_sep] + [32 contents_hash] + [186 ORDER_TYPE_STRING] + [2-byte len]` (317 bytes total). Live-verified: FOK buy matched, tx `0xd064ed98ad9626bbaa17afce05b8307beae0a62c1b3280fa022275364c85f859`.
- **fix**: `buy` / `sell` V2 CLOB amount precision — CLOB V2 rejects orders where maker (USDC) has more than 2 decimal places or taker (shares) has more than 5. `buy` rounds maker UP when within user's requested budget (avoids falling below $1 market minimum), then recomputes taker from adjusted maker. `sell` rounds maker (shares) down to 5dp and taker (USDC received) down to 2dp. Adds `decimal_to_u256_bytes()` for 256-bit token ID ABI encoding without u128 overflow.
- **feat**: `balance` — shows deposit wallet pUSD balance when deposit wallet is configured.
- **feat**: `switch-mode` — accepts `deposit-wallet` in addition to `eoa`/`proxy`.
- **feat**: `quickstart` — on first run (no creds.json): checks for existing proxy, then deposit wallet, then routes to `setup-deposit-wallet` for brand-new users.
- **feat**: Relayer API — `relayer_wallet_create`, `relayer_wallet_batch`, `get_wallet_nonce`, `sync_balance_allowance_deposit_wallet`.
- **feat**: Batch signing — `sign_batch_via_onchainos` (EIP-712 `DepositWallet.Batch` type).
- **compat**: Existing EOA and PolyProxy users unaffected — `creds.json` mode field gates all routing.

### v0.5.1 (2026-04-27) — V2 cutover resilience + QA fixes

- **fix**: `buy.rs` POLY_PROXY V2 allowance check now reads on-chain pUSD allowance (`get_pusd_allowance`) instead of CLOB `/balance-allowance`, which hard-codes `signature_type=0` and scopes the lookup to the EOA address. The bug caused a redundant `proxy_pusd_approve` to fire on every V2 buy after setup-proxy, wasting ~0.01 POL per trade. Source of truth is now consistent with `setup-proxy`.
- **fix (regression from v0.4.11 Bug #3)**: `buy.rs` EOA V1 allowance check restored to on-chain `get_usdc_allowance` (`eth_call`). The v0.5.0 merge regressed this to the CLOB API (`get_balance_allowance`), which returns stale values — causing a redundant unlimited approval on every V1 EOA buy. Both V1 (USDC.e) and V2 (pUSD) EOA paths now use on-chain eth_call for idempotent allowance checks. The CLOB API allowance fetch has been removed from the parallel pre-flight join.
- **fix**: `get_clob_version` now returns `Result<u8>` and bails with a retry hint on network/parse failure, instead of silently defaulting to V1. Prevents `buy`/`sell`/`redeem`/`rfq` from routing V2-era orders through the V1 path during the cutover hour, which would produce confusing 404/405 responses from the upgraded server. `balance` softly degrades to `clob_version: "unknown"` and continues.
- **fix**: `rfq` now resolves series IDs (e.g. `btc-5m`) before calling `resolve_market_token`. Previously, passing a series ID to `rfq --market-id` produced "market not found" because the series resolution step in `buy::run()` was bypassed.
- **fix**: `create-readonly-key` pre-flights the CLOB version and exits with a clear JSON error when the server is still v1, instead of propagating an opaque "Unauthorized/Invalid api key" from the v2-only `/auth/readonly-api-key` endpoint.
- **fix**: `sell` live output now includes `market_id` and `fee_rate_bps` fields, matching the dry-run output schema. These fields were present in `--dry-run` but missing from the real-order response.
- **feat**: `buy.rs` pre-flight POL gas check for POLY_PROXY V2: when a wrap or first-time V2 approve is required, ensure EOA has ≥ 0.05 POL and bail with a clear error otherwise — so users aren't stuck mid-flow at first V2 trade.
- **feat**: `balance` output now includes a top-level `clob_version` field (`V1` / `V2` / `unknown`). Lets users confirm at a glance which exchange path their next trade will hit.
- **chore**: Approval log message updated from "Approving {amount} USDC.e" to "Approving unlimited {token} for {exchange} (one-time)" — makes clear that the approval sets MAX_UINT and only fires once per exchange contract.
- **docs**: SKILL.md — `orders --limit` flag documented; `get-market` output fields split by lookup path (condition_id vs slug); SKILL.md "Overview" section adds "What users see at cutover" subsection.

### v0.5.0 (2026-04-21) — pUSD collateral migration + CLOB v2 completion

- **feat (breaking-compatible)**: Full CLOB v2 support. Plugin auto-detects the active CLOB version via `GET /version` and branches on `OrderVersion::V1` vs `V2`. All new orders use v2 EIP-712 signing: domain version `"2"`, new exchange contracts (`CTF_EXCHANGE_V2 = 0xE111...`, `NEG_RISK_CTF_EXCHANGE_V2 = 0xe222...`), updated order struct (removed `taker`/`nonce`/`feeRateBps`; added `timestamp_ms`/`metadata`/`builder`). V1 orders placed before the upgrade remain placeable if the CLOB reports version 1 — no forced migration for existing users.
- **feat**: `orders` command — list open orders for the authenticated user (`--state OPEN|MATCHED|DELAYED|UNMATCHED`). `--v1` flag queries both live order book and `/data/pre-migration-orders` endpoint, deduplicates by order_id, and surfaces a migration notice when V1 orders are detected. Each order shows `version` (`V1` or `V2`) based on field-presence detection.
- **feat**: `watch` command — poll a market's live trade feed every N seconds (`--interval`, default 5; minimum 2). Tracks high-water timestamp to avoid reprinting; prints new events as JSON lines in chronological order.
- **feat**: `rfq` command — Request-for-Quote block trade flow. Step 1: `POST /rfq/request` → quote ID. Step 2: `GET /rfq/quote/{id}` → display price/amount/expiry. Step 3 (with `--confirm`): sign a V2 EIP-712 order at the quoted price and submit `POST /rfq/confirm`.
- **feat**: `create-readonly-key` command — derive a read-only Polymarket CLOB API key via L1 ClobAuth (`POST /auth/readonly-api-key`). Prints key to stdout; not saved to creds.json. Write operations will be rejected by the CLOB server.
- **feat**: `--order-type FAK` (fill-and-kill) support in `buy` and `sell` — fills as much as possible at or better than the given price, cancels the remainder. Complement to FOK (full-fill or nothing).
- **fix**: Approval in `buy` and `sell` now routes to the correct exchange contract based on CLOB version: V2 orders approved against `CTF_EXCHANGE_V2` / `NEG_RISK_CTF_EXCHANGE_V2`; V1 orders against the legacy v1 addresses. Prevents "not enough allowance" rejections after the v2 upgrade.
- **fix**: `orders` command uses CLOB v2 endpoint `GET /data/orders` (v1's `GET /orders?state=X` returns HTTP 405 in v2). HMAC signature now computed over the base path without query string (v2 requirement). Response parsing updated for paginated format `{"data": [...], "next_cursor": "...", "count": N}`.
- **docs**: SKILL.md updated with `orders`, `watch`, `rfq`, `create-readonly-key` command documentation; FAK order type added to Order Type Selection Guide; Key Contracts section split into v2 (active) and v1 (legacy) tables; CLOB v2 migration note added to Overview.
- **feat**: **pUSD collateral migration** (due ~2026-04-28). Polymarket is replacing USDC.e with pUSD (`0xC011...`) as collateral for V2 exchange contracts. Changes:
  - `buy`: For V2 orders, checks pUSD balance instead of USDC.e. If pUSD is insufficient but USDC.e is sufficient, **auto-wraps** USDC.e → pUSD via the Collateral Onramp (`wrap(USDC_E, recipient, amount)`) before placing the order — no manual intervention required.
  - `buy`: For V2 orders, approves pUSD (not USDC.e) to the V2 exchange contract.
  - `balance`: Shows pUSD balance alongside USDC.e for both EOA and proxy wallets.
  - `redeem`: Passes pUSD (not USDC.e) as `collateralToken` in `redeemPositions` for V2 markets.
  - `withdraw`: Auto-detects whether proxy holds pUSD or USDC.e; withdraws whichever covers the requested amount.
  - `onchainos`: New helpers — `get_pusd_balance`, `wrap_usdc_to_pusd`, `proxy_wrap_usdc_to_pusd`, `withdraw_pusd_from_proxy`.
  - `config`: Added `Contracts::PUSD` and `Contracts::COLLATERAL_ONRAMP` constants.
### v0.4.11 (2026-04-25)

- **fix (Bug #1)**: `onchainos` binary path resolution in non-interactive shells — added `onchainos_bin()` helper that tries `~/.local/bin/onchainos` before falling back to bare `"onchainos"`. Non-interactive shells (e.g. Claude Code) never source `~/.zshrc`, so `~/.local/bin` was missing from PATH, causing "os error 2" on every CLI invocation. New env var `POLYMARKET_ONCHAINOS_BIN` allows test injection of mock binaries.
- **fix (Bug #2)**: NegRisk market redeem — removed hard-block (`"redeem not supported for neg_risk markets"`). Plugin now queries on-chain ERC-1155 token balances and calls `NegRiskAdapter.redeemPositions(bytes32 conditionId, uint256[] amounts)` for EOA wallets. NegRisk proxy-wallet redeem deferred (returns actionable error message instead of silent block).
- **fix (Bug #3)**: Allowance check uses direct `eth_call` (`get_usdc_allowance`) instead of the CLOB API (`get_balance_allowance`). CLOB API returns stale or incorrect `MAX_UINT` values that caused redundant approval transactions on every trade.
- **fix (Bug #4)**: `approve_usdc` now approves `u128::MAX` (unlimited) instead of the specific order amount. Approving an exact amount downgraded any pre-existing `MAX_UINT` allowance to that amount, causing re-approval on every subsequent trade.
- **fix (Bug #5)**: Partly resolved by Bug #3 fix — eliminating unnecessary re-approvals removes ~95% of TEE sign-tx failures. Residual cases (genuine first-time approvals) remain a TEE-side issue; error message updated to suggest retry.
- **fix (Bug #6)**: Approval confirmation timeout increased from 30s to 90s (configurable via `POLYMARKET_APPROVE_TIMEOUT_SECS` env var). 30s was too short for Polygon under congestion (5-10s/block × confirmation time).
- **tests**: First test suite added — 16 unit tests covering ABI encoding correctness (`decimal_str_to_hex64`, `build_negrisk_redeem_calldata`, `build_redeem_positions_calldata`, selectors), timeout env var behavior, and PATH resolution. All tests run with `cargo test` without network access.

### v0.4.10 (2026-04-22)

- **feat**: Strategy attribution reporting — `buy` / `sell` / `redeem` each accept an optional `--strategy-id <id>`. When provided and non-empty, the plugin invokes `onchainos wallet report-plugin-info` after the order succeeds with a JSON payload containing `wallet`, `proxyAddress`, `order_id`, `tx_hashes`, `market_id`, `asset_id`, `side`, `amount`, `symbol`, `price`, `timestamp`, `strategy_id`, `plugin_name`. Omitting the flag skips reporting entirely. Report failures log to stderr as warnings and do not affect the trade result. `symbol` encodes the collateral / quote asset (Polymarket: `USDC.e`).

### v0.4.6 (2026-04-15)

- **chore**: Version bump.

### v0.4.5 (2026-04-15)

- **fix**: Correct GCD divisibility step in `buy` and `sell` — minimum order is now 1 share (≈$1) instead of an inflated 10 shares (≈$9.87) for prices coprime with 10,000,000 (e.g. 0.987, 0.983, 0.991). The `tick_scale * 10_000` factor in the original GCD formula caused `gcd = 1` for such prices, making `step_raw = 10,000,000` (10 shares). New formula aligns to whole shares (1,000,000 raw) and computes the smallest valid share count from `gcd(price_ticks × SHARE_RAW, tick_scale)`.

### v0.4.2 (2026-04-14)

- **feat**: `get-series` command — get current/next slot for 12 recurring Up/Down series markets (BTC/ETH/SOL/XRP × 5m/15m/4h). Returns `condition_id`, `up_token_id`, `down_token_id`, prices, and window times. NYSE trading hours enforced for 5m/15m series; 4h runs 24/7.
- **feat**: Series ID routing in `buy` and `sell` — pass `--market-id btc-5m` (or any series ID) directly; resolves current active slot automatically.
- **feat**: `--token-id` fast path for `buy` and `sell` — skip all market resolution when token ID is known (from `get-series` output). `--market-id` optional when `--token-id` provided.
- **fix (C1)**: GCD amount alignment bug fixed in both `buy` and `sell`. `gcd(step_raw, 100)*100` → `gcd(step_raw, 10_000)*10_000`. Fixes order rejection on `tick_size=0.001` markets.
- **fix (M2)**: `setup-proxy --dry-run` no longer writes credentials or switches mode. Both `mode_switched` and `recovered` branches guarded by `if !dry_run`.
- **fix (N6)**: `sell` output no longer includes `maker_amount_raw`/`taker_amount_raw` raw fields.
- **docs**: SKILL.md — `get-series` section added; `--token-id` flag in buy/sell tables; `--market-id` marked optional*; `positions`, `list-5m`, `withdraw` added to command table.

### v0.4.1 (2026-04-14)

- **feat**: `deposit` smart advisor — when `--amount` is omitted, scans Polygon and all bridge chains in parallel, returns alternatives ranked by available USD value with `recommended_command` and `hint` fields.
- **fix**: `deposit` blocks native coin deposits (ETH, BNB, sentinel `0xEeee...`) before any on-chain action; error message suggests wrapped alternative (WETH, WBNB).
- **feat**: `onchainos::get_chain_balances(chain)` — calls onchainos wallet balance and returns token list with `usd_value`; silent on failure.

### v0.4.0 (2026-04-14)

- **feat**: `list-5m` command — list upcoming 5-minute crypto Up/Down markets for BTC, ETH, SOL, XRP, BNB, DOGE, HYPE.
- **feat**: Multi-chain `deposit` — `--chain`, `--token`, `--list` flags. Supports bridging from Ethereum, Arbitrum, Base, Optimism, BNB, Monad.
- **feat**: `list-markets --breaking` — filter by 24h volume hottest events.
- **feat**: `list-markets --category <sports|elections|crypto>` — filter markets by category.
- **feat**: Geo check added to `buy` and `sell` — hard fail before any trading attempt if region is restricted.
- **feat**: EOA POL balance check in `buy` and `sell` — warns before approval/signing if POL < 0.01.

### v0.3.0 (2026-04-13)

- **feat**: POLY_PROXY trading mode. New `setup-proxy` command deploys a Polymarket proxy wallet (one-time POL gas); subsequent `buy`/`sell` orders are relayer-paid (no POL per trade). `setup-proxy` runs 6 on-chain approvals (USDC.e + CTF for all 3 exchanges) idempotently at setup time — no per-trade approve calls.
- **feat**: `balance` command shows POL and USDC.e for EOA and proxy wallet (if initialized).
- **feat**: `deposit` transfers USDC.e from EOA → proxy wallet; `withdraw` transfers back (proxy → EOA).
- **feat**: `switch-mode --mode eoa|proxy` changes the persistent default trading mode.
- **feat**: `buy --mode eoa|proxy` and `sell --mode eoa|proxy` override mode for a single order without changing the stored default.
- **feat**: `get-positions` now auto-queries the proxy wallet in POLY_PROXY mode; displays `pol_balance` and `usdc_e_balance` in EOA mode. Filters out zero-value resolved losing positions (Data API cache persists these after on-chain redeem).
- **feat**: `positions` alias for `get-positions`.
- **fix**: `sell` in POLY_PROXY mode no longer fails with "insufficient token balance" — CLOB API `/balance-allowance` returns 0 for proxy wallets regardless of actual balance; pre-flight check now skipped for proxy mode.
- **fix**: Mode-mismatch error messages: `buy` in EOA mode with no USDC.e hints `polymarket deposit` (proxy mode) or top-up; `sell` in EOA mode with no tokens hints `polymarket switch-mode --mode proxy`.
- **fix**: RPC updated from `polygon-rpc.com` → `polygon.drpc.org` for improved reliability.

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
