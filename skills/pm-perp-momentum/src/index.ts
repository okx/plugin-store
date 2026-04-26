import { parseDryRunArgs, runDryRunRuntime } from "./runtime/dry-run.js";
import { runExportProofCommand } from "./runtime/export-proof.js";
import { parseLiveArgs, runLiveRuntime } from "./runtime/live.js";
import { runReplayCommand } from "./runtime/replay.js";
import { runResolveMarketCommand } from "./runtime/resolve-market.js";
import { runSuggestMarketsCommand } from "./runtime/suggest-markets.js";
import { runVerifyProofCommand } from "./runtime/verify-proof.js";

function printHelp(): void {
  process.stdout.write(
    [
      "pm-perp-momentum",
      "",
      "Usage:",
      "  node dist/src/index.js dry-run --pm-market <id> --signal-side <YES|NO> \\",
      "    --entry-threshold <pct> --exit-threshold <pct> --dwell-seconds <seconds> \\",
      "    --perp <market> --side <LONG|SHORT> --notional-usd <usd> \\",
      "    --leverage <n> --stop-loss-pct <pct> [--leaderboard-mode <volume-max|tx-max|address-max>] \\",
      "    [--entry-slices <n>] [--take-profit-targets <pct,pct,...>] [--state-db <path>]",
      "",
      "  node dist/src/index.js live --pm-market <id> --signal-side <YES|NO> \\",
      "    --entry-threshold <pct> --exit-threshold <pct> --dwell-seconds <seconds> \\",
      "    --perp <market> --side <LONG|SHORT> --notional-usd <usd> \\",
      "    --leverage <n> --stop-loss-pct <pct> [--leaderboard-mode <volume-max|tx-max|address-max>] \\",
      "    [--entry-slices <n>] [--take-profit-targets <pct,pct,...>] [--daily-loss-limit-usd <n>] \\",
      "    [--consecutive-loss-limit <n>] [--state-db <path>] [--resume-open-position] \\",
      "    --confirm-live --risk-ack \"I acknowledge leveraged trading risk and accept full responsibility.\"",
      "",
      "  node dist/src/index.js resolve-market --input <slug-or-url> [--json]",
      "  node dist/src/index.js suggest-markets [--limit <n>] [--min-liquidity <n>] [--category <tag>] [--json]",
      "  node dist/src/index.js replay --state-db <path> [--run-id <id>] [--replay-speed <n>] [--json]",
      "  node dist/src/index.js export-proof --state-db <path> [--run-id <id>] [--format <json|csv>] [--output <path>]",
      "  node dist/src/index.js verify-proof --state-db <path> [--run-id <id>] [--json]",
      ""
    ].join("\n")
  );
}

async function main(): Promise<void> {
  const [, , command, ...rest] = process.argv;

  if (!command || command === "--help" || command === "-h") {
    printHelp();
    return;
  }

  if (command === "dry-run") {
    const config = parseDryRunArgs(rest);
    await runDryRunRuntime(config);
    return;
  }

  if (command === "live") {
    const config = parseLiveArgs(rest);
    await runLiveRuntime(config);
    return;
  }

  if (command === "resolve-market") {
    await runResolveMarketCommand(rest);
    return;
  }

  if (command === "suggest-markets") {
    await runSuggestMarketsCommand(rest);
    return;
  }

  if (command === "replay") {
    await runReplayCommand(rest);
    return;
  }

  if (command === "export-proof") {
    await runExportProofCommand(rest);
    return;
  }

  if (command === "verify-proof") {
    await runVerifyProofCommand(rest);
    return;
  }

  throw new Error(`Unsupported command: ${command}`);
}

main().catch((error: unknown) => {
  const message =
    error instanceof Error ? error.message : "Unknown runtime error";
  process.stderr.write(`pm-perp-momentum failed: ${message}\n`);
  process.exit(1);
});
