import { describe, expect, it } from "vitest";

import { parsePolymarketTick } from "../src/watcher/polymarket-ws.js";

describe("parsePolymarketTick", () => {
  it("parses direct mid payload", () => {
    const tick = parsePolymarketTick("fed-cuts", {
      market: "fed-cuts",
      mid: 0.71
    });

    expect(tick).not.toBeNull();
    expect(tick?.mid).toBe(0.71);
  });

  it("parses best bid and ask payload", () => {
    const tick = parsePolymarketTick("fed-cuts", {
      market: "fed-cuts",
      best_bid: "0.40",
      best_ask: "0.44"
    });

    expect(tick).not.toBeNull();
    expect(tick?.mid).toBeCloseTo(0.42, 8);
  });

  it("ignores payload from another market", () => {
    const tick = parsePolymarketTick("fed-cuts", {
      market: "other-market",
      mid: 0.55
    });

    expect(tick).toBeNull();
  });

  it("ignores invalid probability values", () => {
    const tick = parsePolymarketTick("fed-cuts", {
      market: "fed-cuts",
      mid: 1.2
    });

    expect(tick).toBeNull();
  });

  it("parses token_id based payloads", () => {
    const tick = parsePolymarketTick("12345", {
      token_id: "12345",
      best_bid: "0.22",
      best_ask: "0.28"
    });

    expect(tick).not.toBeNull();
    expect(tick?.mid).toBeCloseTo(0.25, 8);
  });
});
