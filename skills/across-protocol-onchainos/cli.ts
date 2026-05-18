#!/usr/bin/env tsx
// cli.ts — dispatcher CLI for across-protocol-onchainos.
//
// Form: B-REST (v1.14 taxonomy) — wraps the Across Swap API at app.across.to/api
// Tools live in index.ts; this dispatcher imports + invokes them by name.
//
// Invocation:
//   tsx cli.ts <toolName> '<jsonParams>'
//   tsx cli.ts --version
//   tsx cli.ts --help
//
// Examples:
//   tsx cli.ts --help
//   tsx cli.ts listSupportedChains '{}'
//   tsx cli.ts getQuote '{"inputToken":"0xaf88...","outputToken":"0x833...","amount":"10000000","originChainId":42161,"destinationChainId":8453}'
//   tsx cli.ts buildSwap '{"inputToken":"...","outputToken":"...","amount":"...","walletAddress":"0x..."}'
//
// Exit codes:
//   0  — status: 'ok' OR status: 'pending_sign'
//   1  — status: 'error' (ToolError shape)
//   2  — invalid invocation (unknown tool, bad JSON)
//   3  — runtime exception (uncaught)
//
// Runtime mode is controlled via env vars in runtime.ts (ACROSS_RUNTIME=real).
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
  k => typeof (tools as Record<string, unknown>)[k] === 'function'
);

function printHelp(): void {
  console.log(`across-protocol-onchainos v${VERSION}

Usage:
  tsx cli.ts <toolName> '<jsonParams>'
  tsx cli.ts --version
  tsx cli.ts --help

Available tools (${TOOL_NAMES.length}):
${TOOL_NAMES.map(n => '  ' + n).join('\n')}

Supported chains: ethereum, arbitrum, base, optimism, polygon, zksync, linea
Output: pretty-printed JSON to stdout, errors to stderr.

Runtime mode (env var):
  ACROSS_RUNTIME = mock | real   (default: mock)
  ACROSS_API_BASE = <override base>   (default: https://app.across.to/api)

Live broadcast pattern (after a write tool returns pending_sign):
  onchainos wallet contract-call \\
    --chain <chain> \\
    --to <unsigned_tx.to> \\
    --input-data <unsigned_tx.data> \\
    --amt <unsigned_tx.value> \\
    --biz-type defi \\
    --strategy across-protocol-onchainos \\
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
