export const STRATEGY_ID = "pm-perp-momentum" as const;

export type SupportedPlugin = "hyperliquid-plugin" | "polymarket-plugin";

const WRITE_OPERATIONS = new Set(["buy", "sell", "swap", "order"]);

export function isWriteOperation(args: string[]): boolean {
  const operation = args[0]?.trim().toLowerCase();
  return operation ? WRITE_OPERATIONS.has(operation) : false;
}

export function withStrategyId(args: string[], force = false): string[] {
  if (args.includes("--strategy-id")) {
    throw new Error(
      "Do not pass --strategy-id directly. The execution wrapper injects it automatically."
    );
  }

  if (!force && !isWriteOperation(args)) {
    return [...args];
  }

  return [...args, "--strategy-id", STRATEGY_ID];
}
