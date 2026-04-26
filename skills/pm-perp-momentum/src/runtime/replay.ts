import Database from "better-sqlite3";

import { computeStopLossTrigger } from "../risk/stop-loss.js";
import { ThresholdDetector } from "../signal/threshold.js";
import { pctToProb } from "./common.js";

interface ReplayArgs {
  dbPath: string;
  runId?: string;
  speed: number;
  json: boolean;
}

interface ReplayTick {
  ts: number;
  tickKind: "signal" | "mark";
  signalMid: number;
  markPrice: number | null;
}

interface ReplayPosition {
  entryMarkPrice: number;
}

interface ReplayRuntimeConfig {
  entryThresholdPct: number;
  exitThresholdPct: number;
  dwellSeconds: number;
  stopLossPct: number;
  perpSide: "LONG" | "SHORT";
  takeProfitTargetsPct: number[];
}

interface ReplayAction {
  ts: number;
  type: "ENTER" | "TAKE_PROFIT" | "STOP_LOSS" | "EXIT";
  signalMid: number;
  detail: string;
}

function parseArgs(argv: string[]): ReplayArgs {
  const args = new Map<string, string>();
  const flags = new Set<string>();
  const valueArgs = new Set(["--state-db", "--run-id", "--replay-speed"]);
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

  const dbPath = args.get("--state-db");
  if (!dbPath) {
    throw new Error("Missing required argument: --state-db <path>");
  }
  const speed = Number(args.get("--replay-speed") ?? "1");
  if (!Number.isFinite(speed) || speed <= 0) {
    throw new Error("replay-speed must be a positive number.");
  }
  return {
    dbPath,
    runId: args.get("--run-id"),
    speed,
    json: flags.has("--json")
  };
}

function resolveRunId(db: Database.Database, runId?: string): string {
  if (runId) {
    return runId;
  }
  const row = db
    .prepare("SELECT run_id FROM runs ORDER BY created_at_ms DESC LIMIT 1")
    .get() as { run_id: string } | undefined;
  if (!row?.run_id) {
    throw new Error("No run records are available for replay.");
  }
  return row.run_id;
}

function parseConfig(rawConfig: string): ReplayRuntimeConfig {
  const cfg = JSON.parse(rawConfig) as Record<string, unknown>;
  const sideRaw = cfg.perpSide;
  const perpSide =
    sideRaw === "SHORT" || sideRaw === "LONG" ? sideRaw : ("LONG" as const);
  const targetsRaw = cfg.takeProfitTargetsPct;
  const targets = Array.isArray(targetsRaw)
    ? targetsRaw.map((item) => Number(item)).filter((item) => Number.isFinite(item))
    : [1, 2, 3];
  return {
    entryThresholdPct: Number(cfg.entryThresholdPct),
    exitThresholdPct: Number(cfg.exitThresholdPct),
    dwellSeconds: Number(cfg.dwellSeconds),
    stopLossPct: Number(cfg.stopLossPct),
    perpSide,
    takeProfitTargetsPct: targets.length > 0 ? targets : [1, 2, 3]
  };
}

function shouldTakeProfit(
  side: "LONG" | "SHORT",
  entryMid: number,
  signalMid: number,
  targetPct: number
): boolean {
  const ratio = targetPct / 100;
  if (side === "LONG") {
    return signalMid >= entryMid * (1 + ratio);
  }
  return signalMid <= entryMid * (1 - ratio);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

export async function runReplayCommand(argv: string[]): Promise<void> {
  const args = parseArgs(argv);
  const db = new Database(args.dbPath, { readonly: true });
  try {
    const runId = resolveRunId(db, args.runId);
    const run = db
      .prepare("SELECT config_json as configJson FROM runs WHERE run_id = @runId LIMIT 1")
      .get({ runId }) as { configJson: string } | undefined;
    if (!run?.configJson) {
      throw new Error(`Run not found: ${runId}`);
    }

    const cfg = parseConfig(run.configJson);
    const ticks = db
      .prepare(
        "SELECT ts, tick_kind as tickKind, signal_mid as signalMid, mark_price as markPrice FROM replay_ticks WHERE run_id = @runId ORDER BY ts ASC, id ASC"
      )
      .all({ runId }) as ReplayTick[];
    const positions = db
      .prepare(
        "SELECT entry_mark_price as entryMarkPrice FROM positions WHERE run_id = @runId ORDER BY opened_at_ms ASC"
      )
      .all({ runId }) as ReplayPosition[];
    if (ticks.length === 0) {
      throw new Error(
        "No replay ticks were stored for this run. Re-run live mode without --no-record-ticks."
      );
    }

    const detector = new ThresholdDetector({
      entryThreshold: pctToProb(cfg.entryThresholdPct),
      exitThreshold: pctToProb(cfg.exitThresholdPct),
      dwellMs: cfg.dwellSeconds * 1_000
    });

    const actions: ReplayAction[] = [];
    let inPosition = false;
    let entryMid = 0;
    let entryMarkBasis = 0;
    let nextTakeProfitIndex = 0;
    let stopLossTrigger = 0;
    let latestMarkPrice: number | null = null;
    let previousTs = ticks[0]?.ts ?? 0;
    const positionQueue = positions.map((item) => item.entryMarkPrice);

    for (const tick of ticks) {
      const tickKind = tick.tickKind === "mark" ? "mark" : "signal";
      if (tick.markPrice !== null && tick.markPrice !== undefined) {
        latestMarkPrice = tick.markPrice;
      }

      if (tickKind === "mark") {
        if (
          inPosition &&
          latestMarkPrice !== null &&
          ((cfg.perpSide === "LONG" && latestMarkPrice <= stopLossTrigger) ||
            (cfg.perpSide === "SHORT" && latestMarkPrice >= stopLossTrigger))
        ) {
          actions.push({
            ts: tick.ts,
            type: "STOP_LOSS",
            signalMid: tick.signalMid,
            detail: "stop-loss threshold reached source=mark"
          });
          inPosition = false;
        }
        previousTs = tick.ts;
        continue;
      }

      const event = detector.feed({ ts: tick.ts, mid: tick.signalMid });
      if (event?.kind === "ENTER" && !inPosition) {
        inPosition = true;
        entryMid = event.mid;
        entryMarkBasis =
          positionQueue.shift() ?? latestMarkPrice ?? tick.markPrice ?? tick.signalMid;
        nextTakeProfitIndex = 0;
        stopLossTrigger = computeStopLossTrigger(
          entryMarkBasis,
          cfg.perpSide,
          cfg.stopLossPct
        );
        actions.push({
          ts: tick.ts,
          type: "ENTER",
          signalMid: tick.signalMid,
          detail: `entry-mid=${entryMid.toFixed(6)} entry-mark=${entryMarkBasis.toFixed(
            6
          )} stop-loss-mark=${stopLossTrigger.toFixed(6)}`
        });
      }

      if (inPosition) {
        const usesMarkStopLoss = stopLossTrigger > 1;
        if (usesMarkStopLoss && latestMarkPrice === null) {
          previousTs = tick.ts;
          continue;
        }
        const stopLossSource = usesMarkStopLoss
          ? (latestMarkPrice as number)
          : tick.signalMid;
        if (
          (cfg.perpSide === "LONG" && stopLossSource <= stopLossTrigger) ||
          (cfg.perpSide === "SHORT" && stopLossSource >= stopLossTrigger)
        ) {
          actions.push({
            ts: tick.ts,
            type: "STOP_LOSS",
            signalMid: tick.signalMid,
            detail: `stop-loss threshold reached source=${
              usesMarkStopLoss ? "mark" : "signal"
            }`
          });
          inPosition = false;
        } else {
          const target = cfg.takeProfitTargetsPct[nextTakeProfitIndex];
          if (
            target !== undefined &&
            shouldTakeProfit(cfg.perpSide, entryMid, tick.signalMid, target)
          ) {
            nextTakeProfitIndex += 1;
            actions.push({
              ts: tick.ts,
              type: "TAKE_PROFIT",
              signalMid: tick.signalMid,
              detail: `target=${target.toFixed(2)}%`
            });
            if (nextTakeProfitIndex >= cfg.takeProfitTargetsPct.length) {
              inPosition = false;
            }
          }
        }
      }

      if (event?.kind === "EXIT" && inPosition) {
        actions.push({
          ts: tick.ts,
          type: "EXIT",
          signalMid: tick.signalMid,
          detail: "signal exit threshold reached"
        });
        inPosition = false;
      }

      if (!args.json) {
        const waitMs = Math.max(0, Math.floor((tick.ts - previousTs) / args.speed));
        if (waitMs > 0 && waitMs <= 1000) {
          // Keep replay practical while preserving deterministic order.
          await sleep(waitMs);
        }
      }
      previousTs = tick.ts;
    }

    const summary = {
      runId,
      totalTicks: ticks.length,
      totalActions: actions.length,
      entries: actions.filter((item) => item.type === "ENTER").length,
      exits: actions.filter((item) => item.type === "EXIT").length,
      stopLosses: actions.filter((item) => item.type === "STOP_LOSS").length,
      takeProfits: actions.filter((item) => item.type === "TAKE_PROFIT").length
    };

    if (args.json) {
      process.stdout.write(`${JSON.stringify({ summary, actions }, null, 2)}\n`);
      return;
    }

    process.stdout.write(
      `Replay summary run-id=${summary.runId} ticks=${summary.totalTicks} actions=${summary.totalActions}\n`
    );
    for (const action of actions) {
      process.stdout.write(
        `- ts=${action.ts} type=${action.type} signal=${action.signalMid.toFixed(
          6
        )} ${action.detail}\n`
      );
    }
  } finally {
    db.close();
  }
}
