import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { runVerifyProofCommand } from "../src/runtime/verify-proof.js";
import { StrategyStateStore } from "../src/state/store.js";

describe("verify-proof command", () => {
  it("reports valid proof chain in json mode", async () => {
    const tempDir = mkdtempSync(join(tmpdir(), "pm-perp-verify-test-"));
    const dbPath = join(tempDir, "state.sqlite");
    const store = new StrategyStateStore(dbPath, {
      mode: "test",
      config: { sample: true }
    });
    store.appendEvent("runtime_start", "Started");
    store.appendEvent("runtime_stop", "Stopped");
    store.close();

    let output = "";
    const originalWrite = process.stdout.write.bind(process.stdout);
    const patchedWrite: typeof process.stdout.write = (chunk: string | Uint8Array) => {
      output += chunk.toString();
      return true;
    };
    process.stdout.write = patchedWrite;

    try {
      await runVerifyProofCommand(["--state-db", dbPath, "--json"]);
    } finally {
      process.stdout.write = originalWrite;
      rmSync(tempDir, { recursive: true, force: true });
    }

    const parsed = JSON.parse(output) as {
      fingerprintValid: boolean;
      hashChainValid: boolean;
      issues: string[];
    };
    expect(parsed.fingerprintValid).toBe(true);
    expect(parsed.hashChainValid).toBe(true);
    expect(parsed.issues).toEqual([]);
  });
});
