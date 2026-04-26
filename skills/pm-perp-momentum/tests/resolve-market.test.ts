import { describe, expect, it } from "vitest";

import {
  extractSlugFromInput,
  extractTokenIds,
  resolveMarketFromInput
} from "../src/runtime/resolve-market.js";

describe("resolve-market helpers", () => {
  it("extracts slug from raw slug input", () => {
    expect(extractSlugFromInput("fed-decision-october")).toBe(
      "fed-decision-october"
    );
  });

  it("extracts slug from polymarket event URL", () => {
    expect(
      extractSlugFromInput(
        "https://polymarket.com/event/fed-decision-october?tid=123"
      )
    ).toBe("fed-decision-october");
  });

  it("extracts token ids from mixed market payload", () => {
    const tokenIds = extractTokenIds({
      clobTokenIds: ["111", "222"],
      tokens: [{ token_id: "333" }]
    });
    expect(tokenIds).toEqual(["111", "222", "333"]);
  });

  it("resolves market payload with fetch mock", async () => {
    const fakeFetch = () =>
      Promise.resolve({
        ok: true,
        status: 200,
        text: () =>
          Promise.resolve(
            JSON.stringify({
              id: "42",
              slug: "fed-decision-october",
              question: "Will the Fed cut rates?",
              active: true,
              clobTokenIds: ["1001", "1002"]
            })
          )
      });

    const result = await resolveMarketFromInput(
      "fed-decision-october",
      fakeFetch
    );
    expect(result.slug).toBe("fed-decision-october");
    expect(result.tokenIds).toEqual(["1001", "1002"]);
  });
});
