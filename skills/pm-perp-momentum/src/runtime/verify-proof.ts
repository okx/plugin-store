import { createHash } from "node:crypto";

import Database from "better-sqlite3";

interface VerifyArgs {
  dbPath: string;
  runId?: string;
  json: boolean;
}

interface EventRow {
  id: number;
  runId: string;
  eventType: string;
  message: string;
  payloadJson: string | null;
  previousHash: string | null;
  currentHash: string;
  createdAtMs: number;
}

function parseArgs(argv: string[]): VerifyArgs {
  const args = new Map<string, string>();
  const flags = new Set<string>();
  const valueArgs = new Set(["--state-db", "--run-id"]);
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
  return {
    dbPath,
    runId: args.get("--run-id"),
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
    throw new Error("No run records found in the provided state database.");
  }
  return row.run_id;
}

function hashEvent(previousHash: string, runId: string, row: EventRow): string {
  return createHash("sha256")
    .update(
      [
        previousHash,
        runId,
        row.eventType,
        row.message,
        row.payloadJson ?? "",
        String(row.createdAtMs)
      ].join("|")
    )
    .digest("hex");
}

export function runVerifyProofCommand(argv: string[]): Promise<void> {
  const args = parseArgs(argv);
  const db = new Database(args.dbPath, { readonly: true });
  try {
    const runId = resolveRunId(db, args.runId);
    const run = db
      .prepare(
        "SELECT run_id as runId, mode, fingerprint, config_json as configJson, created_at_ms as createdAtMs FROM runs WHERE run_id = @runId LIMIT 1"
      )
      .get({ runId }) as
      | { runId: string; mode: string; fingerprint: string; configJson: string; createdAtMs: number }
      | undefined;
    if (!run) {
      throw new Error(`Run not found: ${runId}`);
    }

    const expectedFingerprint = createHash("sha256")
      .update(`${run.mode}:${run.configJson}`)
      .digest("hex");
    const events = db
      .prepare(
        "SELECT id, run_id as runId, event_type as eventType, message, payload_json as payloadJson, previous_hash as previousHash, current_hash as currentHash, created_at_ms as createdAtMs FROM events WHERE run_id = @runId ORDER BY id ASC"
      )
      .all({ runId }) as EventRow[];

    const issues: string[] = [];
    let previousHash = "GENESIS";
    for (const row of events) {
      if ((row.previousHash ?? "GENESIS") !== previousHash) {
        issues.push(
          `Event #${row.id} previous hash mismatch (expected=${previousHash}, actual=${row.previousHash ?? "null"})`
        );
      }
      const recalculated = hashEvent(previousHash, runId, row);
      if (recalculated !== row.currentHash) {
        issues.push(
          `Event #${row.id} current hash mismatch (expected=${recalculated}, actual=${row.currentHash})`
        );
      }
      previousHash = row.currentHash;
    }

    if (run.fingerprint !== expectedFingerprint) {
      issues.push(
        `Run fingerprint mismatch (expected=${expectedFingerprint}, actual=${run.fingerprint})`
      );
    }

    const result = {
      runId,
      eventCount: events.length,
      fingerprintValid: run.fingerprint === expectedFingerprint,
      hashChainValid: issues.length === 0,
      issues
    };

    if (args.json) {
      process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
      return Promise.resolve();
    }

    if (issues.length === 0) {
      process.stdout.write(
        `Proof verification passed: run-id=${runId} events=${events.length} fingerprint=valid hash-chain=valid\n`
      );
      return Promise.resolve();
    }

    process.stdout.write(
      `Proof verification failed: run-id=${runId} events=${events.length} issues=${issues.length}\n`
    );
    for (const issue of issues) {
      process.stdout.write(`- ${issue}\n`);
    }
  } finally {
    db.close();
  }
  return Promise.resolve();
}
