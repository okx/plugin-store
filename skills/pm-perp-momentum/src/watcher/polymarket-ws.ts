import { fetchPolymarketMid } from "../execution/adapters.js";

import type {
  ErrorHandler,
  PolymarketTick,
  PolymarketWatcherConfig,
  StatusHandler,
  TickHandler
} from "./types.js";

const DEFAULT_POLL_INTERVAL_MS = 1_000;
const DEFAULT_RECONNECT_BASE_MS = 1_000;
const DEFAULT_RECONNECT_MAX_MS = 30_000;

function asRecord(input: unknown): Record<string, unknown> | null {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    return null;
  }
  return input as Record<string, unknown>;
}

function asNumber(input: unknown): number | undefined {
  if (typeof input === "number" && Number.isFinite(input)) {
    return input;
  }
  if (typeof input === "string" && input.trim().length > 0) {
    const parsed = Number(input);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return undefined;
}

function firstOrderPrice(levels: unknown): number | undefined {
  if (!Array.isArray(levels) || levels.length === 0) {
    return undefined;
  }

  const first = (levels as unknown[])[0];
  if (Array.isArray(first) && first.length > 0) {
    return asNumber(first[0]);
  }

  const record = asRecord(first);
  if (!record) {
    return undefined;
  }

  return asNumber(record.price);
}

export function parsePolymarketTick(
  marketId: string,
  payload: unknown
): PolymarketTick | null {
  const record = asRecord(payload);
  if (!record) {
    return null;
  }

  const payloadMarketId =
    (record.market as string | undefined) ??
    (record.market_id as string | undefined) ??
    (record.asset_id as string | undefined) ??
    (record.token_id as string | undefined) ??
    (record.slug as string | undefined);

  if (payloadMarketId && payloadMarketId !== marketId) {
    return null;
  }

  const bestBid =
    asNumber(record.best_bid) ??
    asNumber(record.bid) ??
    firstOrderPrice(record.bids);

  const bestAsk =
    asNumber(record.best_ask) ??
    asNumber(record.ask) ??
    firstOrderPrice(record.asks);

  const directMid =
    asNumber(record.mid) ??
    asNumber(record.mid_price) ??
    asNumber(record.price) ??
    asNumber(record.p);

  const mid =
    directMid ??
    (bestBid !== undefined && bestAsk !== undefined
      ? (bestBid + bestAsk) / 2
      : undefined);

  if (mid === undefined || mid < 0 || mid > 1) {
    return null;
  }

  return {
    ts: Date.now(),
    marketId,
    bestBid,
    bestAsk,
    mid,
    raw: payload
  };
}

function parsePolymarketPayload(payload: unknown): PolymarketTick | null {
  const record = asRecord(payload);
  if (!record) {
    return null;
  }

  const marketId =
    (record.market as string | undefined) ??
    (record.market_id as string | undefined) ??
    (record.asset_id as string | undefined) ??
    (record.token_id as string | undefined) ??
    (record.slug as string | undefined) ??
    "unknown-market";

  return parsePolymarketTick(marketId, payload);
}

function computeBackoffMs(
  attempt: number,
  baseMs: number,
  maxMs: number
): number {
  const expo = Math.min(maxMs, baseMs * 2 ** Math.max(0, attempt - 1));
  const jitterFactor = 0.8 + Math.random() * 0.4;
  return Math.floor(expo * jitterFactor);
}

export class PolymarketWatcher {
  private readonly marketId: string;
  private readonly pollIntervalMs: number;
  private readonly reconnectBaseMs: number;
  private readonly reconnectMaxMs: number;
  private shouldRun = false;
  private reconnectAttempt = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private pollTimer: NodeJS.Timeout | null = null;
  private inFlight = false;
  private readonly tickHandlers = new Set<TickHandler>();
  private readonly errorHandlers = new Set<ErrorHandler>();
  private readonly statusHandlers = new Set<StatusHandler>();

  public constructor(config: PolymarketWatcherConfig) {
    this.marketId = config.marketId;
    this.pollIntervalMs = config.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS;
    this.reconnectBaseMs = config.reconnectBaseMs ?? DEFAULT_RECONNECT_BASE_MS;
    this.reconnectMaxMs = config.reconnectMaxMs ?? DEFAULT_RECONNECT_MAX_MS;
  }

  public onTick(handler: TickHandler): () => void {
    this.tickHandlers.add(handler);
    return () => {
      this.tickHandlers.delete(handler);
    };
  }

  public onError(handler: ErrorHandler): () => void {
    this.errorHandlers.add(handler);
    return () => {
      this.errorHandlers.delete(handler);
    };
  }

  public onStatus(handler: StatusHandler): () => void {
    this.statusHandlers.add(handler);
    return () => {
      this.statusHandlers.delete(handler);
    };
  }

  public start(): void {
    if (this.shouldRun) {
      return;
    }
    this.shouldRun = true;
    this.connect();
  }

  public stop(): void {
    this.shouldRun = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.pollTimer) {
      clearTimeout(this.pollTimer);
      this.pollTimer = null;
    }
    this.emitStatus({ kind: "disconnected" });
  }

  private connect(): void {
    this.reconnectAttempt = 0;
    this.emitStatus({
      kind: "connected",
      url: "polymarket-plugin subprocess polling"
    });
    this.scheduleNextPoll(0);
  }

  private scheduleNextPoll(delayMs: number): void {
    if (!this.shouldRun) {
      return;
    }
    this.pollTimer = setTimeout(() => {
      this.pollTimer = null;
      void this.pollOnce().catch((error: unknown) => {
        const parsed =
          error instanceof Error ? error : new Error("Unknown polling error.");
        this.emitError(parsed);
        this.scheduleReconnect();
      });
    }, delayMs);
  }

  private async pollOnce(): Promise<void> {
    if (!this.shouldRun) {
      return;
    }
    if (this.inFlight) {
      this.scheduleNextPoll(this.pollIntervalMs);
      return;
    }
    this.inFlight = true;
    try {
      const payload = await this.fetchMarketPayload();
      this.reconnectAttempt = 0;
      this.emitTickIfAny(payload);
      this.scheduleNextPoll(this.pollIntervalMs);
    } finally {
      this.inFlight = false;
    }
  }

  private async fetchMarketPayload(): Promise<unknown> {
    const quote = await fetchPolymarketMid(this.marketId);
    return {
      token_id: this.marketId,
      best_bid: quote.bestBid ?? undefined,
      best_ask: quote.bestAsk ?? undefined,
      mid: quote.mid
    };
  }

  private emitTickIfAny(payload: unknown): void {
    const payloadRecord = asRecord(payload);
    if (payloadRecord && Array.isArray(payloadRecord.price_changes)) {
      for (const item of payloadRecord.price_changes) {
        this.emitTickIfAny(item);
      }
      return;
    }

    const tick = parsePolymarketTick(this.marketId, payload);
    if (!tick) {
      const parsed = parsePolymarketPayload(payload);
      if (parsed && parsed.marketId === this.marketId) {
        for (const handler of this.tickHandlers) {
          handler(parsed);
        }
      }
      return;
    }

    if (tick.marketId !== this.marketId) {
      // Data came through but references another token.
      return;
    }

    if (tick.mid < 0 || tick.mid > 1) {
      return;
    }

    for (const handler of this.tickHandlers) {
      try {
        handler(tick);
      } catch (error) {
        this.emitError(
          error instanceof Error
            ? error
            : new Error("Tick handler threw a non-Error value.")
        );
      }
    }
  }

  private scheduleReconnect(): void {
    if (!this.shouldRun) {
      return;
    }

    this.reconnectAttempt += 1;
    const waitMs = computeBackoffMs(
      this.reconnectAttempt,
      this.reconnectBaseMs,
      this.reconnectMaxMs
    );

    this.scheduleNextPoll(waitMs);

    this.emitStatus({
      kind: "reconnecting",
      attempt: this.reconnectAttempt,
      waitMs
    });
  }

  private emitError(error: Error): void {
    for (const handler of this.errorHandlers) {
      handler(error);
    }
  }

  private emitStatus(
    status: Parameters<StatusHandler>[0]
  ): void {
    for (const handler of this.statusHandlers) {
      handler(status);
    }
  }
}
