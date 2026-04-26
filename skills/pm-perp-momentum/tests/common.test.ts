import { describe, expect, it } from "vitest";

import { parseRuntimeArgs } from "../src/runtime/common.js";

describe("parseRuntimeArgs", () => {
  it("applies tx-max preset defaults", () => {
    const config = parseRuntimeArgs([
      "--pm-market",
      "123",
      "--signal-side",
      "YES",
      "--entry-threshold",
      "70",
      "--exit-threshold",
      "60",
      "--dwell-seconds",
      "1",
      "--perp",
      "ETH",
      "--side",
      "LONG",
      "--notional-usd",
      "1000",
      "--leverage",
      "5",
      "--stop-loss-pct",
      "8",
      "--leaderboard-mode",
      "tx-max"
    ]);

    expect(config.entrySlices).toBe(8);
    expect(config.takeProfitTargetsPct).toEqual([0.4, 0.8, 1.2, 1.6]);
    expect(config.dwellSeconds).toBeGreaterThanOrEqual(5);
  });

  it("parses confirm-live and risk-ack flags", () => {
    const config = parseRuntimeArgs([
      "--pm-market",
      "123",
      "--signal-side",
      "YES",
      "--entry-threshold",
      "70",
      "--exit-threshold",
      "60",
      "--dwell-seconds",
      "30",
      "--perp",
      "ETH",
      "--side",
      "LONG",
      "--notional-usd",
      "1000",
      "--leverage",
      "5",
      "--stop-loss-pct",
      "8",
      "--confirm-live",
      "--risk-ack",
      "I acknowledge leveraged trading risk and accept full responsibility."
    ]);

    expect(config.confirmLive).toBe(true);
    expect(config.riskAcknowledgement).toContain("accept full responsibility");
  });

  it("rejects unknown arguments", () => {
    expect(() =>
      parseRuntimeArgs([
        "--pm-market",
        "123",
        "--signal-side",
        "YES",
        "--entry-threshold",
        "70",
        "--exit-threshold",
        "60",
        "--dwell-seconds",
        "30",
        "--perp",
        "ETH",
        "--side",
        "LONG",
        "--notional-usd",
        "1000",
        "--leverage",
        "5",
        "--stop-loss-pct",
        "8",
        "--unknown-flag"
      ])
    ).toThrow("Unsupported argument");
  });
});
