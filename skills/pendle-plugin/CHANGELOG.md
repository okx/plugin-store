# Changelog

## v0.2.8 — 2026-04-21

### Added

- **`quickstart` command**: New onboarding surface that returns a `status` (`active` / `ready` /
  `needs_gas` / `needs_funds` / `no_funds`) based on wallet gas + stablecoin balance + Pendle
  positions, with a concrete `next_command` and `onboarding_steps` for each state. Read-only,
  chain-aware (uses the global `--chain` flag to pick the correct USDC address and native gas
  token). Purely additive — no existing command code was modified.

## v0.2.7 — 2026-04-17

### Changed

- **Binary renamed back to `pendle-plugin`**: The v0.2.4 rename to `pendle` was inconsistent
  with the plugin directory name and the rest of the plugin store. Reverted across all surfaces:
  `Cargo.toml` `[[bin]]`, `plugin.yaml` `binary_name`, `plugin.json` name, clap app name, and
  all SKILL.md command examples and install script paths. The install script now migrates users
  from the old `pendle` binary automatically.

### Documented

- **mint-py `--token-in` accepts any ERC-20**: Live API testing confirmed that any ERC-20 token
  (USDC, USDT, WETH, ARB, WBTC, DAI, etc.) works as `--token-in` via the aggregator routing.
  The market's underlying token mints directly; all others go through a DEX aggregator swap first.
  Only the native ETH sentinel address (`0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee`) is rejected
  by the Pendle Hosted SDK API. SKILL.md updated to reflect the full supported range.

## v0.2.6 — 2026-04-17

### Fixed

- **Install script asset naming**: SKILL.md install script downloaded `pendle-plugin-${TARGET}`
  but CI release assets are named `pendle-${TARGET}` (matching the binary name since v0.2.4).
  Fresh installs from the install script produced 404 errors. Fixed download URL and symlink
  name (`pendle-plugin` → `pendle`). Also cleans up both old and new names for idempotency.

### Documented

- **mint-py: native ETH not supported**: The Pendle SDK returns "Token not found" when
  `0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee` is used as `--token-in` for mint-py.
  SKILL.md now documents this with the correct WETH addresses for Arbitrum and Base.

- **Flag ordering requirement**: Global flags (`--chain`, `--dry-run`, `--confirm`) must
  precede the subcommand. SKILL.md previously (incorrectly) documented that flags work
  after the subcommand. Corrected to show the required ordering.

## v0.2.5 — 2026-04-16

### Fixed

- **M1 — list-markets: impliedApy and liquidity always null**: Pendle API moved `impliedApy`
  and `liquidity` from top-level market fields into a nested `details` sub-object. The plugin
  now lifts both fields back to the top level when the top-level value is null, restoring
  correct APY and TVL display.

- **M2 — get-market: invalid time-frame values rejected by API**: The `--time-frame` flag
  accepted user-facing aliases `1D`, `1W`, `1M` but passed them raw to the Pendle API, which
  expects `hour`, `day`, `week` respectively. The plugin now maps the aliases before the API
  call.

## v0.2.4 — 2026-04-10

### Fixed

- Added global `--confirm` flag (required to broadcast any write transaction)
- Added global `--dry-run` flag (simulate without broadcasting)
- Balance pre-flight checks for all write commands
- `mint-py` and `redeem-py` now use Pendle v2 GET SDK endpoint (fixes classification errors)
- Added `get-market-info` command and `--market-id` alias
- Binary renamed from `pendle-plugin` to `pendle`
