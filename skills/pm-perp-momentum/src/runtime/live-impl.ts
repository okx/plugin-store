import { randomUUID } from "node:crypto";
import { homedir } from "node:os";
import { join } from "node:path";

import {
  executeHyperliquidOrder,
  fetchHyperliquidMarkPrice
} from "../execution/adapters.js";
import {
  StopLossManager,
  computeStopLossTrigger
} from "../risk/stop-loss.js";
import { ThresholdDetector } from "../signal/threshold.js";
import {
  StrategyStateStore,
  type PositionOpenInput,
  type StoredPosition
} from "../state/store.js";
import { PolymarketWatcher } from "../watcher/polymarket-ws.js";
import {
  adjustedSignalMid,
  clampProbability,
  deriveSizeProxy,
  formatPercent,
  log,
  parseRuntimeArgs,
  pctToProb,
  type RuntimeConfigWithExtras,
  validateRuntimeConfig
} from "./common.js";

export type LiveRuntimeConfig = RuntimeConfigWithExtras;

const DEFAULT_DB_PATH = join(homedir(), ".pm-perp-momentum", "state.sqlite");
const RISK_ACK_REQUIRED_PHRASE =
  "I acknowledge leveraged trading risk and accept full responsibility.";
const MARK_RISK_INTERVAL_MS = 1_000;

interface ActivePosition {
  id: string;
  openedAt: number;
  entrySignalMid: number;
  entryMarkPrice: number;
  direction: "LONG" | "SHORT";
  sizeTotal: number;
  remainingSize: number;
  nextTakeProfitIndex: number;
  takeProfitTargetsPct: number[];
  notionalUsd: number;
  leverage: number;
  avgEntryPrice: number | null;
  totalFeesUsd: number;
  filledEntrySize: number;
  filledExitSize: number;
  lastKnownMarkPrice: number | null;
  stopLossManager: StopLossManager;
}

function toActivePosition(position: StoredPosition): ActivePosition {
  const takeProfitTargetsPct = JSON.parse(position.takeProfitTargetsJson) as number[];
  return {
    id: position.id,
    openedAt: position.openedAtMs,
    entrySignalMid: position.entrySignalMid,
    entryMarkPrice: position.entryMarkPrice,
    direction: position.perpSide,
    sizeTotal: position.sizeProxy,
    remainingSize: position.remainingSize,
    nextTakeProfitIndex: position.nextTakeProfitIndex,
    takeProfitTargetsPct,
    notionalUsd: position.notionalUsd,
    leverage: position.leverage,
    avgEntryPrice: position.avgEntryPrice,
    totalFeesUsd: position.totalFeesUsd,
    filledEntrySize: position.filledEntrySize,
    filledExitSize: position.filledExitSize,
    lastKnownMarkPrice: null,
    stopLossManager: StopLossManager.fromStoredPosition(position)
  };
}

function canTriggerTakeProfit(position: ActivePosition, signalMid: number): boolean {
  const target = position.takeProfitTargetsPct[position.nextTakeProfitIndex];
  if (target === undefined) {
    return false;
  }
  const ratio = target / 100;
  if (position.direction === "LONG") {
    return signalMid >= position.entrySignalMid * (1 + ratio);
  }
  return signalMid <= position.entrySignalMid * (1 - ratio);
}

function takeProfitCloseSize(position: ActivePosition): number {
  const targetsCount = position.takeProfitTargetsPct.length;
  const isFinalTarget = position.nextTakeProfitIndex >= targetsCount - 1;
  if (isFinalTarget) {
    return position.remainingSize;
  }
  const baseSlice = position.sizeTotal / targetsCount;
  return Number(Math.min(position.remainingSize, baseSlice).toFixed(8));
}

function computeFallbackPnlUsd(position: ActivePosition, markPrice: number): number {
  const entry = position.avgEntryPrice ?? position.entryMarkPrice;
  const deltaPerUnit =
    position.direction === "LONG" ? markPrice - entry : entry - markPrice;
  return deltaPerUnit * position.filledEntrySize - position.totalFeesUsd;
}

async function executeSlicedEntry(
  config: LiveRuntimeConfig,
  sliceCount: number,
  sizeTotal: number
): Promise<{
  filledSize: number;
  successfulSlices: number;
  failedSlices: number;
  totalFeesUsd: number;
  weightedEntryPrice: number | null;
}> {
  const sliceSize = Number((sizeTotal / sliceCount).toFixed(8));
  let filledSize = 0;
  let successfulSlices = 0;
  let failedSlices = 0;
  let totalFeesUsd = 0;
  let weightedPriceNumerator = 0;
  let weightedSizeDenominator = 0;

  for (let index = 0; index < sliceCount; index += 1) {
    try {
      const order = await executeHyperliquidOrder([
        "order",
        "--market",
        config.perpMarket,
        "--side",
        config.perpSide.toLowerCase(),
        "--size",
        String(sliceSize),
        "--leverage",
        String(config.leverage),
        "--confirm"
      ]);

      const actualFilled = order.filledSize > 0 ? order.filledSize : sliceSize;
      successfulSlices += 1;
      filledSize += actualFilled;
      totalFeesUsd += order.feeUsd;
      if (order.avgPrice !== null) {
        weightedPriceNumerator += order.avgPrice * actualFilled;
        weightedSizeDenominator += actualFilled;
      }
    } catch {
      failedSlices += 1;
    }
  }

  const weightedEntryPrice =
    weightedSizeDenominator > 0
      ? weightedPriceNumerator / weightedSizeDenominator
      : null;

  return {
    filledSize: Number(filledSize.toFixed(8)),
    successfulSlices,
    failedSlices,
    totalFeesUsd: Number(totalFeesUsd.toFixed(8)),
    weightedEntryPrice:
      weightedEntryPrice === null ? null : Number(weightedEntryPrice.toFixed(8))
  };
}

async function closeLivePosition(
  reason: "SIGNAL_EXIT" | "STOP_LOSS",
  config: LiveRuntimeConfig,
  store: StrategyStateStore,
  position: ActivePosition,
  signalMid: number,
  markPrice: number
): Promise<"CLOSED" | "PARTIAL" | "FAILED"> {
  const closeSide = position.direction === "LONG" ? "sell" : "buy";

  let order;
  try {
    order = await executeHyperliquidOrder([
      "order",
      "--market",
      config.perpMarket,
      "--side",
      closeSide,
      "--size",
      String(position.remainingSize),
      "--reduce-only",
      "true",
      "--confirm"
    ]);
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Unknown close order failure.";
    store.appendEvent("live_close_failed", message, { reason });
    log(`live-close-failed reason=${reason} error=${message}`);
    return "FAILED";
  }

  const requestedSize = position.remainingSize;
  const actualClosedRaw = order.filledSize > 0 ? order.filledSize : 0;
  if (actualClosedRaw <= 0) {
    store.appendEvent("live_close_failed", "Close order did not fill.", {
      reason,
      requestedSize,
      command: order.raw.command
    });
    return "FAILED";
  }
  const actualClosed = Number(Math.min(requestedSize, actualClosedRaw).toFixed(8));
  const nextRemaining = Number(Math.max(0, requestedSize - actualClosed).toFixed(8));

  const heldSeconds = Math.floor((Date.now() - position.openedAt) / 1_000);
  const deltaPct = (signalMid - position.entrySignalMid) * 100;
  const avgExitPrice = order.avgPrice ?? markPrice;
  const totalFeesUsd = Number((position.totalFeesUsd + order.feeUsd).toFixed(8));
  const filledExitSize = Number((position.filledExitSize + actualClosed).toFixed(8));
  position.totalFeesUsd = totalFeesUsd;
  position.filledExitSize = filledExitSize;
  position.remainingSize = nextRemaining;

  if (nextRemaining > 0) {
    store.withTransaction(() => {
      store.applyPartialClose(
        position.id,
        nextRemaining,
        totalFeesUsd,
        filledExitSize,
        avgExitPrice
      );
      store.appendEvent("position_partial_close", "Position partially closed.", {
        reason,
        requestedSize,
        closedSize: actualClosed,
        remainingSize: nextRemaining,
        avgExitPrice: Number(avgExitPrice.toFixed(6)),
        command: order.raw.command
      });
    });
    log(
      `live-close partial reason=${reason} closed=${actualClosed} remaining=${nextRemaining}`
    );
    return "PARTIAL";
  }

  const realizedPnlUsd =
    order.realizedPnlUsd ?? computeFallbackPnlUsd(position, markPrice);
  store.withTransaction(() => {
    store.closePosition(
      position.id,
      reason,
      Date.now(),
      realizedPnlUsd,
      avgExitPrice,
      totalFeesUsd,
      filledExitSize
    );
    const riskState = store.applyTradeOutcome(realizedPnlUsd);
    store.appendEvent("position_closed", "Position closed successfully.", {
      reason,
      heldSeconds,
      avgExitPrice: Number(avgExitPrice.toFixed(6)),
      realizedPnlUsd: Number(realizedPnlUsd.toFixed(4)),
      pnlSource: order.realizedPnlUsd === null ? "fallback" : "plugin",
      dailyLossUsd: Number(riskState.dailyLossUsd.toFixed(4)),
      consecutiveLosses: riskState.consecutiveLosses,
      signalDeltaPct: Number(deltaPct.toFixed(4)),
      command: order.raw.command
    });
  });
  log(
    `live-close success reason=${reason} held=${heldSeconds}s signal-delta=${deltaPct.toFixed(
      2
    )}pp`
  );
  return "CLOSED";
}

export function parseLiveArgs(argv: string[]): LiveRuntimeConfig {
  return parseRuntimeArgs(argv);
}

export async function runLiveRuntime(config: LiveRuntimeConfig): Promise<void> {
  validateRuntimeConfig(config);
  if (!config.confirmLive) {
    throw new Error(
      "Live mode requires --confirm-live. Dry-run mode is safer for first-time execution."
    );
  }
  if (config.riskAcknowledgement !== RISK_ACK_REQUIRED_PHRASE) {
    throw new Error(
      `Live mode requires --risk-ack "${RISK_ACK_REQUIRED_PHRASE}".`
    );
  }

  log(
    "CRITICAL: KPI attribution and leaderboard points are ONLY earned when this Skill is executed within the official Onchain OS / Agentic Wallet environment."
  );

  const stateDbPath = config.stateDbPath ?? DEFAULT_DB_PATH;
  const store = new StrategyStateStore(stateDbPath, {
    mode: "live",
    config,
    runLabel: config.runLabel
  });
  const detector = new ThresholdDetector({
    entryThreshold: pctToProb(config.entryThresholdPct),
    exitThreshold: pctToProb(config.exitThresholdPct),
    dwellMs: config.dwellSeconds * 1_000
  });
  const watcher = new PolymarketWatcher({
    marketId: config.pmMarket
  });

  const cleanupFns: Array<() => void> = [];
  const sizeProxy = deriveSizeProxy(config.notionalUsd, config.leverage);
  const takeProfitTargetsPct = [...config.takeProfitTargetsPct];
  let lastPrintedTickAt = 0;
  let processingQueue: Promise<void> = Promise.resolve();
  let stopOpeningTrades = false;
  let activePosition: ActivePosition | null = null;
  let markRiskTimer: NodeJS.Timeout | null = null;
  let lastSignalMid = 0.5;
  let lastRawMid = 0.5;

  const recovered = config.resumeOpenPosition
    ? store.getOpenPosition(config.pmMarket, config.perpMarket)
    : null;
  if (recovered) {
    activePosition = toActivePosition(recovered);
    detector.forceInPosition();
    log(
      `state-recovered open-position id=${recovered.id} stop-loss-mark-price=${recovered.stopLossMarkPrice.toFixed(
        6
      )}`
    );
    store.appendEvent("state_recovered", "Recovered open position from SQLite.", {
      positionId: recovered.id
    });
  } else if (!config.resumeOpenPosition) {
    store.appendEvent(
      "state_recovery_skipped",
      "Open-position recovery disabled for this run."
    );
  }

  const riskBaseline = store.getRiskState();
  if (
    riskBaseline.dailyLossUsd >= config.dailyLossLimitUsd ||
    riskBaseline.consecutiveLosses >= config.consecutiveLossLimit
  ) {
    stopOpeningTrades = true;
    log(
      `risk-gate-engaged on startup daily-loss=${riskBaseline.dailyLossUsd.toFixed(
        2
      )}/${config.dailyLossLimitUsd.toFixed(2)} consecutive-losses=${
        riskBaseline.consecutiveLosses
      }/${config.consecutiveLossLimit}`
    );
  }

  cleanupFns.push(
    watcher.onError((error) => {
      log(`watcher-error: ${error.message}`);
      store.appendEvent("watcher_error", error.message);
    })
  );
  cleanupFns.push(
    watcher.onStatus((status) => {
      if (status.kind === "connected") {
        log(`watcher-connected url=${status.url}`);
        return;
      }
      if (status.kind === "disconnected") {
        log("watcher-disconnected");
        return;
      }
      log(
        `watcher-reconnecting attempt=${status.attempt} wait-ms=${status.waitMs}`
      );
    })
  );

  const handleTick = async (tick: { ts: number; mid: number }): Promise<void> => {
    const adjustedMid = clampProbability(
      adjustedSignalMid(tick.mid, config.signalSide)
    );
    lastSignalMid = adjustedMid;
    lastRawMid = tick.mid;
    if (config.recordTicks) {
      store.recordTick(config.pmMarket, adjustedMid, tick.mid, tick.ts, null, "signal");
    }
    if (Date.now() - lastPrintedTickAt > 3_000) {
      log(
        `tick market=${config.pmMarket} mid=${formatPercent(
          adjustedMid
        )}% signal-side=${config.signalSide}`
      );
      lastPrintedTickAt = Date.now();
    }

    if (activePosition && canTriggerTakeProfit(activePosition, adjustedMid)) {
      const closeSize = takeProfitCloseSize(activePosition);
      const closeSide = activePosition.direction === "LONG" ? "sell" : "buy";
      let tpOrder;
      try {
        tpOrder = await executeHyperliquidOrder([
          "order",
          "--market",
          config.perpMarket,
          "--side",
          closeSide,
          "--size",
          String(closeSize),
          "--reduce-only",
          "true",
          "--confirm"
        ]);
      } catch (error) {
        const message =
          error instanceof Error ? error.message : "Unknown take-profit failure.";
        store.appendEvent("take_profit_failed", message);
        return;
      }

      const targetHit =
        activePosition.takeProfitTargetsPct[activePosition.nextTakeProfitIndex] ?? 0;
      const actualClosed = tpOrder.filledSize > 0 ? tpOrder.filledSize : closeSize;
      activePosition.remainingSize = Number(
        Math.max(0, activePosition.remainingSize - actualClosed).toFixed(8)
      );
      activePosition.filledExitSize += actualClosed;
      activePosition.totalFeesUsd += tpOrder.feeUsd;
      activePosition.nextTakeProfitIndex += 1;

      store.updatePositionAfterTakeProfit(
        activePosition.id,
        activePosition.nextTakeProfitIndex,
        activePosition.remainingSize
      );
      store.appendEvent("take_profit_hit", "Take profit target executed.", {
        targetPct: targetHit,
        closedSize: actualClosed,
        remainingSize: activePosition.remainingSize,
        command: tpOrder.raw.command
      });

      if (activePosition.remainingSize <= 0) {
        const closingPosition = activePosition;
        const markPrice =
          closingPosition.lastKnownMarkPrice ?? closingPosition.entryMarkPrice;
        const realizedPnlUsd =
          tpOrder.realizedPnlUsd ?? computeFallbackPnlUsd(closingPosition, markPrice);
        store.withTransaction(() => {
          store.closePosition(
            closingPosition.id,
            "SIGNAL_EXIT",
            Date.now(),
            realizedPnlUsd,
            tpOrder.avgPrice,
            closingPosition.totalFeesUsd,
            closingPosition.filledExitSize
          );
          const riskState = store.applyTradeOutcome(realizedPnlUsd);
          store.appendEvent(
            "position_closed",
            "Position closed after final take-profit.",
            {
              realizedPnlUsd: Number(realizedPnlUsd.toFixed(4)),
              dailyLossUsd: Number(riskState.dailyLossUsd.toFixed(4)),
              consecutiveLosses: riskState.consecutiveLosses
            }
          );
        });
        activePosition = null;
      }
      return;
    }

    const event = detector.feed({
      ts: tick.ts,
      mid: adjustedMid
    });
    if (!event) {
      return;
    }

    if (event.kind === "ENTER" && !activePosition) {
      if (stopOpeningTrades) {
        store.appendEvent(
          "entry_blocked",
          "Entry was blocked due to risk guardrails.",
          {
            dailyLossLimitUsd: config.dailyLossLimitUsd,
            consecutiveLossLimit: config.consecutiveLossLimit
          }
        );
        return;
      }
      const entryMark = await fetchHyperliquidMarkPrice(config.perpMarket);
      const sliced = await executeSlicedEntry(config, config.entrySlices, sizeProxy);
      if (sliced.filledSize <= 0 || sliced.successfulSlices === 0) {
        store.appendEvent("live_entry_failed", "All entry slices failed.", {
          requestedSlices: config.entrySlices
        });
        return;
      }

      const stopLossMarkPrice = computeStopLossTrigger(
        entryMark.markPrice,
        config.perpSide,
        config.stopLossPct
      );
      const positionId = randomUUID();
      const openInput: PositionOpenInput = {
        id: positionId,
        runId: store.getRunId(),
        pmMarket: config.pmMarket,
        signalSide: config.signalSide,
        perpMarket: config.perpMarket,
        perpSide: config.perpSide,
        notionalUsd: config.notionalUsd,
        leverage: config.leverage,
        sizeProxy: sliced.filledSize,
        avgEntryPrice: sliced.weightedEntryPrice,
        totalFeesUsd: sliced.totalFeesUsd,
        filledEntrySize: sliced.filledSize,
        entrySignalMid: event.mid,
        entryMarkPrice: entryMark.markPrice,
        stopLossPct: config.stopLossPct,
        stopLossMarkPrice,
        remainingSize: sliced.filledSize,
        nextTakeProfitIndex: 0,
        takeProfitTargetsJson: JSON.stringify(takeProfitTargetsPct),
        openedAtMs: Date.now()
      };
      store.openPosition(openInput);

      activePosition = {
        id: positionId,
        openedAt: openInput.openedAtMs,
        entrySignalMid: event.mid,
        entryMarkPrice: entryMark.markPrice,
        direction: config.perpSide,
        sizeTotal: sliced.filledSize,
        remainingSize: sliced.filledSize,
        nextTakeProfitIndex: 0,
        takeProfitTargetsPct,
        notionalUsd: config.notionalUsd,
        leverage: config.leverage,
        avgEntryPrice: sliced.weightedEntryPrice,
        totalFeesUsd: sliced.totalFeesUsd,
        filledEntrySize: sliced.filledSize,
        filledExitSize: 0,
        lastKnownMarkPrice: entryMark.markPrice,
        stopLossManager: new StopLossManager({
          markPriceTrigger: stopLossMarkPrice,
          side: config.perpSide,
          stopLossPct: config.stopLossPct
        })
      };
      store.appendEvent("position_opened", "Position opened successfully.", {
        positionId,
        entryMarkPrice: entryMark.markPrice,
        stopLossMarkPrice,
        requestedSlices: config.entrySlices,
        successfulSlices: sliced.successfulSlices,
        failedSlices: sliced.failedSlices,
        filledSize: sliced.filledSize
      });
      log(
        `live-entry success id=${positionId} filled-size=${sliced.filledSize} slices=${sliced.successfulSlices}/${config.entrySlices} stop-loss-mark=${stopLossMarkPrice.toFixed(
          6
        )}`
      );
      return;
    }

    if (event.kind === "EXIT" && activePosition) {
      const markPrice =
        activePosition.lastKnownMarkPrice ?? activePosition.entryMarkPrice;
      const closeResult = await closeLivePosition(
        "SIGNAL_EXIT",
        config,
        store,
        activePosition,
        adjustedMid,
        markPrice
      );
      if (closeResult === "CLOSED") {
        activePosition = null;
        const riskState = store.getRiskState();
        if (riskState.dailyLossUsd < config.dailyLossLimitUsd) {
          stopOpeningTrades = false;
        }
      }
    }
  };

  cleanupFns.push(
    watcher.onTick((tick) => {
      processingQueue = processingQueue
        .then(async () => {
          await handleTick(tick);
        })
        .catch((error: unknown) => {
          const message =
            error instanceof Error ? error.message : "Unknown tick handling error";
          store.appendEvent("tick_error", message);
          log(`tick-error: ${message}`);
        });
    })
  );

  markRiskTimer = setInterval(() => {
    if (!activePosition) {
      return;
    }
    processingQueue = processingQueue
      .then(async () => {
        if (!activePosition) {
          return;
        }
        const quote = await fetchHyperliquidMarkPrice(config.perpMarket);
        activePosition.lastKnownMarkPrice = quote.markPrice;
        if (config.recordTicks) {
          store.recordTick(
            config.pmMarket,
            lastSignalMid,
            lastRawMid,
            Date.now(),
            quote.markPrice,
            "mark"
          );
        }
        if (!activePosition.stopLossManager.shouldTrigger(quote.markPrice)) {
          return;
        }
        log(
          `stop-loss-triggered mark-price=${quote.markPrice.toFixed(6)} (${activePosition.stopLossManager.describe()})`
        );
        const closeResult = await closeLivePosition(
          "STOP_LOSS",
          config,
          store,
          activePosition,
          activePosition.entrySignalMid,
          quote.markPrice
        );
        if (closeResult === "CLOSED") {
          activePosition = null;
          const riskState = store.getRiskState();
          if (
            riskState.dailyLossUsd >= config.dailyLossLimitUsd ||
            riskState.consecutiveLosses >= config.consecutiveLossLimit
          ) {
            stopOpeningTrades = true;
            log(
              `risk-gate-engaged after stop-loss daily-loss=${riskState.dailyLossUsd.toFixed(
                2
              )}/${config.dailyLossLimitUsd.toFixed(
                2
              )} consecutive-losses=${riskState.consecutiveLosses}/${
                config.consecutiveLossLimit
              }`
            );
          }
        }
      })
      .catch((error: unknown) => {
        const message =
          error instanceof Error ? error.message : "Unknown mark-risk error.";
        store.appendEvent("mark_risk_error", message);
        log(`mark-risk-error: ${message}`);
      });
  }, MARK_RISK_INTERVAL_MS);

  log(
    `live runtime started run-id=${store.getRunId()} mode=${config.leaderboardMode} market=${config.pmMarket} signal-side=${config.signalSide} entry=${config.entryThresholdPct}% exit=${config.exitThresholdPct}% db=${stateDbPath}`
  );
  store.appendEvent("runtime_start", "Live runtime started.", {
    pmMarket: config.pmMarket,
    perpMarket: config.perpMarket
  });
  watcher.start();

  await new Promise<void>((resolve) => {
    const shutdown = (): void => {
      log("shutdown signal received, stopping watcher");
      watcher.stop();
      if (markRiskTimer) {
        clearInterval(markRiskTimer);
        markRiskTimer = null;
      }
      for (const cleanup of cleanupFns) {
        cleanup();
      }
      void processingQueue.finally(() => {
        store.appendEvent("runtime_stop", "Live runtime stopped.");
        store.close();
        resolve();
      });
    };

    process.once("SIGINT", shutdown);
    process.once("SIGTERM", shutdown);
  });
}
