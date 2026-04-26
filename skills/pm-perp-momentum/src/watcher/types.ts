export interface PolymarketTick {
  ts: number;
  marketId: string;
  bestBid?: number;
  bestAsk?: number;
  mid: number;
  raw: unknown;
}

export interface PolymarketWatcherConfig {
  marketId: string;
  pollIntervalMs?: number;
  reconnectBaseMs?: number;
  reconnectMaxMs?: number;
}

export type TickHandler = (tick: PolymarketTick) => void;
export type ErrorHandler = (error: Error) => void;

export type WatcherStatus =
  | { kind: "connected"; url: string }
  | { kind: "disconnected" }
  | { kind: "reconnecting"; attempt: number; waitMs: number };

export type StatusHandler = (status: WatcherStatus) => void;
