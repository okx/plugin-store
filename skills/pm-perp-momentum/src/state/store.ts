import { mkdirSync } from "node:fs";
import { dirname } from "node:path";
import { createHash, randomUUID } from "node:crypto";

import Database from "better-sqlite3";

export type PositionStatus = "OPEN" | "CLOSED";

export interface StoredPosition {
  id: string;
  runId: string;
  pmMarket: string;
  signalSide: "YES" | "NO";
  perpMarket: string;
  perpSide: "LONG" | "SHORT";
  notionalUsd: number;
  leverage: number;
  sizeProxy: number;
  avgEntryPrice: number | null;
  avgExitPrice: number | null;
  totalFeesUsd: number;
  filledEntrySize: number;
  filledExitSize: number;
  entrySignalMid: number;
  entryMarkPrice: number;
  stopLossPct: number;
  stopLossMarkPrice: number;
  remainingSize: number;
  nextTakeProfitIndex: number;
  takeProfitTargetsJson: string;
  openedAtMs: number;
  closedAtMs: number | null;
  closeReason: string | null;
  realizedPnlUsd: number | null;
  status: PositionStatus;
}

export interface PositionOpenInput {
  id: string;
  runId: string;
  pmMarket: string;
  signalSide: "YES" | "NO";
  perpMarket: string;
  perpSide: "LONG" | "SHORT";
  notionalUsd: number;
  leverage: number;
  sizeProxy: number;
  avgEntryPrice: number | null;
  totalFeesUsd: number;
  filledEntrySize: number;
  entrySignalMid: number;
  entryMarkPrice: number;
  stopLossPct: number;
  stopLossMarkPrice: number;
  remainingSize: number;
  nextTakeProfitIndex: number;
  takeProfitTargetsJson: string;
  openedAtMs: number;
}

export interface EventRow {
  id: number;
  runId: string;
  eventType: string;
  message: string;
  payloadJson: string | null;
  previousHash: string | null;
  currentHash: string;
  createdAtMs: number;
}

export interface TickRow {
  id: number;
  runId: string;
  marketId: string;
  tickKind: "signal" | "mark";
  signalMid: number;
  rawMid: number;
  ts: number;
  markPrice: number | null;
}

export interface RunRecord {
  runId: string;
  mode: string;
  fingerprint: string;
  configJson: string;
  label: string | null;
  createdAtMs: number;
}

function toStringValue(input: unknown): string | null {
  if (typeof input === "string") {
    return input;
  }
  if (typeof input === "number" || typeof input === "boolean") {
    return String(input);
  }
  return null;
}

export class StrategyStateStore {
  private readonly db: Database.Database;
  private readonly runId: string;
  private previousEventHash: string;

  public constructor(
    dbPath: string,
    options: { mode: string; config: unknown; runLabel?: string }
  ) {
    mkdirSync(dirname(dbPath), { recursive: true });
    this.db = new Database(dbPath);
    this.db.pragma("journal_mode = WAL");
    this.init();
    this.runId = randomUUID();
    const configJson = JSON.stringify(options.config);
    const fingerprint = createHash("sha256")
      .update(`${options.mode}:${configJson}`)
      .digest("hex");
    this.db
      .prepare(
        `
        INSERT INTO runs (run_id, mode, fingerprint, config_json, label, created_at_ms)
        VALUES (@runId, @mode, @fingerprint, @configJson, @label, @createdAtMs)
      `
      )
      .run({
        runId: this.runId,
        mode: options.mode,
        fingerprint,
        configJson,
        label: options.runLabel ?? null,
        createdAtMs: Date.now()
      });
    this.previousEventHash = "GENESIS";
  }

  public close(): void {
    this.db.close();
  }

  public getRunId(): string {
    return this.runId;
  }

  public withTransaction<T>(fn: () => T): T {
    const txn = this.db.transaction(fn);
    return txn();
  }

  public appendEvent(
    eventType: string,
    message: string,
    payload: Record<string, unknown> | null = null
  ): void {
    const payloadJson = payload ? JSON.stringify(payload) : null;
    const createdAtMs = Date.now();
    const currentHash = createHash("sha256")
      .update(
        [
          this.previousEventHash,
          this.runId,
          eventType,
          message,
          payloadJson ?? "",
          createdAtMs.toString()
        ].join("|")
      )
      .digest("hex");

    const stmt = this.db.prepare(
      `
      INSERT INTO events (
        run_id, event_type, message, payload_json,
        previous_hash, current_hash, created_at_ms
      )
      VALUES (
        @runId, @eventType, @message, @payloadJson,
        @previousHash, @currentHash, @createdAtMs
      )
      `
    );
    stmt.run({
      runId: this.runId,
      eventType,
      message,
      payloadJson,
      previousHash: this.previousEventHash,
      currentHash,
      createdAtMs
    });
    this.previousEventHash = currentHash;
  }

  public getOpenPosition(
    pmMarket: string,
    perpMarket: string,
    options: { currentRunOnly?: boolean } = {}
  ): StoredPosition | null {
    const stmt = this.db.prepare(
      `
      SELECT *
      FROM positions
      WHERE pm_market = @pmMarket
        AND perp_market = @perpMarket
        AND status = 'OPEN'
        AND (@currentRunOnly = 0 OR run_id = @runId)
      ORDER BY opened_at_ms DESC
      LIMIT 1
      `
    );
    const rowScoped = stmt.get({
      pmMarket,
      perpMarket,
      currentRunOnly: options.currentRunOnly ? 1 : 0,
      runId: this.runId
    }) as Record<string, unknown> | undefined;
    return rowScoped ? this.rowToPosition(rowScoped) : null;
  }

  public openPosition(input: PositionOpenInput): void {
    const stmt = this.db.prepare(
      `
      INSERT INTO positions (
        id, run_id, pm_market, signal_side, perp_market, perp_side,
        notional_usd, leverage, size_proxy,
        avg_entry_price, total_fees_usd, filled_entry_size, filled_exit_size, avg_exit_price,
        entry_signal_mid, entry_mark_price, stop_loss_pct, stop_loss_mark_price,
        remaining_size, next_take_profit_index, take_profit_targets_json,
        opened_at_ms, status
      ) VALUES (
        @id, @runId, @pmMarket, @signalSide, @perpMarket, @perpSide,
        @notionalUsd, @leverage, @sizeProxy,
        @avgEntryPrice, @totalFeesUsd, @filledEntrySize, 0, NULL,
        @entrySignalMid, @entryMarkPrice, @stopLossPct, @stopLossMarkPrice,
        @remainingSize, @nextTakeProfitIndex, @takeProfitTargetsJson,
        @openedAtMs, 'OPEN'
      )
      `
    );
    stmt.run(input);
  }

  public updatePositionAfterTakeProfit(
    id: string,
    nextTakeProfitIndex: number,
    remainingSize: number
  ): void {
    const stmt = this.db.prepare(
      `
      UPDATE positions
      SET next_take_profit_index = @nextTakeProfitIndex,
          remaining_size = @remainingSize
      WHERE id = @id
      `
    );
    stmt.run({ id, nextTakeProfitIndex, remainingSize });
  }

  public closePosition(
    id: string,
    reason: string,
    closedAtMs: number,
    realizedPnlUsd: number,
    avgExitPrice: number | null,
    totalFeesUsd: number,
    filledExitSize: number
  ): void {
    const stmt = this.db.prepare(
      `
      UPDATE positions
      SET status = 'CLOSED',
          closed_at_ms = @closedAtMs,
          close_reason = @reason,
          realized_pnl_usd = @realizedPnlUsd,
          avg_exit_price = @avgExitPrice,
          total_fees_usd = @totalFeesUsd,
          filled_exit_size = @filledExitSize
      WHERE id = @id
      `
    );
    stmt.run({
      id,
      reason,
      closedAtMs,
      realizedPnlUsd,
      avgExitPrice,
      totalFeesUsd,
      filledExitSize
    });
  }

  public applyPartialClose(
    id: string,
    remainingSize: number,
    totalFeesUsd: number,
    filledExitSize: number,
    avgExitPrice: number | null
  ): void {
    const stmt = this.db.prepare(
      `
      UPDATE positions
      SET remaining_size = @remainingSize,
          total_fees_usd = @totalFeesUsd,
          filled_exit_size = @filledExitSize,
          avg_exit_price = @avgExitPrice
      WHERE id = @id
      `
    );
    stmt.run({
      id,
      remainingSize,
      totalFeesUsd,
      filledExitSize,
      avgExitPrice
    });
  }

  public recordTick(
    marketId: string,
    signalMid: number,
    rawMid: number,
    ts: number,
    markPrice: number | null = null,
    tickKind: "signal" | "mark" = "signal"
  ): void {
    this.db
      .prepare(
        `
      INSERT INTO replay_ticks (run_id, market_id, tick_kind, signal_mid, raw_mid, ts, mark_price)
      VALUES (@runId, @marketId, @tickKind, @signalMid, @rawMid, @ts, @markPrice)
    `
      )
      .run({
        runId: this.runId,
        marketId,
        tickKind,
        signalMid,
        rawMid,
        ts,
        markPrice
      });
  }

  public applyTradeOutcome(outcomePnlUsd: number): {
    dateKey: string;
    dailyLossUsd: number;
    consecutiveLosses: number;
  } {
    const dateKey = new Date().toISOString().slice(0, 10);
    const existing = this.db
      .prepare(
        `
      SELECT daily_loss_usd, consecutive_losses
      FROM risk_state
      WHERE date_key = @dateKey
    `
      )
      .get({ dateKey }) as Record<string, unknown> | undefined;

    let dailyLossUsd = existing ? Number(existing.daily_loss_usd) : 0;
    let consecutiveLosses = existing ? Number(existing.consecutive_losses) : 0;
    if (outcomePnlUsd < 0) {
      dailyLossUsd += Math.abs(outcomePnlUsd);
      consecutiveLosses += 1;
    } else {
      consecutiveLosses = 0;
    }

    this.db
      .prepare(
        `
      INSERT INTO risk_state (date_key, daily_loss_usd, consecutive_losses, updated_at_ms)
      VALUES (@dateKey, @dailyLossUsd, @consecutiveLosses, @updatedAtMs)
      ON CONFLICT(date_key) DO UPDATE SET
        daily_loss_usd = excluded.daily_loss_usd,
        consecutive_losses = excluded.consecutive_losses,
        updated_at_ms = excluded.updated_at_ms
    `
      )
      .run({
        dateKey,
        dailyLossUsd,
        consecutiveLosses,
        updatedAtMs: Date.now()
      });

    return { dateKey, dailyLossUsd, consecutiveLosses };
  }

  public getRiskState(dateKey = new Date().toISOString().slice(0, 10)): {
    dateKey: string;
    dailyLossUsd: number;
    consecutiveLosses: number;
  } {
    const row = this.db
      .prepare(
        `
      SELECT daily_loss_usd, consecutive_losses
      FROM risk_state
      WHERE date_key = @dateKey
    `
      )
      .get({ dateKey }) as Record<string, unknown> | undefined;
    return {
      dateKey,
      dailyLossUsd: row ? Number(row.daily_loss_usd) : 0,
      consecutiveLosses: row ? Number(row.consecutive_losses) : 0
    };
  }

  public listRuns(limit: number): RunRecord[] {
    return this.db
      .prepare(
        `
      SELECT run_id as runId,
             mode,
             fingerprint,
             config_json as configJson,
             label,
             created_at_ms as createdAtMs
      FROM runs
      ORDER BY created_at_ms DESC
      LIMIT @limit
    `
      )
      .all({ limit }) as RunRecord[];
  }

  public getRun(runId: string): RunRecord | null {
    const row = this.db
      .prepare(
        `
      SELECT run_id as runId,
             mode,
             fingerprint,
             config_json as configJson,
             label,
             created_at_ms as createdAtMs
      FROM runs
      WHERE run_id = @runId
      LIMIT 1
    `
      )
      .get({ runId }) as RunRecord | undefined;
    return row ?? null;
  }

  public listTicks(runId: string): TickRow[] {
    return this.db
      .prepare(
        `
      SELECT id, run_id as runId, market_id as marketId, signal_mid as signalMid, raw_mid as rawMid, ts
             , mark_price as markPrice, tick_kind as tickKind
      FROM replay_ticks
      WHERE run_id = @runId
      ORDER BY ts ASC, id ASC
    `
      )
      .all({ runId }) as TickRow[];
  }

  public listEvents(runId: string): EventRow[] {
    return this.db
      .prepare(
        `
      SELECT id,
             run_id as runId,
             event_type as eventType,
             message,
             payload_json as payloadJson,
             previous_hash as previousHash,
             current_hash as currentHash,
             created_at_ms as createdAtMs
      FROM events
      WHERE run_id = @runId
      ORDER BY id ASC
    `
      )
      .all({ runId }) as EventRow[];
  }

  public listPositions(runId: string): StoredPosition[] {
    const rows = this.db
      .prepare(
        `
      SELECT *
      FROM positions
      WHERE run_id = @runId
      ORDER BY opened_at_ms ASC
    `
      )
      .all({ runId }) as Record<string, unknown>[];
    return rows.map((row) => this.rowToPosition(row));
  }

  private init(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS runs (
        run_id TEXT PRIMARY KEY,
        mode TEXT NOT NULL,
        fingerprint TEXT NOT NULL,
        config_json TEXT NOT NULL,
        label TEXT,
        created_at_ms INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS positions (
        id TEXT PRIMARY KEY,
        run_id TEXT NOT NULL,
        pm_market TEXT NOT NULL,
        signal_side TEXT NOT NULL,
        perp_market TEXT NOT NULL,
        perp_side TEXT NOT NULL,
        notional_usd REAL NOT NULL,
        leverage REAL NOT NULL,
        size_proxy REAL NOT NULL,
        avg_entry_price REAL,
        avg_exit_price REAL,
        total_fees_usd REAL NOT NULL DEFAULT 0,
        filled_entry_size REAL NOT NULL DEFAULT 0,
        filled_exit_size REAL NOT NULL DEFAULT 0,
        entry_signal_mid REAL NOT NULL,
        entry_mark_price REAL NOT NULL DEFAULT 0,
        stop_loss_pct REAL NOT NULL,
        stop_loss_mark_price REAL NOT NULL DEFAULT 0,
        remaining_size REAL NOT NULL,
        next_take_profit_index INTEGER NOT NULL,
        take_profit_targets_json TEXT NOT NULL,
        opened_at_ms INTEGER NOT NULL,
        closed_at_ms INTEGER,
        close_reason TEXT,
        realized_pnl_usd REAL,
        status TEXT NOT NULL CHECK (status IN ('OPEN', 'CLOSED'))
      );

      CREATE TABLE IF NOT EXISTS events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        run_id TEXT NOT NULL,
        event_type TEXT NOT NULL,
        message TEXT NOT NULL,
        payload_json TEXT,
        previous_hash TEXT,
        current_hash TEXT NOT NULL,
        created_at_ms INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS replay_ticks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        run_id TEXT NOT NULL,
        market_id TEXT NOT NULL,
        tick_kind TEXT NOT NULL DEFAULT 'signal',
        signal_mid REAL NOT NULL,
        raw_mid REAL NOT NULL,
        ts INTEGER NOT NULL,
        mark_price REAL
      );

      CREATE TABLE IF NOT EXISTS risk_state (
        date_key TEXT PRIMARY KEY,
        daily_loss_usd REAL NOT NULL,
        consecutive_losses INTEGER NOT NULL,
        updated_at_ms INTEGER NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_positions_lookup
        ON positions (pm_market, perp_market, status);
      CREATE INDEX IF NOT EXISTS idx_positions_run
        ON positions (run_id);
      CREATE INDEX IF NOT EXISTS idx_events_run
        ON events (run_id, id);
      CREATE INDEX IF NOT EXISTS idx_ticks_run
        ON replay_ticks (run_id, ts);
    `);

    const alterStatements = [
      "ALTER TABLE positions ADD COLUMN avg_entry_price REAL",
      "ALTER TABLE positions ADD COLUMN avg_exit_price REAL",
      "ALTER TABLE positions ADD COLUMN total_fees_usd REAL NOT NULL DEFAULT 0",
      "ALTER TABLE positions ADD COLUMN filled_entry_size REAL NOT NULL DEFAULT 0",
      "ALTER TABLE positions ADD COLUMN filled_exit_size REAL NOT NULL DEFAULT 0",
      "ALTER TABLE positions ADD COLUMN entry_mark_price REAL NOT NULL DEFAULT 0",
      "ALTER TABLE positions ADD COLUMN stop_loss_mark_price REAL NOT NULL DEFAULT 0",
      "ALTER TABLE replay_ticks ADD COLUMN mark_price REAL",
      "ALTER TABLE replay_ticks ADD COLUMN tick_kind TEXT NOT NULL DEFAULT 'signal'"
    ];
    for (const statement of alterStatements) {
      try {
        this.db.exec(statement);
      } catch {
        // Column already exists.
      }
    }
  }

  private rowToPosition(row: Record<string, unknown>): StoredPosition {
    return {
      id: String(row.id),
      runId: String(row.run_id),
      pmMarket: String(row.pm_market),
      signalSide: String(row.signal_side) as "YES" | "NO",
      perpMarket: String(row.perp_market),
      perpSide: String(row.perp_side) as "LONG" | "SHORT",
      notionalUsd: Number(row.notional_usd),
      leverage: Number(row.leverage),
      sizeProxy: Number(row.size_proxy),
      avgEntryPrice:
        row.avg_entry_price === null || row.avg_entry_price === undefined
          ? null
          : Number(row.avg_entry_price),
      avgExitPrice:
        row.avg_exit_price === null || row.avg_exit_price === undefined
          ? null
          : Number(row.avg_exit_price),
      totalFeesUsd: Number(row.total_fees_usd ?? 0),
      filledEntrySize: Number(row.filled_entry_size ?? 0),
      filledExitSize: Number(row.filled_exit_size ?? 0),
      entrySignalMid: Number(row.entry_signal_mid),
      entryMarkPrice: Number(row.entry_mark_price ?? 0),
      stopLossPct: Number(row.stop_loss_pct),
      stopLossMarkPrice: Number(row.stop_loss_mark_price ?? 0),
      remainingSize: Number(row.remaining_size),
      nextTakeProfitIndex: Number(row.next_take_profit_index),
      takeProfitTargetsJson: String(row.take_profit_targets_json),
      openedAtMs: Number(row.opened_at_ms),
      closedAtMs:
        row.closed_at_ms === null || row.closed_at_ms === undefined
          ? null
          : Number(row.closed_at_ms),
      closeReason:
        row.close_reason === null || row.close_reason === undefined
          ? null
          : toStringValue(row.close_reason),
      realizedPnlUsd:
        row.realized_pnl_usd === null || row.realized_pnl_usd === undefined
          ? null
          : Number(row.realized_pnl_usd),
      status: String(row.status) as PositionStatus
    };
  }
}
