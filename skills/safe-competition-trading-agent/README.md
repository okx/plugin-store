# Safe Competition Trading Agent

Competition-aware Agentic Wallet trading skill with risk-gated execution.

This plugin is designed as the main strategy skill for OKX Agentic Wallet trading competitions. It uses onchainOS as the primary data and trading layer, then composes a safe workflow with `agent-workflow-composer` and blocks unsafe trades with `agent-risk-firewall`.

## Why This Exists

Agentic trading competitions need more than a simple swap command. A useful agent must know:

- whether the competition is active;
- whether the wallet has joined;
- which chains count for the competition;
- whether the selected pair is an eligible real-token trade;
- whether quote, slippage, price impact, and transaction context are safe enough;
- whether the user has explicitly confirmed execution.

`safe-competition-trading-agent` turns those checks into an ordered strategy workflow.

## What It Does

| Area | Behavior |
| --- | --- |
| Competition preflight | Plans `onchainos competition list/detail/user-status` before trading. |
| Candidate selection | Scores supplied onchainOS token candidates and prefers liquid, lower-risk real tokens. |
| Quote and tx context | Requires onchainOS swap quote and unsigned transaction preparation before firewall. |
| Workflow validation | Uses a deterministic plan that places composer and firewall before execution. |
| Risk gate | Requires `agent-risk-firewall` verdict before execution. |
| Execution safety | Defaults to dry-run; live execution requires confirmation and an allow/warn verdict. |

## Commands

```powershell
safe-competition-trading-agent plan --input request.json --format json
safe-competition-trading-agent dry-run --input request.json --format json
safe-competition-trading-agent execute --input request.json --format json
safe-competition-trading-agent validate --input plan.json --format json
safe-competition-trading-agent template --name competition-safe-swap
safe-competition-trading-agent self-test
```

## Example Request

```json
{
  "intent": "Dry-run a safe competition trade with a 10 USD budget.",
  "chain": "xlayer",
  "goal": "safe-volume",
  "budgetUsd": "10",
  "tokenIn": {"symbol": "USDC"},
  "executionMode": "dry-run"
}
```

## Live Execution Boundary

The CLI does not sign or broadcast transactions by itself. It returns a guarded execution command only when:

- `executionMode` is `confirm-before-execute`;
- `confirmed` is `true`;
- risk verdict is `allow`, or `warn` with confirmation;
- the workflow has passed all safety gates.

The host agent still executes through onchainOS Agentic Wallet.

## Testing

From the repository root:

```powershell
$env:PYTHONDONTWRITEBYTECODE = "1"
python -m pytest .\skills\safe-competition-trading-agent\tests -q -p no:cacheprovider
& "$env:USERPROFILE\.local\bin\plugin-store.exe" lint .\skills\safe-competition-trading-agent
```

Expected results:

```text
tests passed
Plugin 'safe-competition-trading-agent' passed all checks
```

## Safety Notes

- Do not execute before competition detail and user-status are known.
- Do not execute before quote and unsigned transaction context exist.
- Do not execute before `agent-risk-firewall check`.
- Do not execute on a `block` verdict.
- Do not use `--force`.
- Do not request, store, or export private keys, seed phrases, or mnemonics.

## License

MIT
