#!/usr/bin/env tsx
// cli.ts — dispatcher CLI for morpho-cli-onchainos.
//
// Invocation:
//   tsx cli.ts <toolName> '<jsonParams>'
//   tsx cli.ts --version
//   tsx cli.ts --help
//
// Examples:
//   tsx cli.ts --help
//   tsx cli.ts queryVaults '{"chain":"base","assetSymbol":"USDC","sort":"apy_desc","limit":5}'
//   tsx cli.ts prepareDeposit '{"chain":"base","vaultAddress":"0x...","userAddress":"0x...","amount":"1000"}'
//
// Exit codes:
//   0  — status: 'ok' OR status: 'pending_sign'
//   1  — status: 'error' (ToolError shape)
//   2  — invalid invocation (unknown tool, bad JSON)
//   3  — runtime exception (uncaught)
//
// Runtime mode is controlled via env vars in runtime.ts (MORPHO_RUNTIME=real).
// Default is MOCK so this works offline.

import * as tools from './index.ts';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

const VERSION = (() => {
  try {
    const pkg = JSON.parse(readFileSync(join(__dirname, 'package.json'), 'utf8'));
    return pkg.version ?? 'unknown';
  } catch {
    return 'unknown';
  }
})();

const TOOL_NAMES = Object.keys(tools).filter(
  (k) => typeof (tools as Record<string, unknown>)[k] === 'function',
);

function printHelp(): void {
  console.log(`morpho-cli-onchainos v${VERSION}

Usage:
  tsx cli.ts <toolName> '<jsonParams>'
  tsx cli.ts --version
  tsx cli.ts --help

Available tools (${TOOL_NAMES.length}):
${TOOL_NAMES.map((n) => '  ' + n).join('\n')}

Supported chains: ethereum, base, arbitrum, optimism, polygon, unichain, worldchain, katana, hyperevm, monad, stable
Output: pretty-printed JSON to stdout, errors to stderr.

Runtime mode (env var):
  MORPHO_RUNTIME = mock | real   (default: mock)
  MORPHO_CLI_TIMEOUT_MS = <ms>   (default: 60000)
  MORPHO_CLI_PACKAGE = <pkg>     (default: @morpho-org/cli@latest)

Live broadcast pattern (after a write tool returns pending_sign):
  onchainos wallet contract-call \\
    --chain <chain> \\
    --to <unsigned_tx.to> \\
    --input-data <unsigned_tx.data> \\
    --amt <unsigned_tx.value> \\
    --biz-type defi \\
    --strategy morpho-cli-onchainos \\
    --force
`);
}

function bigintReplacer(_key: string, value: unknown): unknown {
  return typeof value === 'bigint' ? value.toString() : value;
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);

  if (argv.length === 0 || argv[0] === '--help' || argv[0] === '-h') {
    printHelp();
    return;
  }

  if (argv[0] === '--version' || argv[0] === '-v') {
    console.log(VERSION);
    return;
  }

  const toolName = argv[0];
  if (!TOOL_NAMES.includes(toolName)) {
    console.error(`Unknown tool: ${toolName}`);
    console.error(`Available: ${TOOL_NAMES.join(', ')}`);
    console.error(`Run with --help for full usage.`);
    process.exit(2);
  }

  let params: Record<string, unknown> = {};
  if (argv[1]) {
    try {
      params = JSON.parse(argv[1]);
    } catch (e) {
      console.error(`Failed to parse params JSON: ${(e as Error).message}`);
      console.error(`Usage: tsx cli.ts ${toolName} '<jsonParams>'`);
      process.exit(2);
    }
  }

  const fn = (tools as Record<string, (p: unknown) => Promise<unknown>>)[toolName];
  try {
    const result = await fn(params);
    console.log(JSON.stringify(result, bigintReplacer, 2));

    const status = (result as { status?: string } | null | undefined)?.status;
    if (status === 'error') {
      process.exit(1);
    }
  } catch (e) {
    console.error(`Runtime error in ${toolName}:`);
    console.error((e as Error).stack ?? (e as Error).message);
    process.exit(3);
  }
}

main();
