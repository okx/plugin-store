# Pendle Plugin Changelog

### v0.2.5 (2026-04-16)

- **fix** (C1): All 8 write commands (`buy-pt`, `sell-pt`, `buy-yt`, `sell-yt`, `add-liquidity`, `remove-liquidity`, `mint-py`, `redeem-py`) now require `--confirm` to execute. Without `--confirm`, the command calls the Pendle SDK and returns a preview JSON (wallet, router, calldata, required_approvals) with no on-chain action. Fixes silent execution with no user gate.
- **fix** (M3): `mint-py` hardcodes ERC-20 approval for `token_in` → router when the Pendle SDK returns an empty `requiredApprovals` array and `token_in` is not native ETH. Prevents "insufficient allowance" failures for ERC-20 inputs (e.g. weETH, USDC).
- **fix** (M2): `get-market --time-frame` now maps `1D`→`hour`, `1W`→`day`, `1M`→`week` before passing to the Pendle API. Previously the user-facing labels were forwarded raw, always causing HTTP 400.
- **fix** (M1): `list-markets` lifts `impliedApy` and `liquidity` from the nested `details` sub-object to the top level of each result when the top-level fields are null. Fixes null APY/liquidity display caused by a Pendle API field migration.

### v0.2.4 (2026-04-16)

- **fix**: Global flags (`--chain`, `--dry-run`, `--api-key`) now accepted in any position — both before and after the subcommand name. Adds `global = true` to clap arg attributes; `pendle-plugin buy-pt --chain 42161` now works.
- **fix**: Binary self-reported name corrected from `pendle` to `pendle-plugin` (aligns `#[command(name = ...)]` with `[[bin]] name` in Cargo.toml). `pendle-plugin --version` now outputs `pendle-plugin 0.2.4`.
- **refactor**: `sdk_convert` gains explicit `enable_aggregator` parameter (all callers pass `true`). Documents that `false` restricts tokenIn to native SY tokens only.

### v0.2.3 (2026-04-15)

- **fix**: `--dry-run` and other global flags can now be placed after the subcommand name. Previously strict global flag positioning caused argument parsing errors.
- **fix**: `get-market --time-frame` now accepts valid values (`1D`, `1W`, `1M`).
- **fix**: Stale `.claude-plugin/plugin.json` corrected.

### v0.2.2 (2026-04-15)

- **fix**: Version corrected to patch increment from v0.2.1.
