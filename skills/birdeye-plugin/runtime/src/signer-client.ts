import { fork, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import type { Address } from '@solana/kit';

type Pending = {
  resolve: (value: unknown) => void;
  reject: (err: Error) => void;
};

type HostMessage =
  | { type: 'ready' }
  | { type: 'address'; id: number; address: string }
  | { type: 'signed'; id: number; signatures: Array<Record<string, string>> }
  | { type: 'error'; id: number; message: string };

const HOST_FILE = join(dirname(fileURLToPath(import.meta.url)), 'signer-host.js');

function createHost(): { child: ChildProcess; ready: Promise<void>; pending: Map<number, Pending> } {
  const allowedKeys = [
    'BIRDEYE_SIGNER_KEY_FILE',
    'BIRDEYE_SIGNER_STATE_FILE',
    'MAX_DAILY_SPEND_USDC_BASE_UNITS',
    'PATH',
    'HOME',
  ];
  const env: Record<string, string> = {};
  for (const k of allowedKeys) {
    const v = process.env[k];
    if (v) env[k] = v;
  }

  const child = fork(HOST_FILE, [], { env, stdio: ['ignore', 'inherit', 'inherit', 'ipc'] });
  const pending = new Map<number, Pending>();

  let resolveReady: () => void;
  let rejectReady: (err: Error) => void;
  const ready = new Promise<void>((res, rej) => {
    resolveReady = res;
    rejectReady = rej;
  });

  child.on('message', (msg: HostMessage) => {
    if (msg.type === 'ready') {
      resolveReady();
      return;
    }
    const p = pending.get(msg.id);
    if (!p) return;
    pending.delete(msg.id);
    if (msg.type === 'error') p.reject(new Error(msg.message));
    else if (msg.type === 'address') p.resolve(msg.address);
    else if (msg.type === 'signed') p.resolve(msg.signatures);
  });

  child.on('exit', (code) => {
    rejectReady(new Error(`signer-host exited with code ${code}`));
    for (const p of pending.values()) p.reject(new Error('signer-host exited'));
    pending.clear();
  });

  return { child, ready, pending };
}

let counter = 0;

export async function createIpcSigner(): Promise<{
  address: Address;
  signTransactions: (
    transactions: ReadonlyArray<{ messageBytes: Uint8Array; signatures: Record<string, Uint8Array | null> }>,
  ) => Promise<Array<Record<string, Uint8Array>>>;
}> {
  const { child, ready, pending } = createHost();
  await ready;

  function call<T>(req: { type: string } & Record<string, unknown>): Promise<T> {
    const id = ++counter;
    return new Promise<T>((resolve, reject) => {
      pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
      child.send({ ...req, id });
    });
  }

  const address = (await call<string>({ type: 'getAddress' })) as Address;

  return {
    address,
    async signTransactions(transactions) {
      const txs = transactions.map((t) => ({
        messageBytes: Buffer.from(t.messageBytes).toString('base64'),
        signatures: Object.fromEntries(
          Object.entries(t.signatures).map(([k, v]) => [k, v ? Buffer.from(v).toString('base64') : null]),
        ),
      }));
      const out = await call<Array<Record<string, string>>>({ type: 'sign', txs });
      return out.map((dict) =>
        Object.fromEntries(
          Object.entries(dict).map(([addr, b64]) => [addr, new Uint8Array(Buffer.from(b64, 'base64'))]),
        ),
      );
    },
  };
}
