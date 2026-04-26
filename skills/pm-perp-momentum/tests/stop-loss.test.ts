import { describe, expect, it } from "vitest";

import { StopLossManager, computeStopLossTrigger } from "../src/risk/stop-loss.js";

describe("stop-loss manager", () => {
  it("computes long trigger below entry", () => {
    const trigger = computeStopLossTrigger(3000, "LONG", 10);
    expect(trigger).toBeCloseTo(2700, 10);
  });

  it("computes short trigger above entry", () => {
    const trigger = computeStopLossTrigger(3000, "SHORT", 10);
    expect(trigger).toBeCloseTo(3300, 10);
  });

  it("triggers long stop-loss when mark price drops to trigger", () => {
    const manager = new StopLossManager({
      markPriceTrigger: 2700,
      side: "LONG",
      stopLossPct: 8
    });

    expect(manager.shouldTrigger(2701)).toBe(false);
    expect(manager.shouldTrigger(2700)).toBe(true);
  });

  it("triggers short stop-loss when mark price rises to trigger", () => {
    const manager = new StopLossManager({
      markPriceTrigger: 3300,
      side: "SHORT",
      stopLossPct: 8
    });

    expect(manager.shouldTrigger(3299)).toBe(false);
    expect(manager.shouldTrigger(3300)).toBe(true);
  });
});
