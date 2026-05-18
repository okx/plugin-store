#!/usr/bin/env node
//
// Solana Balance Checker
//
// Queries native SOL balance and optionally all SPL token balances for a
// given Solana base58 address. Uses only Node.js built-ins
// and the global fetch() API — no external dependencies.
//
// Usage:
//   node balance-solana.mjs <base58_address> [--tokens] [--json]
//
// Arguments:
//   base58_address  A standard Solana base58 address (32-44 characters).
//                   Do NOT pass OWS wallet names — resolve the Solana
//                   address first via `ows wallet list` and extract the
//                   address from the `solana:` line.
//
// Options:
//   --tokens  Also list all non-zero SPL token account balances
//   --json    Output results as JSON
//
// Environment:
//   SOLANA_RPC_URL  Override Solana RPC endpoint (otherwise discovered via rpc.mjs)
//
// Examples:
//   # Check SOL balance for a resolved address
//   node balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr
//
//   # Check SOL + SPL balances
//   node balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr --tokens
//
//   # JSON output for scripting
//   node balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr --tokens --json

import { Connection, PublicKey } from "@solana/web3.js";
import { getRpc } from "../../common/scripts/rpc.mjs";

const COMMITMENT = "confirmed";

// ---------------------------------------------------------------------------
// Parse CLI arguments
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const flags = {};
const positional = [];

for (let i = 0; i < args.length; i++) {
  if (args[i] === "--tokens") {
    flags.tokens = true;
  } else if (args[i] === "--json") {
    flags.json = true;
  } else if (args[i].startsWith("--")) {
    console.error(`Unknown flag: ${args[i]}`);
    process.exit(1);
  } else {
    positional.push(args[i]);
  }
}

if (positional.length < 1) {
  console.error("Usage: node balance-solana.mjs <base58_address> [--tokens] [--json]");
  process.exit(1);
}

// Solana chain ID in deBridge is 7565164 — rpc.mjs handles env var + fallback
const rpcUrl = await getRpc(7565164);
const connection = new Connection(rpcUrl, COMMITMENT);

// ---------------------------------------------------------------------------
// Validate Solana address
// ---------------------------------------------------------------------------
// Base58 Solana addresses are 32-44 characters using the Bitcoin base58
// alphabet (no 0, O, I, l). The caller must resolve wallet names to
// standard base58 addresses before invoking this script.

const BASE58_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

const address = positional[0];

if (!BASE58_RE.test(address)) {
  console.error(`Error: "${address}" is not a valid Solana base58 address.`);
  console.error("This script accepts only standard Solana addresses (32-44 base58 characters).");
  console.error("Resolve wallet names first: run `ows wallet list` and extract the solana: address.");
  process.exit(1);
}

const pubkey = new PublicKey(address);

// ---------------------------------------------------------------------------
// Step 1: Query native SOL balance
// ---------------------------------------------------------------------------
const lamports = await connection.getBalance(pubkey);
const solBalance = lamports / 1e9;

// ---------------------------------------------------------------------------
// Step 2: Query SPL token balances (if --tokens)
// ---------------------------------------------------------------------------
const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
let tokenAccounts = [];

if (flags.tokens) {
  const tokenResult = await connection.getParsedTokenAccountsByOwner(pubkey, {
    programId: TOKEN_PROGRAM_ID,
  });

  tokenAccounts = tokenResult.value
    .map((account) => {
      const info = account.account.data.parsed.info;
      const amount = info.tokenAmount;
      return {
        mint: info.mint,
        amount: amount.uiAmountString,
        decimals: amount.decimals,
        uiAmount: amount.uiAmount,
      };
    })
    .filter((t) => t.uiAmount > 0);
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------
if (flags.json) {
  const output = {
    address,
    solBalance: solBalance.toString(),
  };
  if (flags.tokens) {
    output.tokens = tokenAccounts;
  }
  console.log(JSON.stringify(output, null, 2));
} else {
  console.log(`SOL Balance: ${solBalance} SOL`);

  if (flags.tokens) {
    if (tokenAccounts.length === 0) {
      console.log("\nSPL Tokens: none");
    } else {
      console.log("\nSPL Tokens:");
      for (const t of tokenAccounts) {
        console.log(`  ${t.mint}: ${t.amount} (decimals: ${t.decimals})`);
      }
    }
  }
}
