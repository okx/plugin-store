import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import Database from "better-sqlite3";

interface ExportArgs {
  dbPath: string;
  runId?: string;
  format: "json" | "csv";
  outputPath: string;
}

function parseArgs(argv: string[]): ExportArgs {
  const args = new Map<string, string>();
  const valueArgs = new Set(["--state-db", "--run-id", "--format", "--output"]);
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token || !token.startsWith("--")) {
      throw new Error(`Unexpected positional argument: ${token ?? "<empty>"}`);
    }
    if (!valueArgs.has(token)) {
      throw new Error(`Unsupported argument: ${token}`);
    }
    const value = argv[index + 1];
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

  const formatRaw = args.get("--format") ?? "json";
  if (formatRaw !== "json" && formatRaw !== "csv") {
    throw new Error("format must be either json or csv.");
  }

  const outputPath = args.get("--output") ?? `./proof-export.${formatRaw}`;
  return {
    dbPath,
    runId: args.get("--run-id"),
    format: formatRaw,
    outputPath
  };
}

function escapeCsvField(input: unknown): string {
  const raw =
    input === null || input === undefined
      ? ""
      : typeof input === "string"
        ? input
        : typeof input === "number" || typeof input === "boolean"
          ? String(input)
          : JSON.stringify(input);
  if (raw.includes(",") || raw.includes('"') || raw.includes("\n")) {
    return `"${raw.replace(/"/g, '""')}"`;
  }
  return raw;
}

function toCsv(rows: Array<Record<string, unknown>>): string {
  if (rows.length === 0) {
    return "";
  }
  const headers = Object.keys(rows[0] ?? {});
  const lines = [headers.join(",")];
  for (const row of rows) {
    const line = headers.map((header) => escapeCsvField(row[header])).join(",");
    lines.push(line);
  }
  return `${lines.join("\n")}\n`;
}

function resolveRunId(db: Database.Database, runId?: string): string {
  if (runId) {
    return runId;
  }
  const row = db
    .prepare("SELECT run_id FROM runs ORDER BY created_at_ms DESC LIMIT 1")
    .get() as { run_id: string } | undefined;
  if (!row?.run_id) {
    throw new Error("No run records found in the provided state database.");
  }
  return row.run_id;
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

export function runExportProofCommand(argv: string[]): Promise<void> {
  const args = parseArgs(argv);
  const db = new Database(args.dbPath, { readonly: true });
  try {
    const runId = resolveRunId(db, args.runId);
    const run = db
      .prepare(
        "SELECT run_id as runId, mode, fingerprint, config_json as configJson, label, created_at_ms as createdAtMs FROM runs WHERE run_id = @runId LIMIT 1"
      )
      .get({ runId }) as Record<string, unknown> | undefined;
    if (!run) {
      throw new Error(`Run not found: ${runId}`);
    }

    const positions = db
      .prepare(
        "SELECT * FROM positions WHERE run_id = @runId ORDER BY opened_at_ms ASC"
      )
      .all({ runId }) as Array<Record<string, unknown>>;
    const events = db
      .prepare(
        "SELECT id, run_id as runId, event_type as eventType, message, payload_json as payloadJson, previous_hash as previousHash, current_hash as currentHash, created_at_ms as createdAtMs FROM events WHERE run_id = @runId ORDER BY id ASC"
      )
      .all({ runId }) as Array<Record<string, unknown>>;
    const ticks = db
      .prepare(
        "SELECT id, run_id as runId, market_id as marketId, tick_kind as tickKind, signal_mid as signalMid, raw_mid as rawMid, ts, mark_price as markPrice FROM replay_ticks WHERE run_id = @runId ORDER BY ts ASC, id ASC"
      )
      .all({ runId }) as Array<Record<string, unknown>>;

    if (args.format === "json") {
      ensureParent(args.outputPath);
      writeFileSync(
        args.outputPath,
        JSON.stringify({ run, positions, events, ticks }, null, 2),
        "utf8"
      );
      process.stdout.write(
        `Proof export complete: ${args.outputPath} (format=json, run-id=${runId}).\n`
      );
      return Promise.resolve();
    }

    mkdirSync(args.outputPath, { recursive: true });
    writeFileSync(join(args.outputPath, "run.json"), JSON.stringify(run, null, 2), "utf8");
    writeFileSync(join(args.outputPath, "positions.csv"), toCsv(positions), "utf8");
    writeFileSync(join(args.outputPath, "events.csv"), toCsv(events), "utf8");
    writeFileSync(join(args.outputPath, "ticks.csv"), toCsv(ticks), "utf8");
    process.stdout.write(
      `Proof export complete: ${args.outputPath} (format=csv, run-id=${runId}).\n`
    );
  } finally {
    db.close();
  }
  return Promise.resolve();
}
