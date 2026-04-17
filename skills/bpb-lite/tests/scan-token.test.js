import test from 'node:test';
import assert from 'node:assert/strict';
import { inferPlatform, inferSignal, inferConfidence } from '../scripts/scan-token.js';

test('inferPlatform returns Printr on direct attribution', () => {
  const result = inferPlatform({
    ca: '29CWsqH84TykHDDwA6DtETUtXQPuKbVgKCmxtkBsbrrr',
    printr: { id: 'abc123' },
    bestPair: null,
  });
  assert.equal(result.platform, 'Printr');
  assert.equal(result.confidence, 'High');
});

test('inferPlatform detects Meteora from pair metadata', () => {
  const result = inferPlatform({
    ca: 'Example1111111111111111111111111111111111111',
    printr: null,
    bestPair: { dexId: 'meteora', labels: ['damm-v2'], url: 'https://dexscreener.com/solana/example' },
  });
  assert.equal(result.platform, 'Meteora');
  assert.equal(result.confidence, 'Medium');
});

test('inferSignal returns EARLY for strong structure', () => {
  const result = inferSignal({ marketCapUsd: 40000, volume1hUsd: 50000, liquidityUsd: 15000, holders: 180 });
  assert.equal(result, 'EARLY');
});

test('inferSignal returns CONFIRMATION for lower relative volume', () => {
  const result = inferSignal({ marketCapUsd: 100000, volume1hUsd: 40000, liquidityUsd: 12000, holders: 300 });
  assert.equal(result, 'CONFIRMATION');
});

test('inferConfidence falls to Low for sparse unknown structure', () => {
  const result = inferConfidence({
    attributionConfidence: 'Low',
    marketCapUsd: 0,
    volume1hUsd: 0,
    liquidityUsd: 0,
    holders: null,
  });
  assert.equal(result, 'Low');
});
