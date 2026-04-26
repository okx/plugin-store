export interface Tick {
  ts: number;
  mid: number;
}

export interface ThresholdConfig {
  entryThreshold: number;
  exitThreshold: number;
  dwellMs: number;
}

export type SignalEvent =
  | { kind: "ENTER"; ts: number; mid: number }
  | { kind: "EXIT"; ts: number; mid: number };

export class ThresholdDetector {
  private aboveSince: number | null = null;
  private inPosition = false;

  public constructor(private readonly config: ThresholdConfig) {
    this.validateConfig(config);
  }

  public feed(tick: Tick): SignalEvent | null {
    if (!this.inPosition) {
      return this.maybeEnter(tick);
    }
    return this.maybeExit(tick);
  }

  public isInPosition(): boolean {
    return this.inPosition;
  }

  public forceInPosition(): void {
    this.inPosition = true;
    this.aboveSince = null;
  }

  private validateConfig(config: ThresholdConfig): void {
    const fields: Array<[string, number]> = [
      ["entryThreshold", config.entryThreshold],
      ["exitThreshold", config.exitThreshold]
    ];

    for (const [name, value] of fields) {
      if (value < 0 || value > 1) {
        throw new Error(`${name} must be between 0 and 1.`);
      }
    }

    if (config.exitThreshold >= config.entryThreshold) {
      throw new Error("exitThreshold must be lower than entryThreshold.");
    }

    if (!Number.isFinite(config.dwellMs) || config.dwellMs < 0) {
      throw new Error("dwellMs must be a non-negative finite number.");
    }
  }

  private maybeEnter(tick: Tick): SignalEvent | null {
    if (tick.mid < this.config.entryThreshold) {
      this.aboveSince = null;
      return null;
    }

    this.aboveSince ??= tick.ts;

    if (tick.ts - this.aboveSince < this.config.dwellMs) {
      return null;
    }

    this.inPosition = true;
    this.aboveSince = null;

    return {
      kind: "ENTER",
      ts: tick.ts,
      mid: tick.mid
    };
  }

  private maybeExit(tick: Tick): SignalEvent | null {
    if (tick.mid > this.config.exitThreshold) {
      return null;
    }

    this.inPosition = false;

    return {
      kind: "EXIT",
      ts: tick.ts,
      mid: tick.mid
    };
  }
}
