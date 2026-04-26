import type { StoredPosition } from "../state/store.js";

export interface StopLossSpec {
  markPriceTrigger: number;
  side: "LONG" | "SHORT";
  stopLossPct: number;
}

export function computeStopLossTrigger(
  entryMarkPrice: number,
  side: "LONG" | "SHORT",
  stopLossPct: number
): number {
  const ratio = stopLossPct / 100;
  if (side === "LONG") {
    return entryMarkPrice * (1 - ratio);
  }
  return entryMarkPrice * (1 + ratio);
}

export class StopLossManager {
  private readonly spec: StopLossSpec;

  public constructor(spec: StopLossSpec) {
    this.spec = spec;
  }

  public static fromStoredPosition(position: StoredPosition): StopLossManager {
    return new StopLossManager({
      markPriceTrigger: position.stopLossMarkPrice,
      side: position.perpSide,
      stopLossPct: position.stopLossPct
    });
  }

  public shouldTrigger(markPrice: number): boolean {
    if (this.spec.side === "LONG") {
      return markPrice <= this.spec.markPriceTrigger;
    }
    return markPrice >= this.spec.markPriceTrigger;
  }

  public describe(): string {
    const trigger = this.spec.markPriceTrigger.toFixed(6);
    return `side=${this.spec.side} stop-loss=${this.spec.stopLossPct.toFixed(
      2
    )}% trigger-mark-price=${trigger}`;
  }

  public getTrigger(): number {
    return this.spec.markPriceTrigger;
  }
}
