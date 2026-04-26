import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";

import { describe, expect, it } from "vitest";

import { runReplayCommand } from "../src/runtime/replay.js";
import { StrategyStateStore } from "../src/state/store.js";

describe("replay command", () => {
  it("uses mark-based stop-loss parity when mark ticks are present", async () => {
    const tempDir = mkdtempSync(join(tmpdir(), "pm-perp-replay-test-"));
    const dbPath = join(tempDir, "state.sqlite");

    const store = new StrategyStateStore(dbPath, {
      mode: "live",
      config: {
        pmMarket: "token-1",
        signalSide: "YES",
        entryThresholdPct: 70,
        exitThresholdPct: 60,
        dwellSeconds: 0,
        perpMarket: "ETH",
        perpSide: "LONG",
        notionalUsd: 1000,
        leverage: 5,
        stopLossPct: 10,
        leaderboardMode: "volume-max",
        entrySlices: 3,
        takeProfitTargetsPct: [1.25, 2.5, 4],
        dailyLossLimitUsd: 1000,
        consecutiveLossLimit: 3,
        recordTicks: true,
        resumeOpenPosition: true
      }
    });

    store.openPosition({
      id: randomUUID(),
      runId: store.getRunId(),
      pmMarket: "token-1",
      signalSide: "YES",
      perpMarket: "ETH",
      perpSide: "LONG",
      notionalUsd: 1000,
      leverage: 5,
      sizeProxy: 1,
      avgEntryPrice: 3000,
      totalFeesUsd: 0,
      filledEntrySize: 1,
      entrySignalMid: 0.71,
      entryMarkPrice: 3000,
      stopLossPct: 10,
      stopLossMarkPrice: 2700,
      remainingSize: 1,
      nextTakeProfitIndex: 0,
      takeProfitTargetsJson: JSON.stringify([1.25, 2.5, 4]),
      openedAtMs: 1
    });
    store.recordTick("token-1", 0.71, 0.71, 1, null, "signal");
    store.recordTick("token-1", 0.71, 0.71, 2, 2690, "mark");
    store.close();

    let output = "";
    const originalWrite = process.stdout.write.bind(process.stdout);
    const patchedWrite: typeof process.stdout.write = (chunk: string | Uint8Array) => {
      output += chunk.toString();
      return true;
    };
    process.stdout.write = patchedWrite;

    try {
      await runReplayCommand(["--state-db", dbPath, "--json"]);
    } finally {
      process.stdout.write = originalWrite;
      rmSync(tempDir, { recursive: true, force: true });
    }

    const parsed = JSON.parse(output) as {
      actions: Array<{ type: string; detail: string }>;
    };
    expect(parsed.actions.some((item) => item.type === "STOP_LOSS")).toBe(true);
    expect(parsed.actions.some((item) => item.detail.includes("source=mark"))).toBe(true);
  });
});
