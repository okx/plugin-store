import type { PluginResult } from "./subprocess.js";
import { runPlugin } from "./subprocess.js";

interface RecordLike {
  [key: string]: unknown;
}

function asRecord(input: unknown): RecordLike | null {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    return null;
  }
  return input as RecordLike;
}

function asNumber(input: unknown): number | null {
  if (typeof input === "number" && Number.isFinite(input)) {
    return input;
  }
  if (typeof input === "string" && input.trim().length > 0) {
    const parsed = Number(input);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return null;
}

function deepSearchNumber(
  input: unknown,
  keys: string[],
  depth = 0
): number | null {
  if (depth > 6) {
    return null;
  }
  const record = asRecord(input);
  if (!record) {
    return null;
  }

  for (const key of keys) {
    const value = record[key];
    const direct = asNumber(value);
    if (direct !== null) {
      return direct;
    }
  }

  for (const value of Object.values(record)) {
    if (Array.isArray(value)) {
      for (const item of value) {
        const nested = deepSearchNumber(item, keys, depth + 1);
        if (nested !== null) {
          return nested;
        }
      }
      continue;
    }
    const nested = deepSearchNumber(value, keys, depth + 1);
    if (nested !== null) {
      return nested;
    }
  }

  return null;
}

export interface HyperliquidExecution {
  filledSize: number;
  avgPrice: number | null;
  feeUsd: number;
  realizedPnlUsd: number | null;
  raw: PluginResult;
}

export interface MarkPriceQuote {
  markPrice: number;
  raw: PluginResult;
}

function parseOrderExecution(result: PluginResult): HyperliquidExecution | null {
  const payload = result.parsedJson;
  if (payload === undefined) {
    return null;
  }

  const filledSize =
    deepSearchNumber(payload, [
      "filledSize",
      "filled_size",
      "sizeFilled",
      "executedSize",
      "qty",
      "quantity"
    ]) ?? 0;
  const avgPrice = deepSearchNumber(payload, [
    "avgPrice",
    "averagePrice",
    "fillPrice",
    "price",
    "executionPrice"
  ]);
  const feeUsd =
    deepSearchNumber(payload, ["feeUsd", "feesUsd", "fee", "fees"]) ?? 0;
  const realizedPnlUsd = deepSearchNumber(payload, [
    "realizedPnlUsd",
    "realizedPnl",
    "pnlUsd",
    "pnl"
  ]);

  return {
    filledSize,
    avgPrice,
    feeUsd,
    realizedPnlUsd,
    raw: result
  };
}

export async function executeHyperliquidOrder(args: string[]): Promise<HyperliquidExecution> {
  const result = await runPlugin("hyperliquid-plugin", args);
  if (!result.ok) {
    throw new Error(
      `Hyperliquid order failed: ${result.stderr || "Unknown plugin error."}`
    );
  }
  const parsed = parseOrderExecution(result);
  if (parsed) {
    return parsed;
  }

  return {
    filledSize: 0,
    avgPrice: null,
    feeUsd: 0,
    realizedPnlUsd: null,
    raw: result
  };
}

function parseMarkPrice(result: PluginResult): number | null {
  const payload = result.parsedJson;
  if (payload === undefined) {
    return null;
  }
  return deepSearchNumber(payload, [
    "markPrice",
    "mark_price",
    "midPrice",
    "mid_price",
    "price",
    "oraclePrice"
  ]);
}

export async function fetchHyperliquidMarkPrice(
  market: string
): Promise<MarkPriceQuote> {
  const attempts: string[][] = [
    ["price", "--market", market, "--json"],
    ["mark-price", "--market", market, "--json"],
    ["ticker", "--market", market, "--json"],
    ["quote", "--market", market, "--json"]
  ];

  const failures: string[] = [];
  for (const args of attempts) {
    const result = await runPlugin("hyperliquid-plugin", args);
    if (!result.ok) {
      failures.push(`${result.command} :: ${result.stderr || "unknown error"}`);
      continue;
    }
    const markPrice = parseMarkPrice(result);
    if (markPrice !== null) {
      return { markPrice, raw: result };
    }
  }

  throw new Error(
    `Failed to fetch Hyperliquid mark price. Attempts: ${failures.join(" | ")}`
  );
}

function parsePolymarketMid(result: PluginResult): {
  mid: number;
  bestBid: number | null;
  bestAsk: number | null;
} | null {
  const payload = result.parsedJson;
  if (payload === undefined) {
    return null;
  }

  const mid = deepSearchNumber(payload, ["mid", "midPrice", "mid_price", "price"]);
  const bestBid = deepSearchNumber(payload, ["bestBid", "best_bid", "bid"]);
  const bestAsk = deepSearchNumber(payload, ["bestAsk", "best_ask", "ask"]);

  if (mid === null) {
    if (bestBid !== null && bestAsk !== null) {
      return {
        mid: (bestBid + bestAsk) / 2,
        bestBid,
        bestAsk
      };
    }
    return null;
  }

  return {
    mid,
    bestBid,
    bestAsk
  };
}

export async function fetchPolymarketMid(marketId: string): Promise<{
  mid: number;
  bestBid: number | null;
  bestAsk: number | null;
  raw: PluginResult;
}> {
  const attempts: string[][] = [
    ["market", "--token-id", marketId, "--json"],
    ["market", "--market-id", marketId, "--json"],
    ["price", "--token-id", marketId, "--json"],
    ["book", "--token-id", marketId, "--json"]
  ];

  const failures: string[] = [];
  for (const args of attempts) {
    const result = await runPlugin("polymarket-plugin", args);
    if (!result.ok) {
      failures.push(`${result.command} :: ${result.stderr || "unknown error"}`);
      continue;
    }
    const parsed = parsePolymarketMid(result);
    if (parsed && parsed.mid >= 0 && parsed.mid <= 1) {
      return { ...parsed, raw: result };
    }
  }

  throw new Error(
    `Failed to fetch Polymarket market data via polymarket-plugin. Attempts: ${failures.join(
      " | "
    )}`
  );
}
