#!/usr/bin/env node
//
// EVM native balance checker
//
// Queries native token balance on one or more EVM chains. RPCs are discovered
// dynamically via Chainlist (../../common/scripts/rpc.mjs) — no hardcoded URLs.
//
// Usage:
//   node balance-evm.mjs <0x_address> [--chains <ids>] [--all] [--json]
//
// Arguments:
//   0x_address  A standard EVM address (0x-prefixed, 42 characters).
//               Do NOT pass OWS wallet names or ENS names — resolve
//               the address first via `ows wallet list` or other means.
//
// Options:
//   --chains <ids>  Comma-separated chain IDs (e.g., 1,137,42161)
//   --all           Query all known deBridge EVM chains
//   --token <addr>  Query ERC-20 token balance instead of native balance
//   --json          Output as JSON array
//
// One of --chains or --all is required.
//
// Examples:
//   node balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --chains 1,137,42161
//   node balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --all
//   node balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --chains 137 --json
//   node balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --chains 137 --token 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359

import { getRpc, getChainInfo } from "../../common/scripts/rpc.mjs";

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);

if (args.length === 0 || args.includes("--help") || args.includes("-h")) {
  console.log("Usage: node balance-evm.mjs <0x_address> [--chains <ids>] [--all] [--json]");
  process.exit(0);
}

// All known deBridge EVM chain IDs (used by --all)
const DEBRIDGE_EVM_CHAINS = [
  1,          // Ethereum
  56,         // BNB Chain
  137,        // Polygon
  42161,      // Arbitrum One
  10,         // Optimism
  43114,      // Avalanche C-Chain
  8453,       // Base
  59144,      // Linea
  100000014,  // Sonic
  100000020,  // Berachain
  100000023,  // Mantle
  100000017,  // Abstract
];

const flags = { json: false, chains: null, all: false, token: null };
const positional = [];

for (let i = 0; i < args.length; i++) {
  if (args[i] === "--json") flags.json = true;
  else if (args[i] === "--all") flags.all = true;
  else if (args[i] === "--chains" && args[i + 1]) flags.chains = args[++i].split(",").map(Number);
  else if (args[i] === "--token" && args[i + 1]) flags.token = args[++i];
  else if (!args[i].startsWith("--")) positional.push(args[i]);
  else { console.error(`Unknown flag: ${args[i]}`); process.exit(1); }
}

if (positional.length < 1) {
  console.error("Error: address or wallet name required");
  process.exit(1);
}
if (flags.all) {
  flags.chains = DEBRIDGE_EVM_CHAINS;
} else if (!flags.chains || flags.chains.length === 0) {
  console.error("Error: --chains or --all is required (e.g., --chains 1,137,42161 or --all)");
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Validate EVM address
// ---------------------------------------------------------------------------
// The caller must resolve wallet names to standard 0x addresses before
// invoking this script (e.g., via `ows wallet list` → extract eip155: address).

const address = positional[0];

if (!/^0x[0-9a-fA-F]{40}$/.test(address)) {
  console.error(`Error: "${address}" is not a valid EVM address.`);
  console.error("This script accepts only standard 0x-prefixed addresses (42 characters).");
  console.error("Resolve wallet names first: run `ows wallet list` and extract the eip155: address.");
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Balance query
// ---------------------------------------------------------------------------
function formatBalance(hexValue, decimals) {
  const raw = BigInt(hexValue);
  if (raw === 0n) return "0";
  const divisor = 10n ** BigInt(decimals);
  const whole = raw / divisor;
  const remainder = raw % divisor;
  if (remainder === 0n) return whole.toString();
  const fracStr = remainder.toString().padStart(decimals, "0").replace(/0+$/, "");
  return `${whole}.${fracStr}`;
}

// ERC-20 balanceOf(address) selector: 0x70a08231
const BALANCE_OF_SELECTOR = "0x70a08231";
// ERC-20 decimals() selector: 0x313ce567
const DECIMALS_SELECTOR = "0x313ce567";
// ERC-20 symbol() selector: 0x95d89b41
const SYMBOL_SELECTOR = "0x95d89b41";

async function rpcCall(rpcUrl, method, params) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10000);
  const resp = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
    signal: controller.signal,
  });
  clearTimeout(timeout);
  const text = await resp.text();
  let data;
  try { data = JSON.parse(text); } catch { throw new Error(`RPC returned invalid JSON (HTTP ${resp.status})`); }
  if (data.error) throw new Error(data.error.message);
  return data.result;
}

function decodeUint(hex) {
  return BigInt(hex);
}

function decodeString(hex) {
  // ABI-encoded string: offset (32 bytes) + length (32 bytes) + data
  const clean = hex.replace(/^0x/, "");
  if (clean.length < 128) return null;
  const len = parseInt(clean.slice(64, 128), 16);
  const bytes = Buffer.from(clean.slice(128, 128 + len * 2), "hex");
  return bytes.toString("utf8");
}

async function fetchBalance(chainId) {
  // Resolve chain info first so error output still shows the chain name
  let name = `Chain ${chainId}`;
  let symbol = "???";
  let decimals = 18;
  try {
    const info = await getChainInfo(chainId);
    if (info) {
      name = info.name;
      symbol = info.nativeCurrency?.symbol || symbol;
      decimals = info.nativeCurrency?.decimals ?? decimals;
    }
  } catch {}

  try {
    const rpcUrl = await getRpc(chainId);

    if (flags.token) {
      // ERC-20 token balance query
      const paddedAddr = address.toLowerCase().replace(/^0x/, "").padStart(64, "0");
      const balanceData = BALANCE_OF_SELECTOR + paddedAddr;

      // Fetch decimals and symbol first — these are required for correct display.
      // Do NOT default to 18 decimals on failure: that silently produces wrong values.
      let tokenDecimals;
      let tokenSymbol;
      try {
        const decHex = await rpcCall(rpcUrl, "eth_call", [{ to: flags.token, data: DECIMALS_SELECTOR }, "latest"]);
        tokenDecimals = Number(decodeUint(decHex));
      } catch (err) {
        return { chain: name, chainId, balance: null, symbol: flags.token, tokenAddress: flags.token, error: `failed to read token decimals: ${err.message}` };
      }
      try {
        const symHex = await rpcCall(rpcUrl, "eth_call", [{ to: flags.token, data: SYMBOL_SELECTOR }, "latest"]);
        tokenSymbol = decodeString(symHex) || flags.token;
      } catch {
        tokenSymbol = flags.token;
      }

      const balHex = await rpcCall(rpcUrl, "eth_call", [{ to: flags.token, data: balanceData }, "latest"]);

      return { chain: name, chainId, balance: formatBalance(balHex, tokenDecimals), symbol: tokenSymbol, tokenAddress: flags.token };
    }

    // Native balance query
    const result = await rpcCall(rpcUrl, "eth_getBalance", [address, "latest"]);
    return { chain: name, chainId, balance: formatBalance(result, decimals), symbol };
  } catch (err) {
    return { chain: name, chainId, balance: null, symbol, error: err.message };
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
const results = await Promise.allSettled(flags.chains.map(fetchBalance));
const rows = results.map((r) => r.value).filter(Boolean);

if (flags.json) {
  console.log(JSON.stringify(rows, null, 2));
} else if (rows.length === 0) {
  console.log("No results.");
} else {
  const nameW = Math.max(14, ...rows.map((r) => r.chain.length + 2));
  console.log(`${"Chain".padEnd(nameW)}Balance`);
  for (const row of rows) {
    const val = row.error ? `(error: ${row.error})` : `${row.balance} ${row.symbol}`;
    console.log(`${row.chain.padEnd(nameW)}${val}`);
  }
}
