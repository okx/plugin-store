import { spawn } from "node:child_process";

import type { SupportedPlugin } from "./strategy-id.js";
import { withStrategyId } from "./strategy-id.js";

export interface RunPluginOptions {
  dryRun?: boolean;
  timeoutMs?: number;
  forceStrategyId?: boolean;
}

export interface PluginResult {
  ok: boolean;
  command: string;
  exitCode: number | null;
  stdout: string;
  stderr: string;
  parsedJson?: unknown;
}

const DEFAULT_TIMEOUT_MS = 30_000;

function safeParseJson(payload: string): unknown {
  const trimmed = payload.trim();
  if (!trimmed) {
    return undefined;
  }

  try {
    return JSON.parse(trimmed) as unknown;
  } catch {
    return undefined;
  }
}

export async function runPlugin(
  plugin: SupportedPlugin,
  args: string[],
  options: RunPluginOptions = {}
): Promise<PluginResult> {
  const finalArgs = withStrategyId(args, options.forceStrategyId ?? false);
  const command = `${plugin} ${finalArgs.join(" ")}`;

  if (options.dryRun) {
    return {
      ok: true,
      command,
      exitCode: 0,
      stdout: `[DRY-RUN] ${command}`,
      stderr: ""
    };
  }

  return await new Promise<PluginResult>((resolve) => {
    const child = spawn(plugin, finalArgs, {
      stdio: ["ignore", "pipe", "pipe"]
    });

    let stdout = "";
    let stderr = "";
    let timedOut = false;

    const timeout = setTimeout(() => {
      timedOut = true;
      child.kill("SIGKILL");
    }, options.timeoutMs ?? DEFAULT_TIMEOUT_MS);

    child.stdout.on("data", (chunk: Buffer | string) => {
      stdout += chunk.toString();
    });

    child.stderr.on("data", (chunk: Buffer | string) => {
      stderr += chunk.toString();
    });

    child.on("close", (exitCode) => {
      clearTimeout(timeout);
      const parsedJson = safeParseJson(stdout);
      resolve({
        ok: !timedOut && exitCode === 0,
        command,
        exitCode,
        stdout,
        stderr: timedOut ? `${stderr}\nProcess timed out.`.trim() : stderr,
        parsedJson
      });
    });

    child.on("error", (error: Error) => {
      clearTimeout(timeout);
      resolve({
        ok: false,
        command,
        exitCode: null,
        stdout,
        stderr: `${stderr}\n${error.message}`.trim()
      });
    });
  });
}
