#!/usr/bin/env node
//
// Auto-discover private keys from environment variables and .env files.
// Identifies key format (EVM vs Solana), derives addresses, and re-exports
// normalized results. Zero external dependencies — uses only Node.js built-ins.
//
// Usage:
//   node env-keys.mjs                   # human-readable output
//   node env-keys.mjs --json            # JSON output for piping
//   node env-keys.mjs --export          # print export statements
//
// Also used as a library:
//   import { discoverKeys } from "./env-keys.mjs";
//   const keys = await discoverKeys();
//
// Scan order (by env var name):
//   1. Exact: PRIVATE_KEY
//   2. Suffix: *_PRIVATE_KEY  (e.g., ETH_PRIVATE_KEY, SOLANA_PRIVATE_KEY)
//   3. Suffix: *_KEY          (e.g., WALLET_KEY, SECRET_KEY — filtered by value format)
//
// Sources scanned:
//   - process.env (current environment)
//   - .env in current directory
//   - .env in home directory (~/.env)
//   - .env.local in current directory
//
// Key classification:
//   - EVM:    0x-prefixed 64 hex chars, or bare 64 hex chars (32 bytes)
//   - Solana: JSON array of 64 numbers (keypair), or base58-encoded ~88 chars (64 bytes)
//   - Unknown: value that doesn't match EVM or Solana patterns
//
// Security:
//   - NEVER prints, logs, or exports raw private key values
//   - Only outputs: source, variable name, chain type, derived address
//   - Issues warnings about insecure storage (plaintext env vars, .env on disk)

import { readFileSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";
import { createPrivateKey, createPublicKey } from "node:crypto";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Base58 codec (Bitcoin alphabet — no external deps)
// ---------------------------------------------------------------------------
const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BASE58_MAP = new Uint8Array(256).fill(255);
for (let i = 0; i < 58; i++) BASE58_MAP[BASE58_ALPHABET.charCodeAt(i)] = i;

function base58Decode(str) {
  const bytes = [];
  for (const c of str) {
    let carry = BASE58_MAP[c.charCodeAt(0)];
    if (carry === 255) throw new Error(`Invalid base58 character: ${c}`);
    for (let j = 0; j < bytes.length; j++) {
      carry += bytes[j] * 58;
      bytes[j] = carry & 0xff;
      carry >>= 8;
    }
    while (carry > 0) {
      bytes.push(carry & 0xff);
      carry >>= 8;
    }
  }
  // Leading '1's → leading zero bytes
  for (const c of str) {
    if (c !== "1") break;
    bytes.push(0);
  }
  return Buffer.from(bytes.reverse());
}

function base58Encode(buf) {
  const digits = [0];
  for (const byte of buf) {
    let carry = byte;
    for (let j = 0; j < digits.length; j++) {
      carry += digits[j] << 8;
      digits[j] = carry % 58;
      carry = (carry / 58) | 0;
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = (carry / 58) | 0;
    }
  }
  let str = "";
  // Leading zero bytes → leading '1's
  for (const byte of buf) {
    if (byte !== 0) break;
    str += "1";
  }
  for (let i = digits.length - 1; i >= 0; i--) {
    str += BASE58_ALPHABET[digits[i]];
  }
  return str;
}

// ---------------------------------------------------------------------------
// Key format detection
// ---------------------------------------------------------------------------
const HEX_RE = /^(0x)?[0-9a-fA-F]{64}$/;
const BASE58_RE = /^[1-9A-HJ-NP-Za-km-z]{60,100}$/;

/** @returns {"evm"|"solana-keypair"|"solana-secret"|null} */
function classifyKey(value) {
  const trimmed = value.trim();

  // JSON array of numbers — Solana keypair (64 bytes) or secret (32 bytes)
  if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
    try {
      const arr = JSON.parse(trimmed);
      if (Array.isArray(arr) && arr.every((n) => Number.isInteger(n) && n >= 0 && n <= 255)) {
        if (arr.length === 64) return "solana-keypair";
        if (arr.length === 32) return "solana-secret";
      }
    } catch {}
    return null;
  }

  // 0x-prefixed hex → EVM
  if (/^0x[0-9a-fA-F]{64}$/.test(trimmed)) return "evm";

  // Bare 64 hex chars — could be EVM or Solana (ambiguous, prefer EVM since
  // Solana keys are almost never stored as bare hex)
  if (/^[0-9a-fA-F]{64}$/.test(trimmed)) return "evm";

  // 128 hex chars (64 bytes) — Solana keypair in hex
  if (/^[0-9a-fA-F]{128}$/.test(trimmed)) return "solana-keypair";

  // Base58 ~87-88 chars → likely Solana keypair (64 bytes base58-encoded)
  if (BASE58_RE.test(trimmed)) {
    try {
      const decoded = base58Decode(trimmed);
      if (decoded.length === 64) return "solana-keypair";
      if (decoded.length === 32) return "solana-secret";
    } catch {}
  }

  return null;
}

// ---------------------------------------------------------------------------
// Address derivation
// ---------------------------------------------------------------------------

/**
 * Derive EVM address from a 32-byte private key using Node.js crypto.
 * Uses secp256k1 ECDH to get the uncompressed public key, then hashes with
 * keccak256 and takes the last 20 bytes.
 */
async function deriveEvmAddress(hexKey) {
  const bare = hexKey.replace(/^0x/, "");
  const keyBuf = Buffer.from(bare, "hex");

  // Try ethers first (more reliable), fall back to manual ECDH
  try {
    const { computeAddress } = await import("ethers");
    return computeAddress("0x" + bare);
  } catch {}

  // Try viem
  try {
    const { privateKeyToAccount } = await import("viem/accounts");
    return privateKeyToAccount(("0x" + bare)).address;
  } catch {}

  // Manual: use Node.js ECDH for secp256k1 public key + keccak256
  // Node.js crypto.createECDH supports secp256k1
  const { createECDH, createHash } = await import("node:crypto");
  const ecdh = createECDH("secp256k1");
  ecdh.setPrivateKey(keyBuf);
  const uncompressedPub = ecdh.getPublicKey(); // 65 bytes: 04 + x + y
  const pubNoPrefix = uncompressedPub.subarray(1); // remove 04 prefix

  // keccak256 — try Node.js (OpenSSL may expose it as 'sha3-256' in some
  // builds, but Ethereum uses pre-NIST keccak). If keccak is unavailable,
  // try the 'keccak256' name that some OpenSSL builds provide.
  let hash;
  for (const alg of ["keccak256", "sha3-256"]) {
    try {
      hash = createHash(alg).update(pubNoPrefix).digest();
      break;
    } catch {}
  }

  if (!hash) {
    // Last resort: if neither ethers/viem nor keccak is available,
    // we cannot derive the address. Return null.
    return null;
  }

  // NOTE: sha3-256 (FIPS 202) differs from keccak256 (pre-NIST).
  // If only sha3-256 was available, the address may be wrong.
  // We prefer ethers/viem above for correctness.
  return "0x" + hash.subarray(12).toString("hex");
}

/**
 * Derive Solana address (base58 public key) from a keypair or secret key.
 * For 64-byte keypairs, the public key is bytes 32-63.
 * For 32-byte secrets, derive via ed25519.
 */
function deriveSolanaAddress(keyBytes) {
  if (keyBytes.length === 64) {
    // Full keypair: public key is the last 32 bytes
    return base58Encode(keyBytes.subarray(32));
  }

  if (keyBytes.length === 32) {
    // 32-byte secret: derive ed25519 public key via Node.js crypto
    try {
      // Wrap raw 32-byte seed in PKCS8 DER for ed25519
      const pkcs8Prefix = Buffer.from("302e020100300506032b657004220420", "hex");
      const der = Buffer.concat([pkcs8Prefix, keyBytes]);
      const privKey = createPrivateKey({ key: der, format: "der", type: "pkcs8" });
      const pubKey = createPublicKey(privKey);
      const rawPub = pubKey.export({ type: "spki", format: "der" });
      // ed25519 SPKI DER: 12-byte prefix + 32-byte raw public key
      const pubBytes = rawPub.subarray(rawPub.length - 32);
      return base58Encode(pubBytes);
    } catch {
      return null;
    }
  }

  return null;
}

// ---------------------------------------------------------------------------
// Extract raw key bytes for Solana classification
// ---------------------------------------------------------------------------
function extractSolanaBytes(value, type) {
  const trimmed = value.trim();

  if (type === "solana-keypair" || type === "solana-secret") {
    // JSON array
    if (trimmed.startsWith("[")) {
      return Buffer.from(JSON.parse(trimmed));
    }
    // Hex
    if (/^[0-9a-fA-F]+$/.test(trimmed)) {
      return Buffer.from(trimmed, "hex");
    }
    // Base58
    return base58Decode(trimmed);
  }

  return null;
}

// ---------------------------------------------------------------------------
// .env file parser (minimal, handles KEY=VALUE and KEY="VALUE")
// ---------------------------------------------------------------------------
function parseEnvFile(filePath) {
  const entries = [];
  if (!existsSync(filePath)) return entries;

  let content;
  try {
    content = readFileSync(filePath, "utf-8");
  } catch {
    return entries;
  }

  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    const eqIdx = trimmed.indexOf("=");
    if (eqIdx < 1) continue;

    const key = trimmed.slice(0, eqIdx).trim();
    let val = trimmed.slice(eqIdx + 1).trim();

    // Strip surrounding quotes
    if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith("'") && val.endsWith("'"))) {
      val = val.slice(1, -1);
    }

    if (val) entries.push({ key, value: val });
  }

  return entries;
}

// ---------------------------------------------------------------------------
// Name-pattern matching
// ---------------------------------------------------------------------------
const SKIP_PREFIXES = [
  "API_", "AWS_", "SSH_", "GPG_", "SSL_", "TLS_", "HMAC_",
  "ENCRYPTION_", "SIGNING_", "SESSION_", "COOKIE_", "JWT_",
  "NEXTAUTH_", "NEXT_PUBLIC_", "REACT_APP_", "VITE_",
];

function isKeyCandidate(name) {
  const upper = name.toUpperCase();

  // Tier 1: exact match
  if (upper === "PRIVATE_KEY") return true;

  // Tier 2: ends with _PRIVATE_KEY
  if (upper.endsWith("_PRIVATE_KEY")) return true;

  // Tier 3: ends with _KEY — but skip non-crypto names
  if (upper.endsWith("_KEY")) {
    // Skip known non-crypto key patterns
    if (SKIP_PREFIXES.some((p) => upper.startsWith(p))) return false;
    // Skip if name contains API, TOKEN, ACCESS, SECRET (for API keys)
    if (/API|TOKEN|ACCESS|LICENSE|REGISTRY/.test(upper)) return false;
    return true;
  }

  // Also match *_SECRET_KEY
  if (upper.endsWith("_SECRET_KEY")) return true;

  return false;
}

// ---------------------------------------------------------------------------
// Main discovery function (exported for library use)
// ---------------------------------------------------------------------------

/**
 * @typedef {Object} DiscoveredKey
 * @property {string} name - Environment variable name
 * @property {string} source - Where found: "env", ".env", "~/.env", ".env.local"
 * @property {"evm"|"solana"|"unknown"} chain - Detected chain type
 * @property {string|null} address - Derived address (null if derivation failed)
 * @property {string} format - Raw format: "hex", "hex-0x", "base58", "json-array"
 */

/**
 * Discover private keys from environment variables and .env files.
 * Never exposes key values — only metadata and derived addresses.
 * @returns {Promise<DiscoveredKey[]>}
 */
export async function discoverKeys() {
  const seen = new Map(); // name → { value, source } — deduplicate across sources
  const results = [];

  // Collect candidates from env vars (highest priority)
  for (const [key, value] of Object.entries(process.env)) {
    if (isKeyCandidate(key) && value) {
      seen.set(key, { value, source: "env" });
    }
  }

  // Collect from .env files (lower priority — env vars take precedence)
  const envFiles = [
    { path: resolve(".env"), source: ".env" },
    { path: resolve(".env.local"), source: ".env.local" },
    { path: resolve(homedir(), ".env"), source: "~/.env" },
  ];

  for (const { path, source } of envFiles) {
    for (const { key, value } of parseEnvFile(path)) {
      if (isKeyCandidate(key) && !seen.has(key)) {
        seen.set(key, { value, source });
      }
    }
  }

  // Classify and derive addresses
  for (const [name, { value, source }] of seen) {
    const type = classifyKey(value);
    if (!type) continue; // not a recognizable private key format

    const trimmed = value.trim();
    let chain, address, format;

    if (type === "evm") {
      chain = "evm";
      format = trimmed.startsWith("0x") ? "hex-0x" : "hex";
      address = await deriveEvmAddress(trimmed);
    } else if (type === "solana-keypair" || type === "solana-secret") {
      chain = "solana";
      const keyBytes = extractSolanaBytes(trimmed, type);

      if (trimmed.startsWith("[")) format = "json-array";
      else if (/^[0-9a-fA-F]+$/.test(trimmed)) format = "hex";
      else format = "base58";

      address = keyBytes ? deriveSolanaAddress(keyBytes) : null;
    } else {
      chain = "unknown";
      format = "unknown";
      address = null;
    }

    results.push({ name, source, chain, address, format });
  }

  // Sort: env-privkey first, then by name
  results.sort((a, b) => {
    const srcOrder = { env: 0, ".env": 1, ".env.local": 2, "~/.env": 3 };
    const diff = (srcOrder[a.source] ?? 9) - (srcOrder[b.source] ?? 9);
    return diff !== 0 ? diff : a.name.localeCompare(b.name);
  });

  return results;
}

// ---------------------------------------------------------------------------
// CLI entrypoint
// ---------------------------------------------------------------------------
const __filename = fileURLToPath(import.meta.url);
if (process.argv[1] === __filename) {
  const args = process.argv.slice(2);
  const asJson = args.includes("--json");
  const asExport = args.includes("--export");

  const keys = await discoverKeys();

  if (keys.length === 0) {
    if (asJson) {
      console.log("[]");
    } else {
      console.log("No private keys found in environment or .env files.");
      console.log("\nSearched:");
      console.log("  - Environment variables matching *PRIVATE_KEY, *_KEY");
      console.log("  - .env, .env.local, ~/.env");
    }
    process.exit(0);
  }

  if (asJson) {
    console.log(JSON.stringify(keys, null, 2));
  } else if (asExport) {
    // Print export statements for downstream scripts
    // Groups by chain, exports canonical variable names
    const evmKeys = keys.filter((k) => k.chain === "evm");
    const solKeys = keys.filter((k) => k.chain === "solana");

    if (evmKeys.length > 0) {
      const primary = evmKeys[0];
      console.log(`# EVM (from ${primary.source}:${primary.name})`);
      console.log(`export EVM_ADDRESS="${primary.address || ""}"`);
      console.log(`export EVM_KEY_SOURCE="${primary.source}"`);
      console.log(`export EVM_KEY_VAR="${primary.name}"`);
    }
    if (solKeys.length > 0) {
      const primary = solKeys[0];
      console.log(`# Solana (from ${primary.source}:${primary.name})`);
      console.log(`export SOLANA_ADDRESS="${primary.address || ""}"`);
      console.log(`export SOLANA_KEY_SOURCE="${primary.source}"`);
      console.log(`export SOLANA_KEY_VAR="${primary.name}"`);
    }
  } else {
    // Human-readable table
    const hasInsecure = keys.some((k) => k.source !== "env");

    console.log("Discovered Private Keys\n");

    const nameW = Math.max(12, ...keys.map((k) => k.name.length + 2));
    const srcW = Math.max(8, ...keys.map((k) => k.source.length + 2));
    const chainW = 10;

    console.log(
      `${"Variable".padEnd(nameW)}${"Source".padEnd(srcW)}${"Chain".padEnd(chainW)}Address`
    );
    console.log("-".repeat(nameW + srcW + chainW + 44));

    for (const k of keys) {
      const addr = k.address || "(derivation failed)";
      console.log(
        `${k.name.padEnd(nameW)}${k.source.padEnd(srcW)}${k.chain.padEnd(chainW)}${addr}`
      );
    }

    if (hasInsecure) {
      const fileKeys = keys.filter((k) => k.source !== "env");
      console.log(
        `\n⚠️  WARNING: ${fileKeys.length} key(s) found in plaintext files on disk.`
      );
      console.log("   Files:", [...new Set(fileKeys.map((k) => k.source))].join(", "));
      console.log("   Recommend migrating to OWS (encrypted, local self-custody):");
      console.log("   curl -fsSL https://docs.openwallet.sh/install.sh | bash && ows wallet create");
    }

    // Always warn about env var keys
    const envKeys = keys.filter((k) => k.source === "env");
    if (envKeys.length > 0) {
      console.log(
        `\n⚠️  ${envKeys.length} key(s) in plaintext environment variables.`
      );
      console.log("   Consider OWS for encrypted key storage.");
    }
  }
}
