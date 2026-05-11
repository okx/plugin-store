import { readFileSync } from 'node:fs';
import { createKeyPairSignerFromBytes } from '@solana/kit';
import { base58 } from '@scure/base';
import {
  DEFAULT_KEY_FILE,
  DEFAULT_MAX_DAILY_USDC_BASE_UNITS,
  DEFAULT_STATE_FILE,
  assertFileMode0600,
  checkAndRecord,
  loadState,
  parseUsdcAmountFromMessageBytes,
  saveState,
} from './spend-cap.js';

type SignRequest = {
  type: 'sign';
  id: number;
  txs: Array<{ messageBytes: string; signatures: Record<string, string | null> }>;
};

type AddressRequest = { type: 'getAddress'; id: number };

type Request = SignRequest | AddressRequest;

type Response =
  | { type: 'address'; id: number; address: string }
  | { type: 'signed'; id: number; signatures: Array<Record<string, string>> }
  | { type: 'error'; id: number; message: string };

function send(msg: Response): void {
  if (!process.send) throw new Error('signer-host must run as child process');
  process.send(msg);
}

function readKeyBytes(): Uint8Array {
  const file = process.env.BIRDEYE_SIGNER_KEY_FILE || DEFAULT_KEY_FILE;
  assertFileMode0600(file);
  const raw = readFileSync(file, 'utf-8').trim();
  return base58.decode(raw);
}

function getMaxDailyBaseUnits(): bigint {
  return BigInt(process.env.MAX_DAILY_SPEND_USDC_BASE_UNITS || DEFAULT_MAX_DAILY_USDC_BASE_UNITS);
}

async function main(): Promise<void> {
  const keyBytes = readKeyBytes();
  const signer = await createKeyPairSignerFromBytes(keyBytes);
  const stateFile = process.env.BIRDEYE_SIGNER_STATE_FILE || DEFAULT_STATE_FILE;
  const maxDaily = getMaxDailyBaseUnits();

  process.on('message', async (raw: Request) => {
    try {
      if (raw.type === 'getAddress') {
        send({ type: 'address', id: raw.id, address: signer.address as string });
        return;
      }

      if (raw.type === 'sign') {
        const txs = raw.txs.map((t) => ({
          messageBytes: Buffer.from(t.messageBytes, 'base64') as unknown as Uint8Array,
          signatures: Object.fromEntries(
            Object.entries(t.signatures).map(([k, v]) => [k, v ? (Buffer.from(v, 'base64') as unknown as Uint8Array) : null]),
          ),
        }));

        let nextState = loadState(stateFile);
        for (const tx of txs) {
          const amount = parseUsdcAmountFromMessageBytes(tx.messageBytes);
          nextState = checkAndRecord(nextState, amount, maxDaily);
        }
        const signed = await signer.signTransactions(txs as never);
        saveState(stateFile, nextState);
        const out = signed.map((dict) =>
          Object.fromEntries(
            Object.entries(dict).map(([addr, sig]) => [addr, Buffer.from(sig as Uint8Array).toString('base64')]),
          ),
        );
        send({ type: 'signed', id: raw.id, signatures: out });
      }
    } catch (e) {
      send({ type: 'error', id: raw.id, message: (e as Error).message });
    }
  });

  if (process.send) process.send({ type: 'ready' });
}

main().catch((e) => {
  if (process.send) process.send({ type: 'error', id: -1, message: (e as Error).message });
  process.exit(1);
});
