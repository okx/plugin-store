#!/usr/bin/env npx tsx
//
// Discover a working RPC endpoint for a given EVM chain ID using the
// Chainlist registry. Prefers privacy-respecting endpoints (tracking=none).
//
// Usage:
//   npx tsx rpc.ts <chainId>
//   npx tsx rpc.ts <chainId> --all          # print all RPCs, not just first
//   npx tsx rpc.ts <chainId> --json         # output as JSON
//
// Examples:
//   npx tsx rpc.ts 1                        # → https://eth.llamarpc.com
//   npx tsx rpc.ts 42161 --all              # → all Arbitrum RPCs
//   npx tsx rpc.ts 137 --json               # → {"chainId":137,"rpc":"https://...","name":"Polygon"}
//
// Environment variable override:
//   RPC_URL=https://my-rpc.com npx tsx rpc.ts 1   # skips Chainlist, uses env var
//
// Also used as a library by other scripts:
//   import { getRpc, getRpcs } from "./rpc";

const CHAINLIST_URL = "https://chainlist.org/rpcs.json";
const CACHE_PATH = "/tmp/chainlist-rpcs.json";
const CACHE_TTL_MS = 24 * 60 * 60 * 1000; // 24 hours

// deBridge internal chain IDs → standard EVM chain IDs
const DEBRIDGE_ID_MAP: Record<number, number> = {
  100000001: 245022934, // Neon
  100000002: 100,       // Gnosis
  100000014: 146,       // Sonic
  100000017: 2741,      // Abstract
  100000020: 80094,     // Berachain
  100000023: 5000,      // Mantle
  100000026: 728126428, // Tron (not EVM, but mapped)
};

import { readFileSync, writeFileSync, existsSync, statSync } from "fs";

interface ChainlistEntry {
  name: string;
  chainId: number;
  nativeCurrency: { name: string; symbol: string; decimals: number };
  rpc: Array<{ url: string; tracking?: string; isOpenSource?: boolean }>;
  isTestnet?: boolean;
}

async function fetchChainlist(): Promise<ChainlistEntry[]> {
  // Check cache
  if (existsSync(CACHE_PATH)) {
    const stat = statSync(CACHE_PATH);
    if (Date.now() - stat.mtimeMs < CACHE_TTL_MS) {
      return JSON.parse(readFileSync(CACHE_PATH, "utf-8"));
    }
  }
  const resp = await fetch(CHAINLIST_URL);
  const data = (await resp.json()) as ChainlistEntry[];
  try {
    writeFileSync(CACHE_PATH, JSON.stringify(data));
  } catch {
    // Cache write failure is non-fatal
  }
  return data;
}

function resolveChainId(input: number): number {
  return DEBRIDGE_ID_MAP[input] ?? input;
}

export async function getRpcs(chainId: number): Promise<string[]> {
  const evmChainId = resolveChainId(chainId);

  // Check environment variable first
  const envRpc = process.env.RPC_URL || process.env[`RPC_URL_${evmChainId}`];
  if (envRpc) return [envRpc];

  const chains = await fetchChainlist();
  const chain = chains.find(
    (c) => c.chainId === evmChainId && !c.isTestnet
  );
  if (!chain) return [];

  return chain.rpc
    .filter(
      (r) =>
        r.tracking === "none" || r.tracking === "limited" || !r.tracking
    )
    .filter((r) => !r.url.includes("${") && !r.url.includes("{"))
    .sort((a, b) => {
      // Prefer tracking=none
      const score = (t?: string) =>
        t === "none" ? 0 : t === "limited" ? 1 : 2;
      return score(a.tracking) - score(b.tracking);
    })
    .map((r) => r.url);
}

export async function getRpc(chainId: number): Promise<string> {
  const rpcs = await getRpcs(chainId);
  if (rpcs.length === 0) {
    throw new Error(`No RPC found for chain ${chainId}`);
  }

  // Health-check the first few candidates
  for (const url of rpcs.slice(0, 5)) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 3000);
      const resp = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          method: "eth_blockNumber",
          params: [],
          id: 1,
        }),
        signal: controller.signal,
      });
      clearTimeout(timeout);
      const data = (await resp.json()) as any;
      if (data.result) return url;
    } catch {
      continue;
    }
  }

  // Fallback: return first without health check
  return rpcs[0];
}

export async function getChainInfo(
  chainId: number
): Promise<{ name: string; nativeCurrency: { symbol: string; decimals: number } } | null> {
  const evmChainId = resolveChainId(chainId);
  const chains = await fetchChainlist();
  const chain = chains.find(
    (c) => c.chainId === evmChainId && !c.isTestnet
  );
  if (!chain) return null;
  return {
    name: chain.name,
    nativeCurrency: {
      symbol: chain.nativeCurrency.symbol,
      decimals: chain.nativeCurrency.decimals,
    },
  };
}

// CLI entrypoint
if (process.argv[1]?.endsWith("rpc.ts") || process.argv[1]?.includes("rpc")) {
  const args = process.argv.slice(2);
  const chainId = parseInt(args.find((a) => !a.startsWith("-")) || "", 10);
  const showAll = args.includes("--all");
  const asJson = args.includes("--json");

  if (!chainId) {
    console.error("Usage: npx tsx rpc.ts <chainId> [--all] [--json]");
    process.exit(1);
  }

  (async () => {
    if (showAll) {
      const rpcs = await getRpcs(chainId);
      if (asJson) {
        console.log(JSON.stringify({ chainId, rpcs }));
      } else {
        rpcs.forEach((r) => console.log(r));
      }
    } else {
      const rpc = await getRpc(chainId);
      const info = await getChainInfo(chainId);
      if (asJson) {
        console.log(JSON.stringify({ chainId, rpc, name: info?.name ?? "unknown" }));
      } else {
        console.log(rpc);
      }
    }
  })().catch((err) => {
    console.error(err.message);
    process.exit(1);
  });
}
