export interface RuntimeConfig {
  pmMarket: string;
  signalSide: "YES" | "NO";
  entryThresholdPct: number;
  exitThresholdPct: number;
  dwellSeconds: number;
  perpMarket: string;
  perpSide: "LONG" | "SHORT";
  notionalUsd: number;
  leverage: number;
  stopLossPct: number;
  leaderboardMode: "volume-max" | "tx-max" | "address-max";
  entrySlices: number;
  takeProfitTargetsPct: number[];
  dailyLossLimitUsd: number;
  consecutiveLossLimit: number;
  recordTicks: boolean;
  resumeOpenPosition: boolean;
}

export interface RuntimeExtras {
  stateDbPath?: string;
  confirmLive: boolean;
  riskAcknowledgement?: string;
  replayRunId?: string;
  replaySpeed: number;
  runLabel?: string;
}

export type RuntimeConfigWithExtras = RuntimeConfig & RuntimeExtras;

function parseNumericArg(raw: string, name: string): number {
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`Invalid numeric value for ${name}: ${raw}`);
  }
  return value;
}

function parseIntegerArg(raw: string, name: string): number {
  const value = Number(raw);
  if (!Number.isInteger(value)) {
    throw new Error(`Invalid integer value for ${name}: ${raw}`);
  }
  return value;
}

function parseTakeProfitTargets(raw: string): number[] {
  const parts = raw
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);

  if (parts.length === 0) {
    throw new Error(
      "take-profit-targets must contain at least one numeric percentage."
    );
  }

  const values = parts.map((part) => parseNumericArg(part, "--take-profit-targets"));
  values.sort((a, b) => a - b);
  return values;
}

function readRequired(args: Map<string, string>, key: string): string {
  const value = args.get(key);
  if (!value) {
    throw new Error(`Missing required argument: ${key}`);
  }
  return value;
}

function parseArgMaps(argv: string[]): { args: Map<string, string>; flags: Set<string> } {
  const args = new Map<string, string>();
  const flags = new Set<string>();

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token || !token.startsWith("--")) {
      continue;
    }

    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      flags.add(token);
      continue;
    }

    args.set(token, value);
    index += 1;
  }

  return { args, flags };
}

function assertNoUnknownArgs(
  argv: string[],
  valueArgs: ReadonlySet<string>,
  flagArgs: ReadonlySet<string>
): void {
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token || !token.startsWith("--")) {
      throw new Error(`Unexpected positional argument: ${token ?? "<empty>"}`);
    }
    if (valueArgs.has(token)) {
      const value = argv[index + 1];
      if (!value || value.startsWith("--")) {
        throw new Error(`Missing value for argument: ${token}`);
      }
      index += 1;
      continue;
    }
    if (flagArgs.has(token)) {
      continue;
    }
    throw new Error(`Unsupported argument: ${token}`);
  }
}

function applyLeaderboardPreset(
  mode: "volume-max" | "tx-max" | "address-max"
): {
  entrySlices: number;
  takeProfitTargetsPct: number[];
  dailyLossLimitUsd: number;
  consecutiveLossLimit: number;
  dwellSecondsFloor: number;
} {
  if (mode === "volume-max") {
    return {
      entrySlices: 3,
      takeProfitTargetsPct: [1.25, 2.5, 4],
      dailyLossLimitUsd: 1500,
      consecutiveLossLimit: 5,
      dwellSecondsFloor: 10
    };
  }
  if (mode === "tx-max") {
    return {
      entrySlices: 8,
      takeProfitTargetsPct: [0.4, 0.8, 1.2, 1.6],
      dailyLossLimitUsd: 900,
      consecutiveLossLimit: 4,
      dwellSecondsFloor: 5
    };
  }
  return {
    entrySlices: 2,
    takeProfitTargetsPct: [0.8, 1.6, 2.4],
    dailyLossLimitUsd: 500,
    consecutiveLossLimit: 3,
    dwellSecondsFloor: 20
  };
}

export function parseRuntimeArgs(argv: string[]): RuntimeConfigWithExtras {
  assertNoUnknownArgs(
    argv,
    new Set([
      "--pm-market",
      "--signal-side",
      "--entry-threshold",
      "--exit-threshold",
      "--dwell-seconds",
      "--perp",
      "--side",
      "--notional-usd",
      "--leverage",
      "--stop-loss-pct",
      "--leaderboard-mode",
      "--entry-slices",
      "--take-profit-targets",
      "--daily-loss-limit-usd",
      "--consecutive-loss-limit",
      "--state-db",
      "--risk-ack",
      "--run-id",
      "--replay-speed",
      "--run-label"
    ]),
    new Set(["--no-record-ticks", "--confirm-live", "--resume-open-position"])
  );
  const { args, flags } = parseArgMaps(argv);

  const signalSide = readRequired(args, "--signal-side");
  if (signalSide !== "YES" && signalSide !== "NO") {
    throw new Error("signal-side must be YES or NO.");
  }

  const perpSide = readRequired(args, "--side");
  if (perpSide !== "LONG" && perpSide !== "SHORT") {
    throw new Error("side must be LONG or SHORT.");
  }

  const leaderboardModeRaw = args.get("--leaderboard-mode") ?? "volume-max";
  if (
    leaderboardModeRaw !== "volume-max" &&
    leaderboardModeRaw !== "tx-max" &&
    leaderboardModeRaw !== "address-max"
  ) {
    throw new Error(
      "leaderboard-mode must be one of: volume-max, tx-max, address-max."
    );
  }
  const preset = applyLeaderboardPreset(leaderboardModeRaw);

  const entrySlices = args.has("--entry-slices")
    ? parseIntegerArg(readRequired(args, "--entry-slices"), "--entry-slices")
    : preset.entrySlices;
  const takeProfitTargetsPct = args.has("--take-profit-targets")
    ? parseTakeProfitTargets(readRequired(args, "--take-profit-targets"))
    : preset.takeProfitTargetsPct;
  const dailyLossLimitUsd = args.has("--daily-loss-limit-usd")
    ? parseNumericArg(
        readRequired(args, "--daily-loss-limit-usd"),
        "--daily-loss-limit-usd"
      )
    : preset.dailyLossLimitUsd;
  const consecutiveLossLimit = args.has("--consecutive-loss-limit")
    ? parseIntegerArg(
        readRequired(args, "--consecutive-loss-limit"),
        "--consecutive-loss-limit"
      )
    : preset.consecutiveLossLimit;
  const dwellSecondsParsed = parseNumericArg(
    readRequired(args, "--dwell-seconds"),
    "--dwell-seconds"
  );

  return {
    pmMarket: readRequired(args, "--pm-market"),
    signalSide,
    entryThresholdPct: parseNumericArg(
      readRequired(args, "--entry-threshold"),
      "--entry-threshold"
    ),
    exitThresholdPct: parseNumericArg(
      readRequired(args, "--exit-threshold"),
      "--exit-threshold"
    ),
    dwellSeconds: Math.max(dwellSecondsParsed, preset.dwellSecondsFloor),
    perpMarket: readRequired(args, "--perp"),
    perpSide,
    notionalUsd: parseNumericArg(
      readRequired(args, "--notional-usd"),
      "--notional-usd"
    ),
    leverage: parseNumericArg(readRequired(args, "--leverage"), "--leverage"),
    stopLossPct: parseNumericArg(
      readRequired(args, "--stop-loss-pct"),
      "--stop-loss-pct"
    ),
    leaderboardMode: leaderboardModeRaw,
    entrySlices,
    takeProfitTargetsPct,
    dailyLossLimitUsd,
    consecutiveLossLimit,
    recordTicks: !flags.has("--no-record-ticks"),
    resumeOpenPosition: flags.has("--resume-open-position"),
    stateDbPath: args.get("--state-db"),
    confirmLive: flags.has("--confirm-live"),
    riskAcknowledgement: args.get("--risk-ack"),
    replayRunId: args.get("--run-id"),
    replaySpeed: args.has("--replay-speed")
      ? parseNumericArg(readRequired(args, "--replay-speed"), "--replay-speed")
      : 1,
    runLabel: args.get("--run-label")
  };
}

export function validateRuntimeConfig(config: RuntimeConfig): void {
  if (config.entryThresholdPct <= config.exitThresholdPct) {
    throw new Error(
      "entry-threshold must be greater than exit-threshold for hysteresis."
    );
  }
  if (config.entryThresholdPct < 0 || config.entryThresholdPct > 100) {
    throw new Error("entry-threshold must be between 0 and 100.");
  }
  if (config.exitThresholdPct < 0 || config.exitThresholdPct > 100) {
    throw new Error("exit-threshold must be between 0 and 100.");
  }
  if (config.dwellSeconds < 0) {
    throw new Error("dwell-seconds must be non-negative.");
  }
  if (config.leverage <= 0) {
    throw new Error("leverage must be greater than 0.");
  }
  if (config.notionalUsd <= 0) {
    throw new Error("notional-usd must be greater than 0.");
  }
  if (config.stopLossPct <= 0 || config.stopLossPct >= 50) {
    throw new Error("stop-loss-pct must be between 0 and 50.");
  }
  if (!Number.isInteger(config.entrySlices) || config.entrySlices <= 0) {
    throw new Error("entry-slices must be a positive integer.");
  }
  if (config.takeProfitTargetsPct.length === 0) {
    throw new Error("take-profit-targets must not be empty.");
  }
  for (const target of config.takeProfitTargetsPct) {
    if (target <= 0 || target > 100) {
      throw new Error("take-profit-target values must be between 0 and 100.");
    }
  }
  if (config.dailyLossLimitUsd <= 0) {
    throw new Error("daily-loss-limit-usd must be greater than 0.");
  }
  if (!Number.isInteger(config.consecutiveLossLimit) || config.consecutiveLossLimit <= 0) {
    throw new Error("consecutive-loss-limit must be a positive integer.");
  }
}

export function nowIso(): string {
  return new Date().toISOString();
}

export function log(message: string): void {
  process.stdout.write(`[${nowIso()}] ${message}\n`);
}

export function pctToProb(value: number): number {
  return value / 100;
}

export function clampProbability(input: number): number {
  return Math.max(0, Math.min(1, input));
}

export function adjustedSignalMid(mid: number, signalSide: "YES" | "NO"): number {
  return signalSide === "YES" ? mid : 1 - mid;
}

export function formatPercent(input: number): string {
  return (input * 100).toFixed(2);
}

export function deriveSizeProxy(notionalUsd: number, leverage: number): number {
  const margin = notionalUsd / leverage;
  return Number(margin.toFixed(6));
}
