import { mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { describe, expect, it } from "vitest";

import { StrategyStateStore } from "../src/state/store.js";

describe("StrategyStateStore", () => {
  it("writes hash-chained events and risk state", () => {
    const tempDir = mkdtempSync(join(tmpdir(), "pm-perp-store-test-"));
    const dbPath = join(tempDir, "state.sqlite");

    const store = new StrategyStateStore(dbPath, {
      mode: "test",
      config: { sample: true }
    });

    store.appendEvent("event_one", "First event.");
    store.appendEvent("event_two", "Second event.");

    const runs = store.listRuns(1);
    expect(runs.length).toBe(1);
    const runId = runs[0]?.runId;
    expect(runId).toBeDefined();

    const events = store.listEvents(runId as string);
    expect(events.length).toBe(2);
    expect(events[0]?.currentHash).toBeTruthy();
    expect(events[1]?.previousHash).toBe(events[0]?.currentHash);

    const stateLoss = store.applyTradeOutcome(-50);
    expect(stateLoss.dailyLossUsd).toBe(50);
    expect(stateLoss.consecutiveLosses).toBe(1);

    const stateWin = store.applyTradeOutcome(10);
    expect(stateWin.consecutiveLosses).toBe(0);

    store.close();
    rmSync(tempDir, { recursive: true, force: true });
  });
});
