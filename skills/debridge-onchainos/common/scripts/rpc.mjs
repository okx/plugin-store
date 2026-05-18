#!/usr/bin/env node
//
// Discover a working RPC endpoint for a given chain ID using the
// Chainlist registry. Prefers privacy-respecting endpoints (tracking=none).
// Supports both EVM chains (via Chainlist) and Solana (via env var / public RPC).
//
// Usage:
//   node rpc.mjs <chainId>
//   node rpc.mjs <chainId> --all          # print all RPCs, not just first
//   node rpc.mjs <chainId> --json         # output as JSON
//
// Examples:
//   node rpc.mjs 1                        # → https://eth.llamarpc.com
//   node rpc.mjs 42161 --all              # → all Arbitrum RPCs
//   node rpc.mjs 137 --json               # → {"chainId":137,"rpc":"https://...","name":"Polygon"}
//   node rpc.mjs 7565164                  # → Solana RPC
//
// Environment variable override:
//   RPC_URL=https://my-rpc.com node rpc.mjs 1     # skips Chainlist, uses env var
//   SOLANA_RPC_URL=https://... node rpc.mjs 7565164
//
// Also used as a library by other .mjs scripts:
//   import { getRpc, getRpcs, getChainInfo } from "./rpc.mjs";

import { readFileSync, writeFileSync, existsSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";

const CHAINLIST_URL = "https://chainlist.org/rpcs.json";
const CACHE_PATH = "/tmp/chainlist-rpcs.json";
const CACHE_TTL_MS = 24 * 60 * 60 * 1000; // 24 hours

// deBridge chain ID for Solana
const SOLANA_CHAIN_ID = 7565164;
const SOLANA_DEFAULT_RPC = "https://api.mainnet-beta.solana.com";

// deBridge internal chain IDs → standard EVM chain IDs
const DEBRIDGE_ID_MAP = {
  100000001: 245022934, // Neon
  100000002: 100,       // Gnosis
  100000014: 146,       // Sonic
  100000017: 2741,      // Abstract
  100000020: 80094,     // Berachain
  100000023: 5000,      // Mantle
  100000026: 728126428, // Tron (not EVM, but mapped)
};

async function fetchChainlist() {
  // Check cache
  if (existsSync(CACHE_PATH)) {
    const stat = statSync(CACHE_PATH);
    if (Date.now() - stat.mtimeMs < CACHE_TTL_MS) {
      return JSON.parse(readFileSync(CACHE_PATH, "utf-8"));
    }
  }
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10000);
  let resp;
  try {
    resp = await fetch(CHAINLIST_URL, { signal: controller.signal });
  } catch (err) {
    clearTimeout(timeout);
    if (err.name === "AbortError") throw new Error("Chainlist fetch timed out after 10 s");
    throw err;
  }
  clearTimeout(timeout);
  const data = await resp.json();
  try {
    writeFileSync(CACHE_PATH, JSON.stringify(data));
  } catch {
    // Cache write failure is non-fatal
  }
  return data;
}

function resolveChainId(input) {
  return DEBRIDGE_ID_MAP[input] ?? input;
}

/**
 * Return an ordered list of RPC URLs for the given chain ID.
 * Checks env vars first, then Chainlist. For Solana, returns
 * SOLANA_RPC_URL or the public mainnet-beta endpoint.
 * @param {number} chainId - deBridge or standard chain ID
 * @returns {Promise<string[]>}
 */
export async function getRpcs(chainId) {
  const evmChainId = resolveChainId(chainId);

  // Solana is not on Chainlist — handle separately
  if (chainId === SOLANA_CHAIN_ID || evmChainId === SOLANA_CHAIN_ID) {
    const envRpc =
      process.env.SOLANA_RPC_URL ||
      process.env.RPC_URL ||
      process.env[`RPC_URL_${SOLANA_CHAIN_ID}`];
    return [envRpc || SOLANA_DEFAULT_RPC];
  }

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
      const score = (t) =>
        t === "none" ? 0 : t === "limited" ? 1 : 2;
      return score(a.tracking) - score(b.tracking);
    })
    .map((r) => r.url);
}

/**
 * Return a single working RPC URL for the given chain ID.
 * Health-checks up to 5 candidates with a 3-second timeout each.
 * @param {number} chainId - deBridge or standard chain ID
 * @returns {Promise<string>}
 */
export async function getRpc(chainId) {
  const rpcs = await getRpcs(chainId);
  if (rpcs.length === 0) {
    throw new Error(`No RPC found for chain ${chainId}`);
  }

  const evmChainId = resolveChainId(chainId);

  // Skip health check for Solana — it uses a different JSON-RPC method
  if (chainId === SOLANA_CHAIN_ID || evmChainId === SOLANA_CHAIN_ID) {
    return rpcs[0];
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
      const data = await resp.json();
      if (data.result) return url;
    } catch {
      continue;
    }
  }

  // Fallback: return first without health check
  return rpcs[0];
}

/**
 * Return chain metadata (name and native currency) for the given chain ID.
 * For Solana, returns hardcoded metadata since it is not on Chainlist.
 * @param {number} chainId - deBridge or standard chain ID
 * @returns {Promise<{name: string, nativeCurrency: {symbol: string, decimals: number}} | null>}
 */
export async function getChainInfo(chainId) {
  const evmChainId = resolveChainId(chainId);

  // Solana is not on Chainlist — return hardcoded info
  if (chainId === SOLANA_CHAIN_ID || evmChainId === SOLANA_CHAIN_ID) {
    return {
      name: "Solana",
      nativeCurrency: { symbol: "SOL", decimals: 9 },
    };
  }

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

// CLI entrypoint — runs only when executed directly
const __filename = fileURLToPath(import.meta.url);
if (process.argv[1] === __filename) {
  const args = process.argv.slice(2);
  const chainId = parseInt(args.find((a) => !a.startsWith("-")) || "", 10);
  const showAll = args.includes("--all");
  const asJson = args.includes("--json");

  if (!chainId) {
    console.error("Usage: node rpc.mjs <chainId> [--all] [--json]");
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
        console.log(
          JSON.stringify({ chainId, rpc, name: info?.name ?? "unknown" })
        );
      } else {
        console.log(rpc);
      }
    }
  })().catch((err) => {
    console.error(err.message);
    process.exit(1);
  });
}
