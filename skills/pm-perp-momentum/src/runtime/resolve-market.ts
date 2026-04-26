interface ResolveMarketArgs {
  input: string;
  json: boolean;
}

interface FetchLikeResponse {
  ok: boolean;
  status: number;
  text(): Promise<string>;
}

type FetchLike = (url: string, init?: RequestInit) => Promise<FetchLikeResponse>;

interface ResolvedMarket {
  id: string;
  slug: string;
  question: string;
  active: boolean;
  tokenIds: string[];
}

const REQUEST_TIMEOUT_MS = 10_000;
const MAX_ATTEMPTS = 3;

function toStringValue(input: unknown): string | undefined {
  if (typeof input === "string") {
    return input;
  }
  if (typeof input === "number" || typeof input === "boolean") {
    return String(input);
  }
  return undefined;
}

function asRecord(input: unknown): Record<string, unknown> | null {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    return null;
  }
  return input as Record<string, unknown>;
}

function parseJsonPayload<T>(text: string): T {
  try {
    return JSON.parse(text) as T;
  } catch {
    throw new Error("The API returned invalid JSON.");
  }
}

function maybeParseJsonStringArray(input: unknown): string[] {
  if (typeof input !== "string") {
    return [];
  }

  try {
    const parsed = JSON.parse(input) as unknown;
    return Array.isArray(parsed) ? parsed.map(String) : [];
  } catch {
    return [];
  }
}

export function extractSlugFromInput(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) {
    throw new Error("Input cannot be empty.");
  }

  if (!trimmed.includes("://")) {
    return trimmed;
  }

  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    throw new Error("Input URL is not valid.");
  }

  const parts = url.pathname.split("/").filter(Boolean);
  const eventIndex = parts.indexOf("event");
  const eventSlug = eventIndex >= 0 ? parts[eventIndex + 1] : undefined;
  if (eventSlug) {
    return decodeURIComponent(eventSlug);
  }

  const last = parts.at(-1);
  if (!last) {
    throw new Error("Could not extract market slug from URL.");
  }
  return decodeURIComponent(last);
}

export function extractTokenIds(rawMarket: unknown): string[] {
  const market = asRecord(rawMarket);
  if (!market) {
    return [];
  }

  const output = new Set<string>();

  const directArrayCandidates: unknown[] = [
    market.clobTokenIds,
    market.clob_token_ids,
    market.tokenIds,
    market.token_ids,
    market.assets_ids
  ];

  for (const candidate of directArrayCandidates) {
    if (Array.isArray(candidate)) {
      for (const id of candidate) {
        const parsed = toStringValue(id);
        if (parsed) {
          output.add(parsed);
        }
      }
    }
    for (const id of maybeParseJsonStringArray(candidate)) {
      output.add(String(id));
    }
  }

  const tokenObjectsCandidates: unknown[] = [market.tokens, market.outcomes];
  for (const candidate of tokenObjectsCandidates) {
    if (!Array.isArray(candidate)) {
      continue;
    }
    for (const item of candidate) {
      const record = asRecord(item);
      if (!record) {
        continue;
      }
      const id =
        record.token_id ??
        record.tokenId ??
        record.asset_id ??
        record.id ??
        record.outcomeTokenId;
      if (id !== undefined && id !== null) {
        const parsed = toStringValue(id);
        if (parsed) {
          output.add(parsed);
        }
      }
    }
  }

  return [...output];
}

function parseResolveArgs(argv: string[]): ResolveMarketArgs {
  const args = new Map<string, string>();
  const flags = new Set<string>();
  const valueArgs = new Set(["--input"]);
  const flagArgs = new Set(["--json"]);

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token || !token.startsWith("--")) {
      throw new Error(`Unexpected positional argument: ${token ?? "<empty>"}`);
    }
    if (!valueArgs.has(token) && !flagArgs.has(token)) {
      throw new Error(`Unsupported argument: ${token}`);
    }
    const maybeValue = argv[index + 1];
    if (flagArgs.has(token)) {
      flags.add(token);
      continue;
    }
    if (!maybeValue || maybeValue.startsWith("--")) {
      throw new Error(`Missing value for argument: ${token}`);
    }
    args.set(token, maybeValue);
    index += 1;
  }

  const input = args.get("--input");
  if (!input) {
    throw new Error("Missing required argument: --input <slug-or-url>");
  }

  return {
    input,
    json: flags.has("--json")
  };
}

function parseMarketsApiResponse(payload: unknown): unknown[] {
  if (Array.isArray(payload)) {
    return payload;
  }
  if (payload === null || payload === undefined) {
    return [];
  }
  return [payload];
}

async function fetchJson(
  fetchFn: FetchLike,
  url: string
): Promise<{ ok: boolean; status: number; payload: unknown }> {
  let lastStatus = 0;
  let lastPayload: unknown = "";
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt += 1) {
    const controller = new AbortController();
    const timer = setTimeout(() => {
      controller.abort();
    }, REQUEST_TIMEOUT_MS);
    try {
      const response = await fetchFn(url, {
        headers: {
          Accept: "application/json",
          "User-Agent": "pm-perp-momentum/0.1.0"
        },
        signal: controller.signal
      });
      const raw = await response.text();
      clearTimeout(timer);
      if (response.ok) {
        return {
          ok: true,
          status: response.status,
          payload: parseJsonPayload<unknown>(raw)
        };
      }
      lastStatus = response.status;
      lastPayload = raw;
    } catch (error: unknown) {
      const message =
        error instanceof Error ? error.message : "Unknown network error.";
      lastPayload = message;
    } finally {
      clearTimeout(timer);
    }

    if (attempt < MAX_ATTEMPTS) {
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 250 * 2 ** (attempt - 1));
      });
    }
  }

  return {
    ok: false,
    status: lastStatus,
    payload: lastPayload
  };
}

async function resolveBySlug(
  slug: string,
  fetchFn: FetchLike
): Promise<ResolvedMarket> {
  const encodedSlug = encodeURIComponent(slug);
  const candidateUrls = [
    `https://gamma-api.polymarket.com/markets/slug/${encodedSlug}`,
    `https://gamma-api.polymarket.com/markets?slug=${encodedSlug}`
  ];

  let payload: unknown = null;
  let success = false;
  let status = 0;

  for (const url of candidateUrls) {
    const result = await fetchJson(fetchFn, url);
    status = result.status;
    if (!result.ok) {
      continue;
    }
    payload = result.payload;
    success = true;
    break;
  }

  if (!success) {
    throw new Error(
      `Could not resolve slug "${slug}". API status code: ${status || "unknown"}.`
    );
  }

  const markets = parseMarketsApiResponse(payload);
  if (markets.length === 0) {
    throw new Error(`No market found for slug "${slug}".`);
  }

  const market = asRecord(markets[0]);
  if (!market) {
    throw new Error("Resolved market has an unexpected response format.");
  }

  const tokenIds = extractTokenIds(market);
  if (tokenIds.length === 0) {
    throw new Error(
      "Market was found but no CLOB token IDs were detected in the response."
    );
  }

  return {
    id: toStringValue(market.id) ?? "",
    slug: toStringValue(market.slug) ?? slug,
    question:
      toStringValue(market.question) ??
      toStringValue(market.title) ??
      "Unknown question",
    active: Boolean(market.active ?? true),
    tokenIds
  };
}

export async function resolveMarketFromInput(
  input: string,
  fetchFn: FetchLike = fetch
): Promise<ResolvedMarket> {
  const slug = extractSlugFromInput(input);
  return await resolveBySlug(slug, fetchFn);
}

export async function runResolveMarketCommand(argv: string[]): Promise<void> {
  const args = parseResolveArgs(argv);
  const resolved = await resolveMarketFromInput(args.input);

  if (args.json) {
    process.stdout.write(`${JSON.stringify(resolved, null, 2)}\n`);
    return;
  }

  process.stdout.write(
    [
      "Resolved Polymarket market:",
      `- Question: ${resolved.question}`,
      `- Slug: ${resolved.slug}`,
      `- Market ID: ${resolved.id || "n/a"}`,
      `- Active: ${resolved.active ? "yes" : "no"}`,
      `- Token IDs (${resolved.tokenIds.length}):`,
      ...resolved.tokenIds.map((tokenId, index) => `  ${index + 1}. ${tokenId}`),
      "",
      "Use one of the token IDs above as --pm-market in dry-run/live commands."
    ].join("\n")
  );
}
