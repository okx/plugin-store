import { extractTokenIds } from "./resolve-market.js";

interface SuggestArgs {
  limit: number;
  minLiquidity: number;
  category?: string;
  json: boolean;
}

interface MarketSuggestion {
  slug: string;
  question: string;
  active: boolean;
  volume24h: number;
  liquidity: number;
  tokenIds: string[];
}

const REQUEST_TIMEOUT_MS = 10_000;
const MAX_ATTEMPTS = 3;

function parseArgs(argv: string[]): SuggestArgs {
  const args = new Map<string, string>();
  const flags = new Set<string>();
  const valueArgs = new Set(["--limit", "--min-liquidity", "--category"]);
  const flagArgs = new Set(["--json"]);

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token || !token.startsWith("--")) {
      throw new Error(`Unexpected positional argument: ${token ?? "<empty>"}`);
    }
    if (!valueArgs.has(token) && !flagArgs.has(token)) {
      throw new Error(`Unsupported argument: ${token}`);
    }
    const value = argv[index + 1];
    if (flagArgs.has(token)) {
      flags.add(token);
      continue;
    }
    if (!value || value.startsWith("--")) {
      throw new Error(`Missing value for argument: ${token}`);
    }
    args.set(token, value);
    index += 1;
  }

  const limitRaw = Number(args.get("--limit") ?? "10");
  const minLiquidityRaw = Number(args.get("--min-liquidity") ?? "1000");
  if (!Number.isInteger(limitRaw) || limitRaw <= 0) {
    throw new Error("limit must be a positive integer.");
  }
  if (!Number.isFinite(minLiquidityRaw) || minLiquidityRaw < 0) {
    throw new Error("min-liquidity must be a non-negative number.");
  }

  return {
    limit: limitRaw,
    minLiquidity: minLiquidityRaw,
    category: args.get("--category"),
    json: flags.has("--json")
  };
}

function parseNumber(input: unknown): number {
  if (typeof input === "number" && Number.isFinite(input)) {
    return input;
  }
  if (typeof input === "string") {
    const parsed = Number(input);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return 0;
}

function asRecord(input: unknown): Record<string, unknown> | null {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    return null;
  }
  return input as Record<string, unknown>;
}

function normalizeMarket(input: unknown): MarketSuggestion | null {
  const record = asRecord(input);
  if (!record) {
    return null;
  }

  const tokenIds = extractTokenIds(record);
  if (tokenIds.length === 0) {
    return null;
  }

  return {
    slug: typeof record.slug === "string" ? record.slug : "",
    question:
      typeof record.question === "string"
        ? record.question
        : typeof record.title === "string"
          ? record.title
          : "Unknown question",
    active: Boolean(record.active ?? true),
    volume24h: parseNumber(record.volume24hr ?? record.volume_24hr),
    liquidity: parseNumber(record.liquidity_num ?? record.liquidity),
    tokenIds
  };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

async function fetchMarkets(url: string): Promise<Response> {
  let lastError: Error | null = null;
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt += 1) {
    const controller = new AbortController();
    const timer = setTimeout(() => {
      controller.abort();
    }, REQUEST_TIMEOUT_MS);
    try {
      const response = await fetch(url, {
        headers: {
          Accept: "application/json",
          "User-Agent": "pm-perp-momentum/0.1.0"
        },
        signal: controller.signal
      });
      clearTimeout(timer);
      if (response.ok) {
        return response;
      }
      lastError = new Error(
        `Gamma API request failed with status ${response.status}.`
      );
    } catch (error: unknown) {
      lastError =
        error instanceof Error ? error : new Error("Unknown network error.");
    } finally {
      clearTimeout(timer);
    }
    if (attempt < MAX_ATTEMPTS) {
      await sleep(250 * 2 ** (attempt - 1));
    }
  }
  throw lastError ?? new Error("Gamma API request failed.");
}

export async function runSuggestMarketsCommand(argv: string[]): Promise<void> {
  const args = parseArgs(argv);
  const query = new URLSearchParams({
    active: "true",
    closed: "false",
    limit: String(Math.max(50, args.limit * 5))
  });
  if (args.category) {
    query.set("tag", args.category);
  }

  const response = await fetchMarkets(
    `https://gamma-api.polymarket.com/markets?${query.toString()}`
  );

  const payload = (await response.json()) as unknown;
  const list = Array.isArray(payload) ? payload : [payload];
  const suggestions = list
    .map((item) => normalizeMarket(item))
    .filter((item): item is MarketSuggestion => item !== null)
    .filter((item) => item.active && item.liquidity >= args.minLiquidity)
    .sort((a, b) => b.volume24h - a.volume24h)
    .slice(0, args.limit);

  if (args.json) {
    process.stdout.write(`${JSON.stringify(suggestions, null, 2)}\n`);
    return;
  }

  if (suggestions.length === 0) {
    process.stdout.write(
      "No active markets matched the current filters. Try lowering min-liquidity.\n"
    );
    return;
  }

  process.stdout.write("Suggested active Polymarket markets:\n");
  for (const [index, suggestion] of suggestions.entries()) {
    process.stdout.write(
      [
        `${index + 1}. ${suggestion.question}`,
        `   slug: ${suggestion.slug}`,
        `   volume24h: ${suggestion.volume24h.toFixed(2)}`,
        `   liquidity: ${suggestion.liquidity.toFixed(2)}`,
        `   tokenId[0]: ${suggestion.tokenIds[0] ?? "n/a"}`
      ].join("\n") + "\n"
    );
  }
}
