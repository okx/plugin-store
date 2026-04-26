import { describe, expect, it } from "vitest";

import { ThresholdDetector } from "../src/signal/threshold.js";

describe("ThresholdDetector", () => {
  it("throws when exit threshold is not below entry threshold", () => {
    expect(
      () =>
        new ThresholdDetector({
          entryThreshold: 0.7,
          exitThreshold: 0.7,
          dwellMs: 30_000
        })
    ).toThrow("exitThreshold must be lower than entryThreshold.");
  });

  it("enters only after threshold dwell time is satisfied", () => {
    const detector = new ThresholdDetector({
      entryThreshold: 0.7,
      exitThreshold: 0.6,
      dwellMs: 30_000
    });

    expect(detector.feed({ ts: 1_000, mid: 0.71 })).toBeNull();
    expect(detector.feed({ ts: 20_000, mid: 0.72 })).toBeNull();

    const signal = detector.feed({ ts: 31_001, mid: 0.73 });
    expect(signal).toEqual({ kind: "ENTER", ts: 31_001, mid: 0.73 });
    expect(detector.isInPosition()).toBe(true);
  });

  it("resets dwell timer if signal drops under entry threshold", () => {
    const detector = new ThresholdDetector({
      entryThreshold: 0.8,
      exitThreshold: 0.7,
      dwellMs: 10_000
    });

    expect(detector.feed({ ts: 1_000, mid: 0.81 })).toBeNull();
    expect(detector.feed({ ts: 5_000, mid: 0.79 })).toBeNull();

    expect(detector.feed({ ts: 6_000, mid: 0.82 })).toBeNull();
    expect(detector.feed({ ts: 15_000, mid: 0.83 })).toBeNull();

    const signal = detector.feed({ ts: 16_200, mid: 0.84 });
    expect(signal).toEqual({ kind: "ENTER", ts: 16_200, mid: 0.84 });
  });

  it("exits only when price crosses exit threshold", () => {
    const detector = new ThresholdDetector({
      entryThreshold: 0.65,
      exitThreshold: 0.55,
      dwellMs: 0
    });

    const enter = detector.feed({ ts: 1_000, mid: 0.65 });
    expect(enter).toEqual({ kind: "ENTER", ts: 1_000, mid: 0.65 });

    expect(detector.feed({ ts: 2_000, mid: 0.56 })).toBeNull();
    const exit = detector.feed({ ts: 3_000, mid: 0.55 });
    expect(exit).toEqual({ kind: "EXIT", ts: 3_000, mid: 0.55 });
    expect(detector.isInPosition()).toBe(false);
  });

  it("supports immediate entry when dwell is zero", () => {
    const detector = new ThresholdDetector({
      entryThreshold: 0.5,
      exitThreshold: 0.4,
      dwellMs: 0
    });

    const signal = detector.feed({ ts: 100, mid: 0.51 });
    expect(signal).toEqual({ kind: "ENTER", ts: 100, mid: 0.51 });
  });
});
