#!/usr/bin/env node
//
// deBridge Solana Bridge — sign and broadcast a create_tx response via OWS
//
// Usage:
//   echo '<create_tx JSON>' | node debridge-solana-bridge.mjs <wallet_name> [--rpc <url>] [--json]
//
// The script reads a deBridge create_tx JSON response from stdin.
// The agent calls MCP to get the quote, then pipes it here for signing.
//
// Example:
//   echo '<create_tx JSON from mcp__debridge__create_tx>' | node debridge-solana-bridge.mjs default
//
// Requires:
//   npm install @open-wallet-standard/core

import { signMessage } from "@open-wallet-standard/core";
import { Connection } from "@solana/web3.js";
import bs58 from "bs58";
import { getRpc } from "../../common/scripts/rpc.mjs";

const COMMITMENT = "confirmed";

// ---------------------------------------------------------------------------
// Parse CLI
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const flags = {};
const positional = [];

for (let i = 0; i < args.length; i++) {
  if (args[i] === "--rpc" && args[i + 1]) flags.rpc = args[++i];
  else if (args[i] === "--json") flags.json = true;
  else if (!args[i].startsWith("--")) positional.push(args[i]);
  else { console.error(`Unknown flag: ${args[i]}`); process.exit(1); }
}

if (positional.length < 1) {
  console.error("Usage: echo '<create_tx JSON>' | node debridge-solana-bridge.mjs <wallet_name> [--rpc <url>] [--json]");
  process.exit(1);
}

const walletName = positional[0];
const rpcUrl = flags.rpc || await getRpc(7565164);
const connection = new Connection(rpcUrl, COMMITMENT);

// ---------------------------------------------------------------------------
// Read create_tx response from stdin
// ---------------------------------------------------------------------------
const chunks = [];
for await (const chunk of process.stdin) {
  chunks.push(chunk);
}
const stdinData = Buffer.concat(chunks).toString("utf8");

// Expect plain create_tx JSON on stdin (agent parses MCP response before piping)
const quote = JSON.parse(stdinData.trim());

if (quote.message || quote.code) {
  console.error("deBridge error:", quote.message || JSON.stringify(quote));
  process.exit(1);
}

const est = quote.estimation.dstChainTokenOut;
if (!flags.json) {
  console.log(`Estimated output: ${(Number(est.amount) / Math.pow(10, est.decimals)).toFixed(4)} ${est.symbol} (~$${est.approximateUsdValue.toFixed(2)})`);
  console.log("Order ID:", quote.orderId);
}

// ---------------------------------------------------------------------------
// Sign and broadcast
// ---------------------------------------------------------------------------
const txHex = quote.tx.data.replace(/^0x/, "");
const tx = Buffer.from(txHex, "hex");

if (tx[0] !== 0x01) {
  console.error(`Expected single-signer tx (byte 0 = 0x01), got 0x${tx[0].toString(16).padStart(2, "0")}`);
  process.exit(1);
}

if (tx.length < 70) {
  console.error(`Transaction too short for expected V0 layout: length=${tx.length}, expected at least 70 bytes`);
  process.exit(1);
}

const numKeys = tx[69];
const blockhashOffset = 70 + numKeys * 32;

if (blockhashOffset + 32 > tx.length) {
  console.error(
    `Transaction too short for expected V0 layout: blockhash would end at offset ${blockhashOffset + 32}, but tx length is ${tx.length}`,
  );
  process.exit(1);
}

// Fetch fresh blockhash
const { blockhash } = await connection.getLatestBlockhash(COMMITMENT);

// Insert blockhash
const blockhashBytes = bs58.decode(blockhash);
if (blockhashBytes.length !== 32) {
  console.error(`Invalid blockhash length from RPC (expected 32 bytes, got ${blockhashBytes.length})`);
  process.exit(1);
}
Buffer.from(blockhashBytes).copy(tx, blockhashOffset);

// Sign message bytes (offset 65+)
if (!flags.json) console.log("Signing with OWS...");
const messageHex = tx.subarray(65).toString("hex");
const signResult = signMessage(walletName, "solana", messageHex, undefined, "hex");
const sigBytes = Buffer.from(signResult.signature, "hex");
if (sigBytes.length !== 64) {
  console.error(`Invalid signature length (expected 64 bytes, got ${sigBytes.length})`);
  process.exit(1);
}
sigBytes.copy(tx, 1);

// Broadcast
if (!flags.json) console.log("Broadcasting...");
const txSignature = await connection.sendRawTransaction(tx, {
  skipPreflight: false,
  preflightCommitment: COMMITMENT,
});

if (flags.json) {
  console.log(JSON.stringify({
    txSignature,
    orderId: quote.orderId,
  }, null, 2));
} else {
  console.log("Transaction sent:", txSignature);
  console.log("Order ID:", quote.orderId);
}
