#!/usr/bin/env tsx
// cli.ts — dispatcher CLI for camelot-v3-onchainos.

import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

const VERSION = (() => {
  try {
    return JSON.parse(readFileSync(join(__dirname, 'package.json'), 'utf8')).version ?? 'unknown';
  } catch { return 'unknown'; }
})();

const HAS_INDEX = existsSync(join(__dirname, 'index.ts'));

async function loadTools(): Promise<Record<string, unknown>> {
  if (!HAS_INDEX) return {};
  try { return await import('./index.ts'); }
  catch (e) { console.error(`Warning: failed to import index.ts: ${(e as Error).message}`); return {}; }
}

function printHelp(toolNames: string[]): void {
  console.log(`camelot-v3-onchainos v${VERSION}

Usage:
  tsx cli.ts <toolName> '<jsonParams>'
  tsx cli.ts --version
  tsx cli.ts --help

${HAS_INDEX
  ? `Available tools (${toolNames.length}):
${toolNames.map(n => '  ' + n).join('\n')}`
  : `⚠️  No index.ts found.`}

Supported chains: arbitrum
Output: pretty-printed JSON to stdout, errors to stderr.

Live broadcast pattern (after buildSwap returns pending_sign):
  onchainos wallet contract-call \\
    --chain arbitrum \\
    --to <unsigned_tx.to> \\
    --input-data <unsigned_tx.data> \\
    --amt <unsigned_tx.value> \\
    --biz-type defi \\
    --strategy camelot-v3-onchainos \\
    --force
`);
}

function bigintReplacer(_k: string, v: unknown): unknown {
  return typeof v === 'bigint' ? v.toString() : v;
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const tools = await loadTools();
  const TOOL_NAMES = Object.keys(tools).filter(
    k => typeof (tools as Record<string, unknown>)[k] === 'function',
  );

  if (argv.length === 0 || argv[0] === '--help' || argv[0] === '-h') { printHelp(TOOL_NAMES); return; }
  if (argv[0] === '--version' || argv[0] === '-v') { console.log(VERSION); return; }

  const toolName = argv[0];
  if (!TOOL_NAMES.includes(toolName)) {
    console.error(`Unknown tool: ${toolName}`);
    console.error(`Available: ${TOOL_NAMES.join(', ')}`);
    process.exit(2);
  }

  let params: Record<string, unknown> = {};
  if (argv[1]) {
    try { params = JSON.parse(argv[1]); }
    catch (e) { console.error(`Failed to parse params JSON: ${(e as Error).message}`); process.exit(2); }
  }

  const fn = (tools as Record<string, (p: unknown) => Promise<unknown>>)[toolName];
  try {
    const result = await fn(params);
    console.log(JSON.stringify(result, bigintReplacer, 2));
    const status = (result as { status?: string } | null | undefined)?.status;
    if (status === 'error') process.exit(1);
  } catch (e) {
    console.error(`Runtime error in ${toolName}:`);
    console.error((e as Error).stack ?? (e as Error).message);
    process.exit(3);
  }
}

main();
