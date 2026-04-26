import { runPlugin } from "../execution/subprocess.js";
import { ThresholdDetector } from "../signal/threshold.js";
import { StrategyStateStore } from "../state/store.js";
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

export type DryRunRuntimeConfig = RuntimeConfigWithExtras;

interface PositionState {
  openedAt: number;
  entrySignalMid: number;
  direction: "LONG" | "SHORT";
}

export function parseDryRunArgs(argv: string[]): DryRunRuntimeConfig {
  return parseRuntimeArgs(argv);
}

export async function runDryRunRuntime(
  config: DryRunRuntimeConfig
): Promise<void> {
  validateRuntimeConfig(config);
  const stateDbPath = config.stateDbPath ?? "./fixtures/dry-run.sqlite";
  const store = new StrategyStateStore(stateDbPath, {
    mode: "dry-run",
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

  let lastPrintedTickAt = 0;
  let position: PositionState | null = null;
  const sizeProxy = deriveSizeProxy(config.notionalUsd, config.leverage);

  const cleanupFns: Array<() => void> = [];

  const handleTick = async (tick: {
    ts: number;
    mid: number;
  }): Promise<void> => {
    const adjustedMid = clampProbability(
      adjustedSignalMid(tick.mid, config.signalSide)
    );
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

    const event = detector.feed({
      ts: tick.ts,
      mid: adjustedMid
    });

    if (!event) {
      return;
    }

    if (event.kind === "ENTER" && !position) {
      log(
        `signal-enter triggered at ${formatPercent(
          event.mid
        )}% -> simulated ${config.perpSide} ${config.perpMarket}-PERP`
      );

      const orderResult = await runPlugin(
        "hyperliquid-plugin",
        [
          "order",
          "--market",
          config.perpMarket,
          "--side",
          config.perpSide.toLowerCase(),
          "--size",
          String(sizeProxy),
          "--leverage",
          String(config.leverage),
          "--confirm"
        ],
        { dryRun: true }
      );

      log(`sim-order: ${orderResult.stdout}`);
      store.appendEvent("sim_entry", "Dry-run simulated entry.", {
        command: orderResult.command
      });
      log(
        `sim-risk: stop-loss=${config.stopLossPct.toFixed(
          2
        )}% notional=${config.notionalUsd.toFixed(2)}`
      );

      position = {
        openedAt: tick.ts,
        entrySignalMid: event.mid,
        direction: config.perpSide
      };
      return;
    }

    if (event.kind === "EXIT" && position) {
      log(
        `signal-exit triggered at ${formatPercent(
          event.mid
        )}% -> simulated close ${config.perpMarket}-PERP`
      );

      const closeSide = position.direction === "LONG" ? "sell" : "buy";
      const closeResult = await runPlugin(
        "hyperliquid-plugin",
        [
          "order",
          "--market",
          config.perpMarket,
          "--side",
          closeSide,
          "--size",
          String(sizeProxy),
          "--reduce-only",
          "true",
          "--confirm"
        ],
        { dryRun: true }
      );

      const heldSeconds = Math.floor((tick.ts - position.openedAt) / 1_000);
      const deltaPct = (event.mid - position.entrySignalMid) * 100;
      log(`sim-close: ${closeResult.stdout}`);
      store.appendEvent("sim_exit", "Dry-run simulated exit.", {
        command: closeResult.command
      });
      log(
        `sim-summary: held=${heldSeconds}s signal-delta=${deltaPct.toFixed(
          2
        )}pp`
      );

      position = null;
    }
  };

  cleanupFns.push(
    watcher.onError((error) => {
      log(`watcher-error: ${error.message}`);
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

  cleanupFns.push(
    watcher.onTick((tick) => {
      void handleTick(tick).catch((error: unknown) => {
        const message =
          error instanceof Error ? error.message : "Unknown tick handling error";
        log(`tick-error: ${message}`);
      });
    })
  );

  log(
    `dry-run started run-id=${store.getRunId()} market=${config.pmMarket} signal-side=${config.signalSide} entry=${config.entryThresholdPct}% exit=${config.exitThresholdPct}% db=${stateDbPath}`
  );
  store.appendEvent("runtime_start", "Dry-run runtime started.");
  watcher.start();

  await new Promise<void>((resolve) => {
    const shutdown = (): void => {
      log("shutdown signal received, stopping watcher");
      watcher.stop();
      for (const cleanup of cleanupFns) {
        cleanup();
      }
      store.appendEvent("runtime_stop", "Dry-run runtime stopped.");
      store.close();
      resolve();
    };

    process.once("SIGINT", shutdown);
    process.once("SIGTERM", shutdown);
  });
}
