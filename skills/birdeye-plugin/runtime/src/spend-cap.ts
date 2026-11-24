import { existsSync, mkdirSync, readFileSync, writeFileSync, statSync } from 'node:fs';
import { dirname } from 'node:path';
import { homedir } from 'node:os';
import { getCompiledTransactionMessageDecoder } from '@solana/transaction-messages';

const TOKEN_PROGRAM = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
const TOKEN_2022_PROGRAM = 'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb';
const TRANSFER_CHECKED_DISCRIMINATOR = 12;

export const DEFAULT_STATE_FILE = `${homedir()}/.birdeye/spend.json`;
export const DEFAULT_KEY_FILE = `${homedir()}/.birdeye/key`;
export const DEFAULT_MAX_DAILY_USDC_BASE_UNITS = '100000';

export type SpendState = {
  day: string;
  spentBaseUnits: string;
};

function todayUtc(): string {
  return new Date().toISOString().slice(0, 10);
}

export function loadState(file: string): SpendState {
  if (!existsSync(file)) return { day: todayUtc(), spentBaseUnits: '0' };
  try {
    const raw = JSON.parse(readFileSync(file, 'utf-8')) as SpendState;
    if (raw.day !== todayUtc()) return { day: todayUtc(), spentBaseUnits: '0' };
    return raw;
  } catch {
    return { day: todayUtc(), spentBaseUnits: '0' };
  }
}

export function saveState(file: string, state: SpendState): void {
  const dir = dirname(file);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true, mode: 0o700 });
  writeFileSync(file, JSON.stringify(state), { mode: 0o600 });
}

export function assertFileMode0600(file: string): void {
  const st = statSync(file);
  const mode = st.mode & 0o777;
  if (mode !== 0o600) {
    throw new Error(`${file} must be mode 0600 (current: ${mode.toString(8)}). Run: chmod 600 ${file}`);
  }
}

export function parseUsdcAmountFromMessageBytes(messageBytes: Uint8Array): bigint {
  const decoder = getCompiledTransactionMessageDecoder();
  const msg = decoder.decode(messageBytes) as unknown as {
    staticAccounts: readonly string[];
    instructions: readonly { programAddressIndex: number; data?: Uint8Array }[];
  };
  let total = 0n;
  for (const ix of msg.instructions) {
    const programId = msg.staticAccounts[ix.programAddressIndex];
    if (programId !== TOKEN_PROGRAM && programId !== TOKEN_2022_PROGRAM) continue;
    const data = ix.data;
    if (!data || data.length < 9 || data[0] !== TRANSFER_CHECKED_DISCRIMINATOR) continue;
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    total += view.getBigUint64(1, true);
  }
  return total;
}

export function checkAndRecord(
  state: SpendState,
  amountBaseUnits: bigint,
  maxDailyBaseUnits: bigint,
): SpendState {
  const current = BigInt(state.spentBaseUnits);
  const next = current + amountBaseUnits;
  if (next > maxDailyBaseUnits) {
    throw new Error(
      `Daily spend cap exceeded: would spend ${next} base units (cap: ${maxDailyBaseUnits}, already spent: ${current}, this tx: ${amountBaseUnits})`,
    );
  }
  return { day: state.day, spentBaseUnits: next.toString() };
}
